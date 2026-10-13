use super::support::MockServer;
use app_lib::core::company::{
    config::CompanyConfig, installation_service::CompanyService, session::MemoryCredentials,
};
use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

fn wait(
    app: &CompanyService,
    id: &str,
    preview: bool,
) -> app_lib::core::company::types::OperationView {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let op = app.status(id).unwrap();
        let state = serde_json::to_value(&op).unwrap()["state"]
            .as_str()
            .unwrap()
            .to_string();
        if state == "succeeded" || (preview && op.preview_ready) {
            return op;
        }
        assert!(
            !matches!(state.as_str(), "failed" | "cancelled" | "recovery_required"),
            "{:?}",
            op.error
        );
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn company_flow05_update_retains_customized_backup_and_changes_only_the_selected_target() {
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
    let global_op = app.prepare_install("skl_001").unwrap();
    app.confirm(&global_op.id, true).unwrap();
    wait(&app, &global_op.id, false);
    let project_dir = root.path().join("Project A");
    fs::create_dir(&project_dir).unwrap();
    let project = app.add_project(&project_dir).unwrap();
    let project_op = app.prepare_project_install("skl_001", &project.id).unwrap();
    app.confirm(&project_op.id, true).unwrap();
    wait(&app, &project_op.id, false);
    let all = app.installations().unwrap();
    let global = all.iter().find(|i| i.project_id.is_none()).unwrap();
    let local = all.iter().find(|i| i.project_id.is_some()).unwrap();
    let before_global = fs::read(Path::new(&global.target).join("SKILL.md")).unwrap();
    fs::write(
        Path::new(&local.target).join("SKILL.md"),
        "My customized project skill",
    )
    .unwrap();
    fs::write(&revision, "2").unwrap();
    let checked = app
        .check_updates(&[global.id.clone(), local.id.clone()])
        .unwrap();
    assert!(checked
        .iter()
        .all(|i| serde_json::to_value(i).unwrap()["storeState"] == "new_package"));
    let prepared = app.prepare_update(&local.id).unwrap();
    app.confirm(&prepared.id, true).unwrap();
    let preview = wait(&app, &prepared.id, true);
    assert!(preview.changes.iter().any(|f| f.path == "SKILL.md"));
    assert_eq!(
        fs::read_to_string(Path::new(&local.target).join("SKILL.md")).unwrap(),
        "My customized project skill"
    );
    assert_eq!(
        app.apply_update(&prepared.id, false).unwrap_err().code,
        "CUSTOMIZED_CONFIRMATION_REQUIRED"
    );
    app.apply_update(&prepared.id, true).unwrap();
    wait(&app, &prepared.id, false);
    let after = app.refresh_installation(&local.id).unwrap();
    assert_ne!(after.package_hash, local.package_hash);
    assert_eq!(
        serde_json::to_value(&after).unwrap()["contentState"],
        "original"
    );
    assert_eq!(
        fs::read(Path::new(&global.target).join("SKILL.md")).unwrap(),
        before_global
    );
    let backups = app.backups(&local.id).unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].installation.package_hash, local.package_hash);
    assert_eq!(
        fs::read_to_string(Path::new(&backups[0].path).join("SKILL.md")).unwrap(),
        "My customized project skill"
    );
    let reopened = CompanyService::new(config, credentials).unwrap();
    assert_eq!(reopened.backups(&local.id).unwrap().len(), 1);
    assert_eq!(
        reopened
            .refresh_installation(&local.id)
            .unwrap()
            .package_hash,
        after.package_hash
    );
}

