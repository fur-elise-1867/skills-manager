use super::support::MockServer;
use app_lib::core::company::{
    config::CompanyConfig,
    installation_repository::CompanyRepository,
    installation_service::CompanyService,
    package_store::{content_hash, copy_tree},
    session::MemoryCredentials,
    types::*,
};
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

fn wait(app: &CompanyService, id: &str, preview: bool) -> OperationView {
    let end = Instant::now() + Duration::from_secs(30);
    loop {
        let view = app.status(id).unwrap();
        if (preview && view.preview_ready) || view.state == OperationState::Succeeded {
            return view;
        }
        assert!(
            !matches!(
                view.state,
                OperationState::Failed | OperationState::RecoveryRequired
            ),
            "{:?}",
            view.error
        );
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(20));
    }
}
struct Fixture {
    _root: tempfile::TempDir,
    _server: MockServer,
    config: CompanyConfig,
    credentials: Arc<MemoryCredentials>,
    journal: ReplaceJournal,
    old_bytes: Vec<u8>,
    target: PathBuf,
    stage: PathBuf,
    rollback: PathBuf,
}
fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let revision = root.path().join("revision");
    fs::write(&revision, "1").unwrap();
    let server = MockServer::start_with_revision("normal", Some(&revision));
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let install = app.prepare_install("skl_001").unwrap();
    app.confirm(&install.id, true).unwrap();
    wait(&app, &install.id, false);
    let old = app.installations().unwrap().remove(0);
    fs::write(&revision, "2").unwrap();
    let update = app.prepare_update(&old.id).unwrap();
    app.confirm(&update.id, true).unwrap();
    let view = wait(&app, &update.id, true);
    let incoming = config
        .data_root
        .join("packages")
        .join(&view.skill.as_ref().unwrap().package.sha256)
        .join("original");
    let target = PathBuf::from(&old.target);
    let old_bytes = fs::read(target.join("SKILL.md")).unwrap();
    let old_hash = content_hash(&target, false).unwrap();
    let backup_id = uuid::Uuid::new_v4().to_string();
    let backup_path = config
        .data_root
        .join("backups")
        .join(&old.id)
        .join(&backup_id)
        .join("content");
    copy_tree(&target, &backup_path).unwrap();
    let backup = Backup {
        id: backup_id.clone(),
        installation_id: old.id.clone(),
        created_at: "2026-10-13T00:00:00Z".into(),
        reason: "update".into(),
        path: backup_path.display().to_string(),
        full_hash: old_hash.clone(),
        installation: old.clone(),
    };
    let mut new = old.clone();
    new.package_hash = view.skill.as_ref().unwrap().package.sha256.clone();
    new.content_hash = content_hash(&incoming, true).unwrap();
    let stage = app_lib::core::company::aipro_paths::stage_path(&target, &view.id).unwrap();
    let rollback = target
        .parent()
        .unwrap()
        .join(format!(".company-rollback-{}", view.id));
    assert!(stage.starts_with(root.path()) && rollback.starts_with(root.path()));
    copy_tree(&incoming, &stage).unwrap();
    let journal = ReplaceJournal {
        view,
        old,
        new,
        old_full_hash: old_hash,
        new_full_hash: content_hash(&incoming, false).unwrap(),
        backup_id,
        phase: ReplacePhase::Prepared,
        stage_owned: true,
    };
    let repo = CompanyRepository::open(&config.data_root.join("manager.db")).unwrap();
    repo.save_backup(&backup).unwrap();
    repo.record_replacement(&journal).unwrap();
    Fixture {
        _root: root,
        _server: server,
        config,
        credentials,
        journal,
        old_bytes,
        target,
        stage,
        rollback,
    }
}

