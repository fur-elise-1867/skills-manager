use super::wait_operation;
use app_lib::core::company::{
    config::CompanyConfig, installation_service::CompanyService, session::MemoryCredentials,
};
use std::{fs, path::Path, sync::Arc};

#[test]
fn company_flow07_import_changed_or_unsafe_local_archives_never_write_the_target() {
    let root = tempfile::tempdir().unwrap();
    let config =
        CompanyConfig::for_test("", root.path().join("profile"), root.path().join("data")).unwrap();
    let app = Arc::new(
        CompanyService::new(config.clone(), Arc::new(MemoryCredentials::default())).unwrap(),
    );
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/company-store/packages/jira-project-summary.zip");
    let input = root.path().join("input.zip");
    fs::copy(&fixture, &input).unwrap();
    let prepared = app
        .prepare_local_import(&input, "jira-project-summary", None)
        .unwrap();
    let mut tampered = fs::read(&input).unwrap();
    tampered[0] ^= 1;
    fs::write(&input, tampered).unwrap();
    app.confirm(&prepared.id, true).unwrap();
    assert_eq!(
        wait_operation(&app, &prepared.id).error.unwrap().code,
        "PACKAGE_INTEGRITY_FAILED"
    );
    assert!(!config
        .profile_root
        .join(".aipro/skills/jira-project-summary")
        .exists());
    let malformed = app
        .prepare_local_import(&input, "jira-project-summary", None)
        .unwrap();
    app.confirm(&malformed.id, true).unwrap();
    assert_eq!(
        wait_operation(&app, &malformed.id).state,
        app_lib::core::company::types::OperationState::Failed
    );
    assert!(app.installations().unwrap().is_empty());
    assert!(app.prepare_local_import(&fixture, "../bad", None).is_err());
    let wrapped = root.path().join("wrapped.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&wrapped).unwrap());
    zip.start_file(
        "wrong-wrapper/SKILL.md",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    std::io::Write::write_all(&mut zip, b"Valid skill content").unwrap();
    zip.finish().unwrap();
    let wrong_wrapper = app
        .prepare_local_import(&wrapped, "another-slug", None)
        .unwrap();
    app.confirm(&wrong_wrapper.id, true).unwrap();
    assert_eq!(
        wait_operation(&app, &wrong_wrapper.id).error.unwrap().code,
        "INVALID_PACKAGE"
    );
    assert!(!config
        .profile_root
        .join(".aipro/skills/another-slug")
        .exists());
}

#[test]
fn company_flow07_import_offline_zip_and_7z_are_local_copies_and_require_mcp_bypass() {
    for (file, slug) in [
        ("jira-project-summary.zip", "jira-project-summary"),
        ("excel-insights.7z", "excel-insights"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let config =
            CompanyConfig::for_test("", root.path().join("profile"), root.path().join("data"))
                .unwrap();
        let app = Arc::new(
            CompanyService::new(config.clone(), Arc::new(MemoryCredentials::default())).unwrap(),
        );
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("fixtures/company-store/packages")
            .join(file);
        let stopped = app.prepare_local_import(&fixture, slug, None).unwrap();
        app.cancel(&stopped.id).unwrap();
        assert!(!config
            .profile_root
            .join(".aipro/skills")
            .join(slug)
            .exists());
        let op = app.prepare_local_import(&fixture, slug, None).unwrap();
        assert_eq!(app.confirm(&op.id, false).unwrap_err().code, "MCP_WARNING");
        app.confirm(&op.id, true).unwrap();
        let result = wait_operation(&app, &op.id);
        assert_eq!(
            serde_json::to_value(&result).unwrap()["state"],
            "succeeded",
            "{:?}",
            result.error
        );
        let item = app.installations().unwrap().remove(0);
        let dto = serde_json::to_value(&item).unwrap();
        assert_eq!(dto["origin"], "local");
        assert!(dto["skillId"].is_null());
        assert_eq!(dto["storeState"], "not_applicable");
        assert!(Path::new(&item.target).join("SKILL.md").is_file());
        let checked = app.check_updates(&[item.id.clone()]).unwrap();
        assert_eq!(
            serde_json::to_value(&checked[0]).unwrap()["storeState"],
            "not_applicable"
        );
        assert_eq!(
            app.prepare_update(&item.id).unwrap_err().code,
            "LOCAL_SOURCE"
        );
        let folder = root.path().join("Project");
        fs::create_dir(&folder).unwrap();
        let project = app.add_project(&folder).unwrap();
        let project_op = app
            .prepare_local_import(&fixture, slug, Some(&project.id))
            .unwrap();
        app.confirm(&project_op.id, true).unwrap();
        assert_eq!(
            serde_json::to_value(wait_operation(&app, &project_op.id)).unwrap()["state"],
            "succeeded"
        );
        fs::write(
            Path::new(&item.target).join("SKILL.md"),
            "Global customization",
        )
        .unwrap();
        assert_ne!(
            fs::read_to_string(folder.join(".aipro/skills").join(slug).join("SKILL.md")).unwrap(),
            "Global customization"
        );
    }
}