fn finished(app: &CompanyService, id: &str) -> app_lib::core::company::types::OperationView {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let view = app.status(id).unwrap();
        let state = serde_json::to_value(&view).unwrap()["state"]
            .as_str()
            .unwrap()
            .to_string();
        if matches!(
            state.as_str(),
            "succeeded" | "failed" | "cancelled" | "recovery_required"
        ) {
            return view;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn company_flow05_editing_after_preview_is_preserved_and_preview_edits_never_change_baselines() {
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
    let app = Arc::new(
        CompanyService::new(config.clone(), Arc::new(MemoryCredentials::default())).unwrap(),
    );
    app.login("employee01", "demo-password").unwrap();
    let install = app.prepare_install("skl_001").unwrap();
    app.confirm(&install.id, true).unwrap();
    wait(&app, &install.id, false);
    let item = app.installations().unwrap().remove(0);
    fs::write(&revision, "2").unwrap();
    let op = app.prepare_update(&item.id).unwrap();
    app.confirm(&op.id, true).unwrap();
    wait(&app, &op.id, true);
    fs::write(
        Path::new(&item.target).join("SKILL.md"),
        "Edit made AFTER preview",
    )
    .unwrap();
    app.apply_update(&op.id, true).unwrap();
    let result = finished(&app, &op.id);
    assert_eq!(result.error.unwrap().code, "CONTENT_CHANGED");
    assert_eq!(
        fs::read_to_string(Path::new(&item.target).join("SKILL.md")).unwrap(),
        "Edit made AFTER preview"
    );
    assert!(app.backups(&item.id).unwrap().is_empty());
    let fresh = app.prepare_update(&item.id).unwrap();
    app.confirm(&fresh.id, true).unwrap();
    wait(&app, &fresh.id, true);
    assert!(app
        .preview_open_path(&fresh.id, "incoming", "../SKILL.md")
        .is_err());
    let preview = app
        .preview_open_path(&fresh.id, "incoming", "SKILL.md")
        .unwrap();
    fs::write(preview, "Edit to preview copy").unwrap();
    app.apply_update(&fresh.id, true).unwrap();
    wait(&app, &fresh.id, false);
    assert!(fs::read_to_string(Path::new(&item.target).join("SKILL.md"))
        .unwrap()
        .contains("Jira project summary v2"));
    assert_eq!(
        fs::read_to_string(Path::new(&app.backups(&item.id).unwrap()[0].path).join("SKILL.md"))
            .unwrap(),
        "Edit made AFTER preview"
    );
}

#[test]
fn company_flow05_backup_failure_preserves_the_current_copy_and_foreign_backup_root() {
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
    let app = Arc::new(
        CompanyService::new(config.clone(), Arc::new(MemoryCredentials::default())).unwrap(),
    );
    app.login("employee01", "demo-password").unwrap();
    let install = app.prepare_install("skl_001").unwrap();
    app.confirm(&install.id, true).unwrap();
    wait(&app, &install.id, false);
    let item = app.installations().unwrap().remove(0);
    let original = fs::read(Path::new(&item.target).join("SKILL.md")).unwrap();
    fs::write(&revision, "2").unwrap();
    let op = app.prepare_update(&item.id).unwrap();
    app.confirm(&op.id, true).unwrap();
    wait(&app, &op.id, true);
    fs::write(
        config.data_root.join("backups"),
        "Foreign file: do not remove",
    )
    .unwrap();
    app.apply_update(&op.id, false).unwrap();
    let result = finished(&app, &op.id);
    assert_eq!(serde_json::to_value(result).unwrap()["state"], "failed");
    assert_eq!(
        fs::read(Path::new(&item.target).join("SKILL.md")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(config.data_root.join("backups")).unwrap(),
        "Foreign file: do not remove"
    );
    assert_eq!(
        app.refresh_installation(&item.id).unwrap().package_hash,
        item.package_hash
    );
}

#[test]
fn company_flow05_preview_opens_text_copies_but_never_binary_or_executable_files() {
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
    let install = app.prepare_install("skl_001").unwrap();
    app.confirm(&install.id, true).unwrap();
    wait(&app, &install.id, false);
    let item = app.installations().unwrap().remove(0);
    let target = Path::new(&item.target);
    fs::write(target.join("unsafe.bat"), "echo this must never execute").unwrap();
    fs::write(target.join("binary.txt"), b"\0binary\xff").unwrap();
    fs::write(target.join("notes.txt"), "local text note").unwrap();
    fs::write(&revision, "2").unwrap();
    let op = app.prepare_update(&item.id).unwrap();
    app.confirm(&op.id, true).unwrap();
    let preview = wait(&app, &op.id, true);
    assert_eq!(
        app.preview_open_path(&op.id, "current", "unsafe.bat")
            .unwrap_err()
            .code,
        "PREVIEW_UNAVAILABLE"
    );
    assert_eq!(
        app.preview_open_path(&op.id, "current", "binary.txt")
            .unwrap_err()
            .code,
        "PREVIEW_UNAVAILABLE"
    );
    assert_eq!(
        serde_json::to_value(
            preview
                .changes
                .iter()
                .find(|f| f.path == "unsafe.bat")
                .unwrap()
        )
        .unwrap()["current"]["kind"],
        "binary"
    );
    assert!(app
        .preview_open_path(&op.id, "current", "notes.txt")
        .unwrap()
        .is_file());
    let folder = app.preview_open_path(&op.id, "incoming", "").unwrap();
    assert!(folder.is_dir());
    // Recheck the copy at the opening boundary, even if it was text when prepared.
    fs::write(folder.join("SKILL.md"), b"\0tampered binary").unwrap();
    assert_eq!(
        app.preview_open_path(&op.id, "incoming", "SKILL.md")
            .unwrap_err()
            .code,
        "PREVIEW_UNAVAILABLE"
    );
}

#[test]
fn company_flow05_live_edit_after_old_rename_returns_the_edited_copy_to_its_target() {
    use std::io::{Seek, SeekFrom, Write};
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
    let item = app.installations().unwrap().remove(0);
    let target = Path::new(&item.target);
    // A sizeable resource keeps the post-rename verification in progress while
    // a real filesystem writer edits the directory that was just moved.
    let resource = target.join("resource.bin");
    fs::write(&resource, vec![0; 32 * 1024 * 1024]).unwrap();
    fs::write(&revision, "2").unwrap();
    let op = app.prepare_update(&item.id).unwrap();
    app.confirm(&op.id, true).unwrap();
    wait(&app, &op.id, true);
    let rollback = target
        .parent()
        .unwrap()
        .join(format!(".company-rollback-{}", op.id));
    let writer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !rollback.is_dir() {
            assert!(Instant::now() < deadline, "old directory was not moved");
            std::thread::yield_now();
        }
        let mut editor = fs::OpenOptions::new()
            .write(true)
            .open(rollback.join("resource.bin"))
            .unwrap();
        editor.seek(SeekFrom::End(-1)).unwrap();
        editor.write_all(&[42]).unwrap();
        editor.sync_all().unwrap();
    });
    app.apply_update(&op.id, true).unwrap();
    let result = finished(&app, &op.id);
    writer.join().unwrap();
    assert_eq!(result.error.unwrap().code, "CONTENT_CHANGED");
    assert_eq!(fs::read(&resource).unwrap().last(), Some(&42));
    assert!(target.join("SKILL.md").is_file());
    assert_eq!(
        app.refresh_installation(&item.id).unwrap().package_hash,
        item.package_hash
    );
    let backup = app.backups(&item.id).unwrap().remove(0);
    assert_eq!(
        fs::read(Path::new(&backup.path).join("resource.bin"))
            .unwrap()
            .last(),
        Some(&0)
    );
    let restarted = CompanyService::new(config, credentials).unwrap();
    assert!(restarted.recovery_warnings().is_empty());
    assert_eq!(
        serde_json::to_value(restarted.refresh_installation(&item.id).unwrap()).unwrap()
            ["contentState"],
        "customized"
    );
}
