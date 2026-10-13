use super::support::MockServer;
use app_lib::core::company::{
    config::CompanyConfig, installation_service::CompanyService, session::MemoryCredentials,
};
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

fn wait(app: &CompanyService, id: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let op = app.status(id).unwrap();
        let state = serde_json::to_value(&op).unwrap()["state"]
            .as_str()
            .unwrap()
            .to_string();
        if state == "succeeded" {
            return;
        }
        assert!(
            !matches!(state.as_str(), "failed" | "cancelled" | "recovery_required"),
            "{:?}",
            op.error
        );
        assert!(Instant::now() < deadline, "operation timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn company_flow04_projects_install_independent_copies_and_keep_metadata_after_restart() {
    let server = MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let app = Arc::new(CompanyService::new(config.clone(), credentials.clone()).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let a = root.path().join("Project A");
    let b = root.path().join("Project B");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let project_a = app.add_project(&a).unwrap();
    let project_b = app.add_project(&b).unwrap();
    assert_eq!(app.add_project(&a.join(".")).unwrap().id, project_a.id);
    assert!(!a.join(".aipro").exists());
    assert!(!a.join(".claude").exists());
    let global = app.prepare_install("skl_001").unwrap();
    app.confirm(&global.id, true).unwrap();
    wait(&app, &global.id);
    for project in [&project_a, &project_b] {
        let op = app.prepare_project_install("skl_001", &project.id).unwrap();
        assert_eq!(op.mcp_path, global.mcp_path);
        app.confirm(&op.id, true).unwrap();
        wait(&app, &op.id);
    }
    let installed = app.installations().unwrap();
    assert_eq!(installed.len(), 3);
    let item_a = installed
        .iter()
        .find(|i| i.project_id.as_deref() == Some(&project_a.id))
        .unwrap();
    let target_a = std::path::Path::new(&item_a.target);
    fs::write(target_a.join("SKILL.md"), "My project A customization").unwrap();
    let refreshed = app.refresh_installation(&item_a.id).unwrap();
    assert_eq!(
        serde_json::to_value(refreshed).unwrap()["contentState"],
        "customized"
    );
    for other in installed.iter().filter(|i| i.id != item_a.id) {
        assert_ne!(
            fs::read_to_string(std::path::Path::new(&other.target).join("SKILL.md")).unwrap(),
            "My project A customization"
        );
        assert_eq!(
            serde_json::to_value(app.refresh_installation(&other.id).unwrap()).unwrap()
                ["contentState"],
            "original"
        );
    }
    let original = config
        .data_root
        .join("packages")
        .join(&item_a.package_hash)
        .join("original/SKILL.md");
    assert_ne!(
        fs::read_to_string(original).unwrap(),
        "My project A customization"
    );
    let reopened = CompanyService::new(config, credentials).unwrap();
    assert_eq!(reopened.projects().unwrap().len(), 2);
    assert_eq!(reopened.installations().unwrap().len(), 3);
}

#[test]
fn company_flow04_missing_installation_can_be_reinstalled_without_duplicate_ownership() {
    let server = MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let app =
        Arc::new(CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let first = app.prepare_install("skl_001").unwrap();
    app.confirm(&first.id, true).unwrap();
    wait(&app, &first.id);
    let item = app.installations().unwrap().remove(0);
    let target = std::path::Path::new(&item.target);
    assert!(target.starts_with(root.path()));
    fs::rename(target, root.path().join("user-moved-skill")).unwrap();
    assert_eq!(
        serde_json::to_value(app.refresh_installation(&item.id).unwrap()).unwrap()["contentState"],
        "missing"
    );
    let retry = app.prepare_install("skl_001").unwrap();
    app.confirm(&retry.id, true).unwrap();
    wait(&app, &retry.id);
    let all = app.installations().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, item.id);
    assert!(target.join("SKILL.md").is_file());
    assert!(root.path().join("user-moved-skill/SKILL.md").is_file());
}

#[cfg(windows)]
#[test]
fn company_flow04_legacy_database_keeps_installation_identity_after_path_key_migration() {
    use app_lib::core::company::package_store::content_hash;
    let server = MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let target = config
        .profile_root
        .join(".aipro/skills/jira-project-summary");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&config.data_root).unwrap();
    fs::write(target.join("SKILL.md"), "Legacy managed Jira").unwrap();
    let canonical = fs::canonicalize(&target).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let payload = serde_json::json!({ "id": id, "skillId":"skl_001", "slug":"jira-project-summary", "name":"Legacy Jira", "target":canonical.display().to_string(), "packageHash":"a".repeat(64), "contentHash":content_hash(&target, true).unwrap(), "descriptionSnapshot":"MCP bắt buộc: không", "installedAt":"2026-10-10T00:00:00Z", "mcpBypassed":false });
    let db = rusqlite::Connection::open(config.data_root.join("manager.db")).unwrap();
    db.execute_batch("CREATE TABLE company_installations(id TEXT PRIMARY KEY,target_key TEXT UNIQUE NOT NULL,payload TEXT NOT NULL); PRAGMA user_version=2;").unwrap();
    db.execute(
        "INSERT INTO company_installations VALUES(?1,?2,?3)",
        rusqlite::params![
            id,
            canonical
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase(),
            payload.to_string()
        ],
    )
    .unwrap();
    drop(db);
    let app =
        Arc::new(CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap());
    assert_eq!(app.installations().unwrap()[0].id, id);
    assert!(app.installations().unwrap()[0].project_id.is_none());
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "Legacy managed Jira"
    );
    fs::rename(&target, root.path().join("legacy-preserved")).unwrap();
    app.login("employee01", "demo-password").unwrap();
    let retry = app.prepare_install("skl_001").unwrap();
    app.confirm(&retry.id, true).unwrap();
    wait(&app, &retry.id);
    let all = app.installations().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, id);
    assert_eq!(
        fs::read_to_string(root.path().join("legacy-preserved/SKILL.md")).unwrap(),
        "Legacy managed Jira"
    );
}

#[test]
fn company_flow04_moved_project_shows_missing_without_recreating_it() {
    let server = MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let app =
        Arc::new(CompanyService::new(config, Arc::new(MemoryCredentials::default())).unwrap());
    app.login("employee01", "demo-password").unwrap();
    let path = root.path().join("Project A");
    fs::create_dir(&path).unwrap();
    let project = app.add_project(&path).unwrap();
    let op = app.prepare_project_install("skl_001", &project.id).unwrap();
    app.confirm(&op.id, true).unwrap();
    wait(&app, &op.id);
    let item = app.installations().unwrap().remove(0);
    fs::rename(&path, root.path().join("Moved Project A")).unwrap();
    assert_eq!(
        serde_json::to_value(app.installations().unwrap()[0].clone()).unwrap()["contentState"],
        "missing"
    );
    assert_eq!(
        serde_json::to_value(app.refresh_installation(&item.id).unwrap()).unwrap()["contentState"],
        "missing"
    );
    assert_eq!(
        app.prepare_project_install("skl_001", &project.id)
            .unwrap_err()
            .code,
        "PROJECT_MISSING"
    );
    assert!(!path.exists());
    assert!(root
        .path()
        .join("Moved Project A/.aipro/skills/jira-project-summary/SKILL.md")
        .is_file());
}
