use app_lib::core::company::mcp_check::{check_config, parse_requirements};
use app_lib::core::company::{
    config::CompanyConfig,
    session::{MemoryCredentials, SessionService},
};
use std::sync::Arc;
#[path = "company/recovery.rs"]
mod recovery;
#[path = "company/support.rs"]
mod support;
use app_lib::core::company::installation_service::CompanyService;
use app_lib::core::company::store_client::StoreClient;
use app_lib::core::company::{package_store::prepare_archive, types::PackageMeta};

#[test]
fn company_flow03c_global_install_is_independent_and_survives_restart() {
    let server = support::MockServer::start("normal");
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
    let stopped = app.prepare_install("skl_001").unwrap();
    app.cancel(&stopped.id).unwrap();
    assert!(!config
        .profile_root
        .join(".aipro/skills/jira-project-summary")
        .exists());
    let op = app.prepare_install("skl_001").unwrap();
    app.confirm(&op.id, true).unwrap();
    let result = wait_operation(&app, &op.id);
    assert_eq!(
        serde_json::to_value(&result).unwrap()["state"],
        "succeeded",
        "{:?}",
        result.error
    );
    let installations = app.installations().unwrap();
    assert_eq!(installations.len(), 1);
    let target = std::path::Path::new(&installations[0].target);
    let original = config
        .data_root
        .join("packages")
        .join(&installations[0].package_hash)
        .join("original/SKILL.md");
    let baseline = std::fs::read(&original).unwrap();
    std::fs::write(target.join("SKILL.md"), "customized locally").unwrap();
    assert_eq!(std::fs::read(original).unwrap(), baseline);
    assert_eq!(
        CompanyService::new(config, credentials)
            .unwrap()
            .installations()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn company_flow03c_tampered_or_changed_package_never_writes_target() {
    for (scenario, expected) in [
        ("corrupt-download", "PACKAGE_INTEGRITY_FAILED"),
        ("package-changed", "PACKAGE_CHANGED"),
        ("forbidden-download", "FORBIDDEN"),
    ] {
        let server = support::MockServer::start(scenario);
        let root = tempfile::tempdir().unwrap();
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
        let op = app.prepare_install("skl_001").unwrap();
        app.confirm(&op.id, true).unwrap();
        let result = wait_operation(&app, &op.id);
        assert_eq!(result.error.unwrap().code, expected);
        assert!(!config
            .profile_root
            .join(".aipro/skills/jira-project-summary")
            .exists());
        assert!(app.installations().unwrap().is_empty());
    }
}

#[test]
fn company_flow03c_existing_target_and_foreign_staging_are_preserved() {
    let server = support::MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
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
    let op = app.prepare_install("skl_003").unwrap();
    let stage = config
        .profile_root
        .join(".aipro/skills")
        .join(format!(".company-stage-{}", op.id));
    std::fs::create_dir_all(&stage).unwrap();
    std::fs::write(stage.join("foreign.txt"), "keep me").unwrap();
    app.confirm(&op.id, false).unwrap();
    assert_eq!(
        wait_operation(&app, &op.id).error.unwrap().code,
        "TARGET_CONFLICT"
    );
    assert_eq!(
        std::fs::read_to_string(stage.join("foreign.txt")).unwrap(),
        "keep me"
    );
    let target = config.profile_root.join(".aipro/skills/writing-review");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), "user content").unwrap();
    assert_eq!(
        app.prepare_install("skl_003").unwrap_err().code,
        "TARGET_CONFLICT"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "user content"
    );
}

#[test]
fn company_flow03c_requires_fresh_mcp_check_after_manual_config_change() {
    let server = support::MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
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
    let old = app.prepare_install("skl_001").unwrap();
    app.cancel(&old.id).unwrap();
    std::fs::create_dir_all(config.profile_root.join(".aipro")).unwrap();
    std::fs::write(
        config.profile_root.join(".aipro/mcp.json"),
        br#"{"mcpServers":{"jira-mcp":{},"confluence-mcp":{}}}"#,
    )
    .unwrap();
    let fresh = app.prepare_install("skl_001").unwrap();
    assert_eq!(serde_json::to_value(fresh.mcp).unwrap()["state"], "present");
    assert!(app.confirm(&old.id, true).is_err());
}