#[test]
fn company_flow05_recovery_handles_crash_between_rename_and_journal_and_preserves_committed_updates(
) {
    // 0: prepared; 1: old renamed but phase not saved; 2: old_moved;
    // 3: new renamed but phase not saved; 4: new_moved; 5: DB committed, cleanup pending.
    for checkpoint in 0..6 {
        let mut f = fixture();
        let repo = CompanyRepository::open(&f.config.data_root.join("manager.db")).unwrap();
        if checkpoint >= 1 {
            fs::rename(&f.target, &f.rollback).unwrap();
        }
        if checkpoint >= 2 {
            f.journal.phase = ReplacePhase::OldMoved;
            repo.record_replacement(&f.journal).unwrap();
        }
        if checkpoint >= 3 {
            fs::rename(&f.stage, &f.target).unwrap();
        }
        if checkpoint >= 4 {
            f.journal.phase = ReplacePhase::NewMoved;
            repo.record_replacement(&f.journal).unwrap();
        }
        if checkpoint >= 5 {
            f.journal.phase = ReplacePhase::Committed;
            repo.commit_replacement(&f.journal).unwrap();
        }
        drop(repo);
        let app = CompanyService::new(f.config.clone(), f.credentials.clone()).unwrap();
        assert!(
            app.recovery_warnings().is_empty(),
            "checkpoint {checkpoint}: {:?}",
            app.recovery_warnings()
        );
        let restored = app.refresh_installation(&f.journal.old.id).unwrap();
        if checkpoint == 5 {
            assert_eq!(restored.package_hash, f.journal.new.package_hash);
            assert!(fs::read_to_string(f.target.join("SKILL.md"))
                .unwrap()
                .contains("Jira project summary v2"));
        } else {
            assert_eq!(restored.package_hash, f.journal.old.package_hash);
            assert_eq!(fs::read(f.target.join("SKILL.md")).unwrap(), f.old_bytes);
        }
        assert!(!f.stage.exists());
        assert!(!f.rollback.exists());
        assert_eq!(app.backups(&restored.id).unwrap().len(), 1);
    }
}

#[test]
fn company_flow05_uncertain_rollback_is_retained_and_blocks_only_its_target() {
    let f = fixture();
    fs::rename(&f.target, &f.rollback).unwrap();
    fs::rename(&f.stage, &f.target).unwrap();
    fs::write(
        f.rollback.join("SKILL.md"),
        "Edits in rollback must survive",
    )
    .unwrap();
    let app = CompanyService::new(f.config, f.credentials).unwrap();
    assert!(!app.recovery_warnings().is_empty());
    assert_eq!(
        fs::read_to_string(f.rollback.join("SKILL.md")).unwrap(),
        "Edits in rollback must survive"
    );
    assert!(fs::read_to_string(f.target.join("SKILL.md"))
        .unwrap()
        .contains("Jira project summary v2"));
    assert_eq!(
        app.prepare_update(&f.journal.old.id).unwrap_err().code,
        "RECOVERY_REQUIRED"
    );
    assert!(app.prepare_install("skl_002").is_ok());
}

#[cfg(windows)]
#[test]
fn company_flow05_windows_locked_content_is_never_overwritten() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = fixture();
    let app = Arc::new(CompanyService::new(f.config.clone(), f.credentials.clone()).unwrap());
    // The fixture has a pending journal, which recovery cancels safely before this attempt.
    let op = app.prepare_update(&f.journal.old.id).unwrap();
    app.confirm(&op.id, true).unwrap();
    wait(&app, &op.id, true);
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(f.target.join("SKILL.md"))
        .unwrap();
    app.apply_update(&op.id, false).unwrap();
    let end = Instant::now() + Duration::from_secs(30);
    loop {
        let view = app.status(&op.id).unwrap();
        if view.state == OperationState::Failed {
            break;
        }
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(locked);
    assert_eq!(fs::read(f.target.join("SKILL.md")).unwrap(), f.old_bytes);
    assert_eq!(
        app.refresh_installation(&f.journal.old.id)
            .unwrap()
            .package_hash,
        f.journal.old.package_hash
    );
}
