use super::wait_operation;
use app_lib::core::company::{
    config::CompanyConfig, installation_service::CompanyService, session::MemoryCredentials,
};
use std::{path::Path, sync::Arc};

#[test]
fn company_flow07_storage_rechecks_active_backup_and_pending_restore_references_before_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let config =
        CompanyConfig::for_test("", root.path().join("profile"), root.path().join("data")).unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/company-store/packages/jira-project-summary.zip");
    let op = app
        .prepare_local_import(&fixture, "jira-project-summary", None)
        .unwrap();
    app.confirm(&op.id, true).unwrap();
    assert_eq!(
        wait_operation(&app, &op.id).state,
        app_lib::core::company::types::OperationState::Succeeded
    );
    let item = app.installations().unwrap().remove(0);
    let hash = item.package_hash.clone();
    assert!(app.storage_status().unwrap().package_bytes > 0);
    assert_eq!(
        app.cleanup_packages(&[hash.clone()]).unwrap_err().code,
        "PACKAGE_IN_USE"
    );
    let remove = app.prepare_remove(&item.id).unwrap();
    app.apply_lifecycle(&remove.id).unwrap();
    assert_eq!(
        wait_operation(&app, &remove.id).state,
        app_lib::core::company::types::OperationState::Succeeded
    );
    let backup = app.backups(&item.id).unwrap().remove(0);
    let storage = app.storage_status().unwrap();
    assert!(storage.backup_bytes > 0);
    assert_eq!(storage.unreferenced_package_bytes, 0);
    assert_eq!(
        app.cleanup_packages(&[hash.clone()]).unwrap_err().code,
        "PACKAGE_IN_USE"
    );
    let restore = app.prepare_restore(&backup.id).unwrap();
    assert_eq!(
        app.delete_backup(&backup.id).unwrap_err().code,
        "BACKUP_IN_USE"
    );
    app.cancel(&restore.id).unwrap();
    app.delete_backup(&backup.id).unwrap();
    assert!(app.backups(&item.id).unwrap().is_empty());
    assert!(app.storage_status().unwrap().unreferenced_package_bytes > 0);
    let pending_import = app
        .prepare_local_import(&fixture, "another-skill", None)
        .unwrap();
    assert_eq!(
        app.cleanup_packages(&[hash.clone()]).unwrap_err().code,
        "PACKAGE_IN_USE"
    );
    app.cancel(&pending_import.id).unwrap();
    let package_path = config.data_root.join("packages").join(&hash);
    #[cfg(windows)]
    {
        use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(package_path.join("archive.zip"))
            .unwrap();
        assert!(app.cleanup_packages(&[hash.clone()]).is_err());
        assert!(app
            .storage_status()
            .unwrap()
            .packages
            .iter()
            .any(|p| p.hash == hash));
        drop(lock);
    }
    assert_eq!(app.cleanup_packages(&[hash.clone()]).unwrap(), vec![hash]);
    assert!(!package_path.exists());
    drop(app);
    let restarted = CompanyService::new(config, credentials).unwrap();
    assert_eq!(restarted.storage_status().unwrap().package_bytes, 0);
    assert_eq!(
        serde_json::to_value(&restarted.installations().unwrap()[0]).unwrap()["state"],
        "removed"
    );
}