fn wait_operation(
    app: &Arc<CompanyService>,
    id: &str,
) -> app_lib::core::company::types::OperationView {
    for _ in 0..200 {
        let op = app.status(id).unwrap();
        if matches!(
            serde_json::to_value(&op).unwrap()["state"].as_str(),
            Some("succeeded" | "failed" | "cancelled" | "recovery_required")
        ) {
            return op;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("operation did not finish within 10 seconds");
}

#[test]
fn company_flow03b_prepares_real_zip_and_7z_without_external_extractor() {
    use sha2::{Digest, Sha256};
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/company-store/packages");
    for (name, slug, format) in [
        ("jira-project-summary.zip", "jira-project-summary", "zip"),
        ("excel-insights.7z", "excel-insights", "7z"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = fixture_root.join(name);
        let bytes = std::fs::read(&path).unwrap();
        let meta = PackageMeta {
            format: format.into(),
            size_bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            etag: "\"fixture\"".into(),
        };
        let verified = prepare_archive(&path, &meta, slug, root.path()).unwrap();
        assert!(verified.original_root.join("SKILL.md").is_file());
        assert!(!verified.content_hash.is_empty());
    }
}

#[test]
fn company_flow03b_rejects_unsafe_duplicate_and_markerless_archives() {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    for names in [
        vec!["SKILL.md", "../escape.txt"],
        vec!["SKILL.md", "skill.md"],
        vec!["SKILL.md", "CON.txt"],
        vec!["SKILL.md", "folder/file:stream"],
        vec!["other.txt"],
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("input.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for name in names {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"fixture text").unwrap();
        }
        writer.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let meta = PackageMeta {
            format: "zip".into(),
            size_bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(bytes)),
            etag: "\"fixture\"".into(),
        };
        assert_eq!(
            prepare_archive(&path, &meta, "fixture-skill", &root.path().join("packages"))
                .err()
                .unwrap()
                .code,
            "INVALID_PACKAGE"
        );
        assert!(!root.path().join("escape.txt").exists());
    }
}

#[test]
fn company_flow01_login_restores_and_offline_logout_stays_signed_out() {
    let mut server = support::MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let credentials = Arc::new(MemoryCredentials::default());
    let session = SessionService::new(&config, credentials.clone()).unwrap();
    let view = session.login("employee01", "demo-password").unwrap();
    assert_eq!(view.user.unwrap().id, "usr_001");
    assert!(view.persistent);
    let restored = SessionService::new(&config, credentials.clone()).unwrap();
    assert_eq!(restored.current().user.unwrap().username, "employee01");
    server.stop();
    assert!(restored.logout().is_err());
    assert!(restored.current().user.is_none());
    assert!(SessionService::new(&config, credentials)
        .unwrap()
        .current()
        .user
        .is_none());
}

#[test]
fn company_flow02_authenticated_catalog_keeps_full_description() {
    let server = support::MockServer::start("normal");
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        &server.url,
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let session = SessionService::new(&config, Arc::new(MemoryCredentials::default())).unwrap();
    assert_eq!(
        session.login("bad", "bad").unwrap_err().code,
        "INVALID_CREDENTIALS"
    );
    session.login("employee01", "demo-password").unwrap();
    let client = StoreClient::new(&config).unwrap();
    let token = session.access_token().unwrap();
    let page = client.search(&token, "jira", 1).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.pagination.page_size, 20);
    let detail = client.detail(&token, "skl_001").unwrap();
    assert!(detail
        .description
        .unwrap()
        .contains("\n\nMCP bắt buộc: jira-mcp, confluence-mcp\n"));
    assert_eq!(
        client.detail(&token, "unknown").unwrap_err().code,
        "SKILL_NOT_FOUND"
    );
}

#[test]
fn company_flow01_signed_out_session_does_not_require_a_live_store() {
    let root = tempfile::tempdir().unwrap();
    let config = CompanyConfig::for_test(
        "http://127.0.0.1:4318",
        root.path().join("profile"),
        root.path().join("data"),
    )
    .unwrap();
    let session = SessionService::new(&config, Arc::new(MemoryCredentials::default())).unwrap();
    assert!(session.current().user.is_none());
}

#[test]
fn company_flow03a_compares_exact_keys_without_reading_server_values() {
    let requirements = parse_requirements("MCP bắt buộc: github, GitHub, github");
    let report = check_config(&requirements, br#"{"mcpServers":{"github":null}}"#);
    assert_eq!(report.missing, vec!["GitHub"]);
    assert_eq!(report.present, vec!["github"]);
}

#[test]
fn company_flow03a_invalid_declaration_does_not_claim_requirements_satisfied() {
    let requirements = parse_requirements("MCP bắt buộc: không, jira-mcp");
    let report = check_config(&requirements, br#"{"mcpServers":{"jira-mcp":{}}}"#);
    assert!(!report.reason.is_none());
}
