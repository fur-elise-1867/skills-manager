use crate::sds::contracts::InstallRequest;
#[test]
fn project_selection_survives_native_request_decoding() {
    let value = serde_json::json!({"skillId":"s","releaseId":"r","target":{"agentKey":"aipro","workspaceId":"11111111-1111-4111-8111-111111111111"}});
    let request: InstallRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap()["target"],
        value["target"]
    );
}

#[test]
fn project_global_are_independent_and_aliases_deduplicate() {
    use crate::sds::{contracts::TargetSelection, targets::TargetService};
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    std::fs::create_dir_all(home.join(".aipro")).unwrap();
    std::fs::create_dir(&project).unwrap();
    let targets = TargetService::new(&home, &temp.path().join("settings"));
    let first = targets.add_workspace(&project).unwrap();
    let second = targets.add_workspace(&project).unwrap();
    assert_eq!(first.id, second.id);
    let global = targets.resolve(&TargetSelection::default()).unwrap();
    let local = targets
        .resolve(&TargetSelection {
            agent_key: "aipro".into(),
            workspace_id: first.id.clone(),
        })
        .unwrap();
    assert_ne!(global.path, local.path);
    assert_eq!(local.path, project.join(".aipro/skills"));
    std::fs::rename(&project, temp.path().join("old-project")).unwrap();
    std::fs::create_dir(&project).unwrap();
    assert_eq!(
        targets
            .resolve(&TargetSelection {
                agent_key: "aipro".into(),
                workspace_id: first.id
            })
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
}
