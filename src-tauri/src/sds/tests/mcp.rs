use crate::sds::{contracts::McpState, mcp::inspect_mcp};

#[test]
fn absent_requirements_are_unknown_but_an_explicit_empty_list_needs_no_mcp() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("mcp.json");
    assert_eq!(inspect_mcp(&config, None).state, McpState::Unknown);
    let no_requirements: Vec<String> = vec![];
    assert_eq!(
        inspect_mcp(&config, Some(&no_requirements)).state,
        McpState::NotRequired
    );
    assert!(!config.exists());
}

#[test]
fn checks_exact_mcp_keys_on_each_call_without_exporting_configuration_or_changing_the_file() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("mcp.json");
    let required = vec!["jira-mcp".into()];
    let missing = inspect_mcp(&config, Some(&required));
    assert_eq!(missing.state, McpState::Missing);
    assert_eq!(missing.missing, required);
    for contents in [
        r#"{"mcpServers":{"Jira-mcp":{"command":"synthetic-secret"}}}"#,
        r#"{"mcpServers":{"jira-mcp":{}}}"#,
        r#"{"mcpServers":{"jira-mcp":"invalid"}}"#,
    ] {
        std::fs::write(&config, contents).unwrap();
        assert_eq!(
            inspect_mcp(&config, Some(&required)).state,
            McpState::Missing
        );
        assert_eq!(std::fs::read_to_string(&config).unwrap(), contents);
    }
    let contents = r#"{"mcpServers":{"jira-mcp":{"command":"synthetic-secret","env":{"TOKEN":"synthetic-token"},"url":"https://private.example.invalid/?token=synthetic-token"}}}"#;
    std::fs::write(&config, contents).unwrap();
    let found = inspect_mcp(&config, Some(&required));
    assert_eq!(found.state, McpState::Configured);
    assert!(found.missing.is_empty());
    let view = serde_json::to_string(&found).unwrap();
    assert!(!view.contains("synthetic-"));
    assert!(!view.contains("https://"));
    chrono::DateTime::parse_from_rfc3339(&found.checked_at).unwrap();
    assert_eq!(std::fs::read_to_string(&config).unwrap(), contents);
}

#[test]
fn invalid_missing_shape_or_oversized_mcp_files_remain_unverifiable_and_other_agents_are_not_checked(
) {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("mcp.json");
    let required = vec!["jira-mcp".into()];
    for contents in [
        "not JSON",
        r#"{}"#,
        r#"{"mcpServers":[]}"#,
        r#"{"mcpServers":null}"#,
    ] {
        std::fs::write(&config, contents).unwrap();
        let report = inspect_mcp(&config, Some(&required));
        assert_eq!(report.state, McpState::Unverifiable);
        assert!(report.missing.is_empty());
        assert_eq!(std::fs::read_to_string(&config).unwrap(), contents);
    }
    std::fs::File::create(&config)
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        inspect_mcp(&config, Some(&required)).state,
        McpState::Unverifiable
    );
    assert_eq!(
        crate::sds::mcp::inspect_agent_mcp("codex", &config, Some(&[])).state,
        McpState::Unverifiable
    );
    assert_eq!(
        inspect_mcp(&config, Some(&[String::new()])).state,
        McpState::Unknown
    );
}

#[test]
#[cfg(windows)]
fn mcp_inspection_refuses_junction_ancestors_and_never_exports_their_contents() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let contents = r#"{"mcpServers":{"jira-mcp":{"token":"synthetic-token"}}}"#;
    std::fs::write(external.path().join("mcp.json"), contents).unwrap();
    junction::create(external.path(), root.path().join(".aipro")).unwrap();
    let report = inspect_mcp(
        &root.path().join(".aipro/mcp.json"),
        Some(&["jira-mcp".into()]),
    );
    assert_eq!(report.state, McpState::Unverifiable);
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("synthetic-"));
    assert_eq!(
        std::fs::read_to_string(external.path().join("mcp.json")).unwrap(),
        contents
    );
}
