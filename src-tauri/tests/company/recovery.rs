use app_lib::core::company::{
    aipro_paths::{global_target, stage_path},
    config::CompanyConfig,
    installation_repository::CompanyRepository,
    installation_service::CompanyService,
    mcp_check::{check_config, parse_requirements},
    package_store::{copy_tree, prepare_archive},
    session::MemoryCredentials,
    types::*,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};

fn pending_fixture(
    owned: bool,
) -> (
    tempfile::TempDir,
    CompanyConfig,
    InstallJournal,
    PathBuf,
    PathBuf,
    PathBuf,
) {
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        "http://127.0.0.1:4318",
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    fs::create_dir_all(&config.data_root).unwrap();
    let archive = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/company-store/packages/jira-project-summary.zip");
    let bytes = fs::read(&archive).unwrap();
    let meta = PackageMeta {
        format: "zip".into(),
        size_bytes: bytes.len() as u64,
        sha256: hex::encode(Sha256::digest(bytes)),
        etag: "\"fixture\"".into(),
    };
    let package = prepare_archive(
        &archive,
        &meta,
        "jira-project-summary",
        &config.data_root.join("packages"),
    )
    .unwrap();
    let target = global_target(&config.profile_root, "jira-project-summary").unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let stage = stage_path(&target, &id).unwrap();
    let view = OperationView {
        id: id.clone(),
        state: OperationState::Committing,
        target: target.display().to_string(),
        mcp: check_config(&parse_requirements("MCP bắt buộc: không"), b"{}"),
        mcp_path: String::new(),
        skill: None,
        progress_bytes: meta.size_bytes,
        total_bytes: meta.size_bytes,
        error: None,
    };
    let installation = Installation {
        id,
        skill_id: "skl_001".into(),
        slug: "jira-project-summary".into(),
        name: "Jira".into(),
        target: target.display().to_string(),
        package_hash: meta.sha256.clone(),
        content_hash: package.content_hash,
        description_snapshot: "MCP bắt buộc: không".into(),
        installed_at: "2026-10-12T00:00:00Z".into(),
        mcp_bypassed: false,
    };
    let journal = InstallJournal {
        view,
        installation,
        full_hash: package.full_hash.clone(),
        stage_owned: owned,
    };
    let repository = CompanyRepository::open(&config.data_root.join("manager.db")).unwrap();
    repository
        .save_package(
            &meta,
            &journal.installation.content_hash,
            &package.full_hash,
        )
        .unwrap();
    repository.prepare(&journal).unwrap();
    (root, config, journal, target, stage, package.original_root)
}

#[test]
fn company_flow03c_recovery_commits_complete_owned_target_after_rename() {
    let (_root, config, _journal, target, _stage, original) = pending_fixture(true);
    copy_tree(&original, &target).unwrap();
    let app = CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap();
    assert_eq!(app.installations().unwrap().len(), 1);
    assert!(app.recovery_warnings().is_empty());
    assert!(target.join("SKILL.md").is_file());
}

#[test]
fn company_flow03c_recovery_cancels_before_rename_and_preserves_unowned_stage() {
    for owned in [true, false] {
        let (_root, config, _journal, target, stage, original) = pending_fixture(owned);
        copy_tree(&original, &stage).unwrap();
        let app = CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap();
        assert!(app.installations().unwrap().is_empty());
        assert!(!target.exists());
        assert_eq!(stage.exists(), !owned);
        assert_eq!(app.recovery_warnings().is_empty(), owned);
    }
}

#[test]
fn company_flow03c_recovery_does_not_adopt_foreign_target_even_when_bytes_match() {
    let (_root, config, _journal, target, _stage, original) = pending_fixture(false);
    copy_tree(&original, &target).unwrap();
    let before = fs::read(target.join("SKILL.md")).unwrap();
    let app = CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap();
    assert!(app.installations().unwrap().is_empty());
    assert!(!app.recovery_warnings().is_empty());
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), before);
}

#[test]
fn company_flow03c_recovery_preserves_modified_owned_staging() {
    let (_root, config, _journal, target, stage, original) = pending_fixture(true);
    copy_tree(&original, &stage).unwrap();
    fs::write(stage.join("external-note.txt"), "keep this change").unwrap();
    let app = CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap();
    assert!(app.installations().unwrap().is_empty());
    assert!(!target.exists());
    assert!(!app.recovery_warnings().is_empty());
    assert_eq!(
        fs::read_to_string(stage.join("external-note.txt")).unwrap(),
        "keep this change"
    );
}

#[test]
fn company_flow03c_unresolved_recovery_blocks_only_its_target() {
    let server = super::support::MockServer::start("normal");
    let (_root, mut config, _journal, _target, stage, original) = pending_fixture(true);
    config.store_url = server.url.clone();
    copy_tree(&original, &stage).unwrap();
    fs::write(stage.join("external-note.txt"), "keep this change").unwrap();
    let app = CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap();
    app.login("employee01", "demo-password").unwrap();
    assert_eq!(
        app.prepare_install("skl_001").unwrap_err().code,
        "RECOVERY_REQUIRED"
    );
    assert!(app.prepare_install("skl_003").is_ok());
    assert_eq!(
        fs::read_to_string(stage.join("external-note.txt")).unwrap(),
        "keep this change"
    );
}

#[cfg(windows)]
#[test]
fn company_flow03c_recovery_keeps_foreign_junction_and_starts_with_warning() {
    let (_root, config, _journal, target, _stage, original) = pending_fixture(true);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    junction::create(&original, &target).unwrap();
    let app = CompanyService::new(config, Arc::new(MemoryCredentials::default()))
        .expect("unsafe target must not prevent opening local data");
    assert!(!app.recovery_warnings().is_empty());
    assert!(junction::exists(&target).unwrap());
    assert!(original.join("SKILL.md").is_file());
}
