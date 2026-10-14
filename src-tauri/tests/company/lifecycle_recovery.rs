use super::{support::MockServer, wait_operation};
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
    path::{Path, PathBuf},
    sync::Arc,
};

#[test]
fn company_flow06_recovery_never_deletes_a_new_owner_even_with_identical_snapshot_bytes() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start("normal");
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let install = app.prepare_install("skl_003").unwrap();
    app.confirm(&install.id, false).unwrap();
    assert_eq!(
        wait_operation(&app, &install.id).state,
        OperationState::Succeeded
    );
    let old = app.installations().unwrap().remove(0);
    let remove = app.prepare_remove(&old.id).unwrap();
    app.apply_lifecycle(&remove.id).unwrap();
    assert_eq!(
        wait_operation(&app, &remove.id).state,
        OperationState::Succeeded
    );
    let backup = app.backups(&old.id).unwrap().remove(0);
    let view = app.prepare_restore(&backup.id).unwrap();
    let removed = app.refresh_installation(&old.id).unwrap();
    let repo = CompanyRepository::open(&config.data_root.join("manager.db")).unwrap();
    let mut new = backup.installation.clone();
    new.state = InstallationState::Active;
    let journal = LifecycleJournal {
        view,
        old: removed,
        new,
        old_full_hash: None,
        new_full_hash: Some(backup.full_hash.clone()),
        current_backup: None,
        source_backup: Some(backup),
        phase: ReplacePhase::NewMoved,
        stage_owned: true,
    };
    // An external/previous app run has already placed a new managed owner at
    // the same target; equal hashes alone cannot establish journal ownership.
    let fresh = app.prepare_install("skl_003").unwrap();
    app.confirm(&fresh.id, false).unwrap();
    assert_eq!(
        wait_operation(&app, &fresh.id).state,
        OperationState::Succeeded
    );
    let active = app
        .installations()
        .unwrap()
        .into_iter()
        .find(|i| i.state == InstallationState::Active)
        .unwrap();
    let before = fs::read(Path::new(&active.target).join("SKILL.md")).unwrap();
    repo.record_lifecycle(&journal).unwrap();
    drop(repo);
    drop(app);
    let restarted = CompanyService::new(config, credentials).unwrap();
    assert_eq!(
        fs::read(Path::new(&active.target).join("SKILL.md")).unwrap(),
        before
    );
    assert!(!restarted.recovery_warnings().is_empty());
}

// Seed actual file/SQLite states at each interrupted operation boundary. Observe
#[cfg(windows)]
#[test]
fn company_flow06_recovery_retains_rollback_when_a_writer_can_still_edit_after_verification() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start("normal");
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let install = app.prepare_install("skl_003").unwrap();
    app.confirm(&install.id, false).unwrap();
    assert_eq!(
        wait_operation(&app, &install.id).state,
        OperationState::Succeeded
    );
    let old = app.installations().unwrap().remove(0);
    let remove = app.prepare_remove(&old.id).unwrap();
    app.apply_lifecycle(&remove.id).unwrap();
    assert_eq!(
        wait_operation(&app, &remove.id).state,
        OperationState::Succeeded
    );
    let backup = app.backups(&old.id).unwrap().remove(0);
    let repo = CompanyRepository::open(&config.data_root.join("manager.db")).unwrap();
    let new = repo.installations().unwrap().remove(0);
    let rollback = Path::new(&old.target)
        .parent()
        .unwrap()
        .join(format!(".company-rollback-{}", remove.id));
    copy_tree(Path::new(&backup.path), &rollback).unwrap();
    let journal = LifecycleJournal {
        view: remove,
        old: backup.installation.clone(),
        new,
        old_full_hash: Some(backup.full_hash.clone()),
        new_full_hash: None,
        current_backup: Some(backup),
        source_backup: None,
        phase: ReplacePhase::Committed,
        stage_owned: false,
    };
    repo.record_lifecycle(&journal).unwrap();
    drop(repo);
    drop(app);
    // Read/write/delete sharing lets the old implementation hash and unlink
    // this file even though the writer can still change its bytes at any time.
    let writer = fs::OpenOptions::new()
        .write(true)
        .share_mode(7)
        .open(rollback.join("SKILL.md"))
        .unwrap();
    let restarted = CompanyService::new(config, credentials).unwrap();
    assert!(rollback.join("SKILL.md").exists());
    assert!(!restarted.recovery_warnings().is_empty());
    assert_eq!(
        restarted
            .delete_backup(&journal.current_backup.as_ref().unwrap().id)
            .unwrap_err()
            .code,
        "BACKUP_IN_USE"
    );
    drop(writer);
}

