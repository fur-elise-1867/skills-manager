use crate::sds::targets::resolve_aipro_global;

#[test]
fn selected_target_survives_restart_and_reset_without_creating_agent_files() {
    use crate::sds::targets::TargetService;
    let home = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let selected = tempfile::tempdir().unwrap();
    let service = TargetService::new(home.path(), settings.path());
    assert_eq!(
        service.settings().unwrap().error.unwrap().code,
        "TARGET_NOT_READY"
    );
    assert!(!home.path().join(".aipro").exists());
    let saved = service.set_override(Some(selected.path())).unwrap();
    assert_eq!(saved.target.unwrap().path, selected.path());
    let restarted = TargetService::new(home.path(), settings.path());
    assert_eq!(
        restarted.settings().unwrap().override_root.unwrap(),
        selected.path()
    );
    assert!(restarted
        .set_override(Some(&home.path().join("missing")))
        .is_err());
    assert_eq!(
        restarted.settings().unwrap().target.unwrap().path,
        selected.path()
    );
    assert_eq!(
        restarted.set_override(None).unwrap().error.unwrap().code,
        "TARGET_NOT_READY"
    );
    assert!(TargetService::new(home.path(), settings.path())
        .settings()
        .unwrap()
        .override_root
        .is_none());
    assert_eq!(std::fs::read_dir(selected.path()).unwrap().count(), 0);
}

#[test]
fn fixture_targets_are_locked_inside_the_fixture_root() {
    use crate::sds::targets::TargetService;
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let service = TargetService::fixture(root.path()).unwrap();
    let settings = service.settings().unwrap();
    assert!(settings.locked);
    assert!(settings.target.unwrap().path.starts_with(root.path()));
    assert!(service.set_override(Some(external.path())).is_err());
    assert_eq!(std::fs::read_dir(external.path()).unwrap().count(), 0);
}

#[test]
fn mcp_inspection_stays_global_when_a_custom_destination_is_selected() {
    use crate::sds::{contracts::McpState, targets::TargetService};
    let home = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let selected = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".aipro")).unwrap();
    std::fs::write(
        home.path().join(".aipro/mcp.json"),
        br#"{"mcpServers":{"global":{"command":"synthetic"}}}"#,
    )
    .unwrap();
    std::fs::write(
        selected.path().join("mcp.json"),
        br#"{"mcpServers":{"other":{"command":"synthetic"}}}"#,
    )
    .unwrap();
    let service = TargetService::new(home.path(), settings.path());
    service.set_override(Some(selected.path())).unwrap();
    assert_eq!(
        service.inspect_mcp(Some(&["global".into()])).state,
        McpState::Configured
    );
    assert_eq!(
        service.inspect_mcp(Some(&["other".into()])).state,
        McpState::Missing
    );
}

#[test]
fn desktop_fixture_services_expose_only_their_sandbox_target() {
    let root = tempfile::tempdir().unwrap();
    let services =
        crate::sds::commands::SdsServices::new_fixture(root.path(), "http://127.0.0.1:3081/skm")
            .unwrap();
    let target = services.targets.settings().unwrap().target.unwrap();
    assert_eq!(target.path, root.path().join("agent-home/.aipro/skills"));
    assert!(!target.path.exists());
    for name in ["sds_get_target_settings", "sds_set_target_override"] {
        assert!(crate::sds::runtime::COMMANDS.contains(&name));
    }
}

#[test]
fn resolves_aipro_global_without_creating_directories_and_accepts_an_explicit_existing_override() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        resolve_aipro_global(home.path(), None).unwrap_err().code,
        "TARGET_NOT_READY"
    );
    std::fs::create_dir(home.path().join(".aipro")).unwrap();
    let target = resolve_aipro_global(home.path(), None).unwrap();
    assert_eq!(target.path, home.path().join(".aipro/skills"));
    assert_eq!(target.agent_key, "aipro");
    assert_eq!(target.workspace_id, "global");
    assert!(!target.path.exists());
    let selected = tempfile::tempdir().unwrap();
    let overridden = resolve_aipro_global(home.path(), Some(selected.path())).unwrap();
    assert_eq!(overridden.path, selected.path());
}

#[test]
#[cfg(windows)]
fn target_roots_and_ancestors_cannot_be_junctions() {
    let home = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::fs::write(external.path().join("sentinel"), b"keep").unwrap();
    std::fs::create_dir(external.path().join("skills")).unwrap();
    junction::create(external.path(), home.path().join(".aipro")).unwrap();
    assert_eq!(
        resolve_aipro_global(home.path(), None).unwrap_err().code,
        "UNSAFE_TARGET"
    );
    assert_eq!(
        resolve_aipro_global(home.path(), Some(&home.path().join(".aipro/skills")))
            .unwrap_err()
            .code,
        "UNSAFE_TARGET"
    );
    assert_eq!(
        std::fs::read(external.path().join("sentinel")).unwrap(),
        b"keep"
    );
}

#[test]
fn target_resolution_rejects_files_and_aliases_without_writing_to_the_destination() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".aipro")).unwrap();
    std::fs::write(home.path().join(".aipro/skills"), b"unmanaged").unwrap();
    assert_eq!(
        resolve_aipro_global(home.path(), None).unwrap_err().code,
        "UNSAFE_TARGET"
    );
    for invalid in [
        home.path().join("folder/../skills"),
        home.path().join("folder/./skills"),
        home.path().join("skills."),
        home.path().join("skills "),
        home.path().join("skills:stream"),
        home.path().join("CON"),
        std::path::PathBuf::from("relative/skills"),
    ] {
        assert_eq!(
            resolve_aipro_global(home.path(), Some(&invalid))
                .unwrap_err()
                .code,
            "UNSAFE_TARGET"
        );
    }
    assert_eq!(
        std::fs::read(home.path().join(".aipro/skills")).unwrap(),
        b"unmanaged"
    );
}

#[test]
#[cfg(windows)]
fn user_selected_target_must_be_an_absolute_local_disk_path() {
    let home = tempfile::tempdir().unwrap();
    for invalid in [
        r"\\127.0.0.1\sds-no-network\skills",
        r"\\?\C:\skills",
        r"C:skills",
        r"\skills",
    ] {
        assert_eq!(
            resolve_aipro_global(home.path(), Some(std::path::Path::new(invalid)))
                .unwrap_err()
                .code,
            "UNSAFE_TARGET"
        );
    }
}
