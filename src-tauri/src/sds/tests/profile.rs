use super::super::profile::{Environment, SdsProfile};

#[test]
fn sds_data_root_is_separate_and_does_not_initialize_upstream() {
    let home = tempfile::tempdir().unwrap();
    let profile = SdsProfile::new(
        home.path().join("SDS/SkillsManager"),
        Environment::Internal,
        false,
        false,
    )
    .unwrap();
    assert_eq!(profile.info().kind, "sds");
    assert!(
        !profile.data_root().exists(),
        "resolution must not initialize any storage"
    );
    assert_ne!(profile.data_root(), home.path().join("skills-manager"));
}

#[test]
fn release_build_refuses_fixture_mode() {
    assert!(SdsProfile::new("test-data".into(), Environment::Development, true, false).is_err());
    assert!(SdsProfile::new("test-data".into(), Environment::Internal, true, true).is_err());
    assert!(SdsProfile::new("test-data".into(), Environment::Development, true, true).is_ok());
}

#[test]
fn explicit_debug_fixtures_use_a_child_root_separate_from_real_sds_data() {
    let root = tempfile::tempdir().unwrap();
    let normal =
        SdsProfile::new(root.path().into(), Environment::Development, false, true).unwrap();
    let fixture =
        SdsProfile::new(root.path().into(), Environment::Development, true, true).unwrap();
    assert_ne!(normal.data_root(), fixture.data_root());
    assert_eq!(
        fixture.data_root(),
        root.path().join("development-fixtures")
    );
    assert!(
        !fixture.data_root().exists(),
        "profile resolution must not create fixture targets"
    );
}

#[test]
fn environment_must_be_explicitly_valid() {
    assert_eq!(
        Environment::parse("development").unwrap(),
        Environment::Development
    );
    assert_eq!(
        Environment::parse("internal").unwrap(),
        Environment::Internal
    );
    assert!(Environment::parse("production").is_err());
}

#[test]
fn mismatched_build_identifier_is_refused_before_upstream_storage() {
    use super::super::profile::matches_identifier;
    assert!(matches_identifier(true, "com.sds.skillsmanager.desktop"));
    assert!(matches_identifier(false, "com.agentskills.desktop"));
    assert!(!matches_identifier(true, "com.agentskills.desktop"));
    assert!(!matches_identifier(false, "com.sds.skillsmanager.desktop"));
}

#[test]
fn sds_registry_has_no_public_install_or_upstream_commands() {
    let commands = super::super::runtime::COMMANDS;
    assert!(commands.contains(&"runtime_profile"));
    for command in [
        "install_from_git",
        "install_from_skillssh",
        "get_settings",
        "git_backup_sync",
    ] {
        assert!(!commands.contains(&command));
    }
}

#[test]
fn dispatch_never_initializes_upstream_in_sds_build() {
    use std::cell::Cell;
    let sds = Cell::new(0);
    let upstream = Cell::new(0);
    super::super::profile::dispatch_runtime(
        true,
        || sds.set(sds.get() + 1),
        || upstream.set(upstream.get() + 1),
    );
    assert_eq!(sds.get(), 1);
    assert_eq!(upstream.get(), 0);
}