// Seed actual file/SQLite states at each interrupted operation boundary. Observe
// recovery only through the public service after restart.
#[test]
fn company_flow06_recovery_restores_or_keeps_committed_state_at_all_rename_boundaries() {
    for kind in ["remove", "restore_present", "restore_absent"] {
        for checkpoint in 0..6 {
            let root = tempfile::tempdir().unwrap();
            let server = MockServer::start("normal");
            let config = CompanyConfig::for_test(
                &server.url,
                root.path().join("profile"),
                root.path().join("data"),
            )
            .unwrap();
            let credentials = Arc::new(MemoryCredentials::default());
            let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
            app.login("employee01", "demo-password").unwrap();
            let install = app.prepare_install("skl_003").unwrap();
            app.confirm(&install.id, false).unwrap();
            assert_eq!(
                wait_operation(&app, &install.id).state,
                OperationState::Succeeded
            );
            let item = app.installations().unwrap().remove(0);
            let target = PathBuf::from(&item.target);
            fs::write(target.join("SKILL.md"), "Snapshot customized content").unwrap();
            let remove = app.prepare_remove(&item.id).unwrap();
            app.apply_lifecycle(&remove.id).unwrap();
            assert_eq!(
                wait_operation(&app, &remove.id).state,
                OperationState::Succeeded
            );
            let source = app.backups(&item.id).unwrap().remove(0);
            if kind != "restore_absent" {
                let restore = app.prepare_restore(&source.id).unwrap();
                app.apply_lifecycle(&restore.id).unwrap();
                assert_eq!(
                    wait_operation(&app, &restore.id).state,
                    OperationState::Succeeded
                );
                fs::write(
                    target.join("SKILL.md"),
                    "Current copy before interrupted operation",
                )
                .unwrap();
            }
            let view = if kind == "remove" {
                app.prepare_remove(&item.id).unwrap()
            } else {
                app.prepare_restore(&source.id).unwrap()
            };
            let repo = CompanyRepository::open(&config.data_root.join("manager.db")).unwrap();
            let old = repo.installations().unwrap().remove(0);
            let old_hash = if target.exists() {
                Some(content_hash(&target, false).unwrap())
            } else {
                None
            };
            let current_backup = old_hash.as_ref().map(|hash| {
                let id = uuid::Uuid::new_v4().to_string();
                let path = config
                    .data_root
                    .join("backups")
                    .join(&old.id)
                    .join(&id)
                    .join("content");
                copy_tree(&target, &path).unwrap();
                let backup = Backup {
                    id,
                    installation_id: old.id.clone(),
                    created_at: "2026-10-14T00:00:00Z".into(),
                    reason: kind.into(),
                    path: path.display().to_string(),
                    full_hash: hash.clone(),
                    installation: old.clone(),
                };
                fs::write(
                    path.parent().unwrap().join("metadata.json"),
                    serde_json::to_vec(&backup).unwrap(),
                )
                .unwrap();
                repo.save_backup(&backup).unwrap();
                backup
            });
            let mut new = if kind == "remove" {
                old.clone()
            } else {
                source.installation.clone()
            };
            new.state = if kind == "remove" {
                InstallationState::Removed
            } else {
                InstallationState::Active
            };
            let stage = app_lib::core::company::aipro_paths::stage_path(&target, &view.id).unwrap();
            let rollback = target
                .parent()
                .unwrap()
                .join(format!(".company-rollback-{}", view.id));
            let restoring = kind != "remove";
            if restoring {
                copy_tree(Path::new(&source.path), &stage).unwrap();
            }
            let mut journal = LifecycleJournal {
                view,
                old,
                new,
                old_full_hash: old_hash,
                new_full_hash: restoring.then(|| source.full_hash.clone()),
                current_backup,
                source_backup: restoring.then_some(source),
                phase: ReplacePhase::Prepared,
                stage_owned: restoring,
            };
            repo.record_lifecycle(&journal).unwrap();
            if checkpoint >= 1 && journal.old_full_hash.is_some() {
                fs::rename(&target, &rollback).unwrap();
            }
            if checkpoint >= 2 {
                journal.phase = ReplacePhase::OldMoved;
                repo.record_lifecycle(&journal).unwrap();
            }
            if checkpoint >= 3 && restoring {
                fs::rename(&stage, &target).unwrap();
            }
            if checkpoint >= 4 {
                journal.phase = ReplacePhase::NewMoved;
                repo.record_lifecycle(&journal).unwrap();
            }
            if checkpoint >= 5 {
                journal.phase = ReplacePhase::Committed;
                repo.commit_lifecycle(&journal).unwrap();
            }
            drop(repo);
            drop(app);
            let restarted = CompanyService::new(config, credentials).unwrap();
            assert!(
                restarted.recovery_warnings().is_empty(),
                "{kind} checkpoint {checkpoint}: {:?}",
                restarted.recovery_warnings()
            );
            let observed = restarted.refresh_installation(&item.id).unwrap();
            let expected = if checkpoint == 5 {
                journal.new.state
            } else {
                journal.old.state
            };
            assert_eq!(observed.state, expected, "{kind} checkpoint {checkpoint}");
            if expected == InstallationState::Active {
                let expected_text = if checkpoint == 5 {
                    "Snapshot customized content"
                } else {
                    "Current copy before interrupted operation"
                };
                assert_eq!(
                    fs::read_to_string(target.join("SKILL.md")).unwrap(),
                    expected_text,
                    "{kind} checkpoint {checkpoint}"
                );
            } else {
                assert!(!target.exists());
            }
            assert!(!stage.exists());
            assert!(!rollback.exists());
        }
    }
}
