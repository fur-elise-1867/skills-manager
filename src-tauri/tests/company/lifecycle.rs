use super::{support::MockServer, wait_operation};
use app_lib::core::company::{
    config::CompanyConfig, installation_service::CompanyService, session::MemoryCredentials,
};
use std::{fs, path::Path, sync::Arc};

fn completed(app: &Arc<CompanyService>, id: &str) {
    let view = wait_operation(app, id);
    assert_eq!(
        serde_json::to_value(&view).unwrap()["state"],
        "succeeded",
        "{:?}",
        view.error
    );
}

#[test]
fn company_flow06_remove_and_update_cannot_hold_conflicting_decisions_for_one_target() {
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
    let app =
        Arc::new(CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let op = app.prepare_install("skl_001").unwrap();
    app.confirm(&op.id, true).unwrap();
    completed(&app, &op.id);
    let item = app.installations().unwrap().remove(0);
    let remove = app.prepare_remove(&item.id).unwrap();
    fs::write(&revision, "2").unwrap();
    assert_eq!(
        app.prepare_update(&item.id).unwrap_err().code,
        "TARGET_BUSY"
    );
    app.cancel(&remove.id).unwrap();
    assert!(app.prepare_update(&item.id).is_ok());
}

#[test]
fn company_flow06_restore_reactivates_the_archived_project_without_recreating_its_root() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start("normal");
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let app =
        Arc::new(CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let folder = root.path().join("Project");
    fs::create_dir(&folder).unwrap();
    let project = app.add_project(&folder).unwrap();
    let op = app.prepare_project_install("skl_003", &project.id).unwrap();
    app.confirm(&op.id, false).unwrap();
    completed(&app, &op.id);
    let item = app.installations().unwrap().remove(0);
    let remove = app.prepare_remove(&item.id).unwrap();
    app.apply_lifecycle(&remove.id).unwrap();
    completed(&app, &remove.id);
    let backup = app.backups(&item.id).unwrap().remove(0);
    app.remove_project(&project.id).unwrap();
    let restore = app.prepare_restore(&backup.id).unwrap();
    app.apply_lifecycle(&restore.id).unwrap();
    completed(&app, &restore.id);
    assert_eq!(
        app.projects()
            .unwrap()
            .iter()
            .map(|p| &p.id)
            .collect::<Vec<_>>(),
        vec![&project.id]
    );
    assert_eq!(
        app.remove_project(&project.id).unwrap_err().code,
        "PROJECT_IN_USE"
    );
}

#[test]
fn company_flow06_remove_preserves_other_copies_and_archived_project_can_be_registered_again() {
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
    let project_root = root.path().join("Project A");
    fs::create_dir(&project_root).unwrap();
    let project = app.add_project(&project_root).unwrap();
    let global = app.prepare_install("skl_003").unwrap();
    app.confirm(&global.id, false).unwrap();
    completed(&app, &global.id);
    let project_op = app.prepare_project_install("skl_003", &project.id).unwrap();
    app.confirm(&project_op.id, false).unwrap();
    completed(&app, &project_op.id);
    let copies = app.installations().unwrap();
    let local = copies.iter().find(|i| i.project_id.is_some()).unwrap();
    let global = copies.iter().find(|i| i.project_id.is_none()).unwrap();
    let unchanged = fs::read(Path::new(&global.target).join("SKILL.md")).unwrap();
    assert_eq!(
        app.remove_project(&project.id).unwrap_err().code,
        "PROJECT_IN_USE"
    );
    fs::write(
        Path::new(&local.target).join("SKILL.md"),
        "Customized before remove",
    )
    .unwrap();
    let remove = app.prepare_remove(&local.id).unwrap();
    app.apply_lifecycle(&remove.id).unwrap();
    completed(&app, &remove.id);
    assert!(!Path::new(&local.target).exists());
    assert!(project_root.is_dir());
    assert_eq!(
        fs::read(Path::new(&global.target).join("SKILL.md")).unwrap(),
        unchanged
    );
    let removed = app.refresh_installation(&local.id).unwrap();
    assert_eq!(serde_json::to_value(removed).unwrap()["state"], "removed");
    let backup = app.backups(&local.id).unwrap().remove(0);
    assert_eq!(backup.reason, "remove");
    app.remove_project(&project.id).unwrap();
    assert!(app.projects().unwrap().is_empty());
    assert_eq!(app.add_project(&project_root).unwrap().id, project.id);
    drop(app);
    let app = Arc::new(CompanyService::new(config, credentials).unwrap());
    let fresh = app.prepare_project_install("skl_003", &project.id).unwrap();
    app.confirm(&fresh.id, false).unwrap();
    completed(&app, &fresh.id);
    assert_eq!(
        app.prepare_restore(&backup.id).unwrap_err().code,
        "TARGET_CONFLICT"
    );
    assert_eq!(
        fs::read(Path::new(&global.target).join("SKILL.md")).unwrap(),
        unchanged
    );
}

#[test]
fn company_flow06_restore_removed_snapshot_needs_no_store_or_empty_backup_and_stays_valid_after_logout(
) {
    let root = tempfile::tempdir().unwrap();
    let mut server = MockServer::start("normal");
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let app =
        Arc::new(CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let install = app.prepare_install("skl_003").unwrap();
    app.confirm(&install.id, false).unwrap();
    completed(&app, &install.id);
    let item = app.installations().unwrap().remove(0);
    let remove = app.prepare_remove(&item.id).unwrap();
    app.apply_lifecycle(&remove.id).unwrap();
    completed(&app, &remove.id);
    let backup = app.backups(&item.id).unwrap().remove(0);
    let restore = app.prepare_restore(&backup.id).unwrap();
    app.logout().unwrap();
    server.stop();
    app.apply_lifecycle(&restore.id).unwrap();
    completed(&app, &restore.id);
    assert_eq!(app.backups(&item.id).unwrap().len(), 1);
    assert!(Path::new(&item.target).join("SKILL.md").is_file());
    let before = fs::read(Path::new(&item.target).join("SKILL.md")).unwrap();
    fs::write(Path::new(&backup.path).join("SKILL.md"), "tampered backup").unwrap();
    assert_eq!(
        app.prepare_restore(&backup.id).unwrap_err().code,
        "BACKUP_INVALID"
    );
    assert_eq!(
        fs::read(Path::new(&item.target).join("SKILL.md")).unwrap(),
        before
    );
}

#[test]
fn company_flow06_restore_offline_recovers_customization_and_original_baseline() {
    let root = tempfile::tempdir().unwrap();
    let revision = root.path().join("revision");
    fs::write(&revision, "1").unwrap();
    let mut server = MockServer::start_with_revision("normal", Some(&revision));
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let op = app.prepare_install("skl_001").unwrap();
    app.confirm(&op.id, true).unwrap();
    assert_eq!(
        serde_json::to_value(wait_operation(&app, &op.id)).unwrap()["state"],
        "succeeded"
    );
    let old = app.installations().unwrap().remove(0);
    fs::write(
        Path::new(&old.target).join("SKILL.md"),
        "My customized skill before update",
    )
    .unwrap();
    fs::write(&revision, "2").unwrap();
    let update = app.prepare_update(&old.id).unwrap();
    app.confirm(&update.id, true).unwrap();
    for _ in 0..500 {
        if app.status(&update.id).unwrap().preview_ready {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    app.apply_update(&update.id, true).unwrap();
    assert_eq!(
        serde_json::to_value(wait_operation(&app, &update.id)).unwrap()["state"],
        "succeeded"
    );
    let backup = app.backups(&old.id).unwrap().remove(0);
    let current = fs::read(Path::new(&old.target).join("SKILL.md")).unwrap();
    app.logout().unwrap();
    server.stop();
    let restore = app.prepare_restore(&backup.id).unwrap();
    assert_eq!(
        fs::read(Path::new(&old.target).join("SKILL.md")).unwrap(),
        current
    );
    app.apply_lifecycle(&restore.id).unwrap();
    assert_eq!(
        serde_json::to_value(wait_operation(&app, &restore.id)).unwrap()["state"],
        "succeeded"
    );
    assert_eq!(
        fs::read_to_string(Path::new(&old.target).join("SKILL.md")).unwrap(),
        "My customized skill before update"
    );
    let restored = app.refresh_installation(&old.id).unwrap();
    assert_eq!(restored.package_hash, old.package_hash);
    assert_eq!(
        serde_json::to_value(restored).unwrap()["contentState"],
        "customized"
    );
    assert_eq!(app.backups(&old.id).unwrap().len(), 2);
    drop(app);
    let restarted = CompanyService::new(config, credentials).unwrap();
    assert_eq!(
        restarted
            .refresh_installation(&old.id)
            .unwrap()
            .package_hash,
        old.package_hash
    );
    assert!(restarted.recovery_warnings().is_empty());
}
