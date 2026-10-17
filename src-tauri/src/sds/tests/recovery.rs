use crate::sds::{
    contracts::{
        McpReport, McpState, OperationResult, OperationState, PreparedOperation, Release,
        SessionContext, Target,
    },
    deployment::fingerprint_tree,
    journal::{commit_install, recover_pending, CommitIntent},
    packages::VerifiedArtifact,
    repository::Repository,
};
use std::path::Path;

fn prepared(root: &Path) -> (Repository, PreparedOperation, VerifiedArtifact) {
    prepared_named(root, "agent-skills")
}
fn prepared_named(
    root: &Path,
    root_name: &str,
) -> (Repository, PreparedOperation, VerifiedArtifact) {
    let source = root.join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), b"# Synthetic recovery test").unwrap();
    let (manifest, fingerprint) = fingerprint_tree(&source).unwrap();
    let artifact = VerifiedArtifact {
        root: source,
        manifest,
        fingerprint,
    };
    let repository = Repository::open(&root.join("sds")).unwrap();
    let release: Release = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../contracts/sds-desktop/v1/fixtures/release.json"
        ))
        .unwrap()["data"]
            .clone(),
    )
    .unwrap();
    let target_root = root.join(root_name);
    std::fs::create_dir(&target_root).unwrap();
    let plan = PreparedOperation {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "install".into(),
        target: Target {
            agent_key: "aipro".into(),
            workspace_id: "global".into(),
            path: target_root.join(&release.install_name),
        },
        reviewed_root: crate::sds::contracts::ReviewedRoot::capture(&target_root).unwrap(),
        release,
        expected_fingerprint: None,
        mcp: McpReport {
            state: McpState::NotRequired,
            missing: vec![],
            reason: None,
            checked_at: chrono::Utc::now().to_rfc3339(),
        },
        requires_mcp_ack: false,
        can_execute: true,
        conflict: None,
        context: SessionContext {
            profile_id: "synthetic-store".into(),
            account_id: "1001".into(),
            generation: 1,
        },
        runtime_epoch: "synthetic-runtime".into(),
    };
    repository
        .create_operation(
            &plan,
            &OperationResult {
                operation_id: plan.id.clone(),
                state: OperationState::Prepared,
                stage: "review".into(),
                error: None,
            },
        )
        .unwrap();
    repository
        .cache_release(&plan.context.profile_id, &plan.release, &artifact)
        .unwrap();
    (repository, plan, artifact)
}

#[test]
fn a_durable_install_intent_commits_the_tree_and_its_library_deployment_together() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    assert!(!plan.target.path.exists());
    commit_install(&repository, &plan, &intent).unwrap();
    assert_eq!(
        std::fs::read(plan.target.path.join("SKILL.md")).unwrap(),
        b"# Synthetic recovery test"
    );
    assert_eq!(
        repository.operation_status(&plan.id).unwrap().state,
        OperationState::Completed
    );
    assert_eq!(repository.deployments().unwrap().len(), 1);
    let invalid_failure = OperationResult {
        operation_id: plan.id.clone(),
        state: OperationState::Failed,
        stage: "late-failure".into(),
        error: None,
    };
    assert!(repository.fail_intent(&invalid_failure).is_err());
    assert_eq!(
        repository.operation_status(&plan.id).unwrap().state,
        OperationState::Completed
    );
    drop(repository);
    let repository = Repository::open(&root.path().join("sds")).unwrap();
    assert!(recover_pending(
        &repository,
        &[plan.target.path.parent().unwrap().to_path_buf()]
    )
    .unwrap()
    .is_empty());
    assert_eq!(repository.deployments().unwrap().len(), 1);
}

#[test]
fn recovery_preserves_identical_unmanaged_targets_and_changed_parent_directories() {
    for replace_parent in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (repository, plan, artifact) = prepared(root.path());
        let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
        if replace_parent {
            std::fs::rename(&intent.target_root, root.path().join("original-root")).unwrap();
            std::fs::create_dir(&intent.target_root).unwrap();
            std::fs::write(intent.target_root.join("new-user-file"), b"keep").unwrap();
            assert_eq!(
                commit_install(&repository, &plan, &intent)
                    .unwrap_err()
                    .code,
                "REVIEW_REQUIRED"
            );
        } else {
            std::fs::remove_dir_all(intent.stage_path()).unwrap();
            std::fs::create_dir(&plan.target.path).unwrap();
            std::fs::write(
                plan.target.path.join("SKILL.md"),
                b"# Synthetic recovery test",
            )
            .unwrap();
        }
        let recovered = recover_pending(&repository, &[intent.target_root.clone()]).unwrap();
        assert_eq!(recovered[0].state, OperationState::RecoveryRequired);
        assert!(repository.deployments().unwrap().is_empty());
        if replace_parent {
            assert_eq!(
                std::fs::read(intent.target_root.join("new-user-file")).unwrap(),
                b"keep"
            );
        } else {
            assert_eq!(
                std::fs::read(plan.target.path.join("SKILL.md")).unwrap(),
                b"# Synthetic recovery test"
            );
        }
    }
}

#[test]
fn commit_and_recovery_keep_local_files_created_after_review() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    std::fs::create_dir(&plan.target.path).unwrap();
    std::fs::write(plan.target.path.join("local.txt"), b"keep local").unwrap();
    assert_eq!(
        commit_install(&repository, &plan, &intent)
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
    assert_eq!(
        recover_pending(&repository, &[intent.target_root]).unwrap()[0].state,
        OperationState::RecoveryRequired
    );
    assert_eq!(
        std::fs::read(plan.target.path.join("local.txt")).unwrap(),
        b"keep local"
    );
    assert!(repository.deployments().unwrap().is_empty());
}

#[test]
fn staging_rejects_changed_cached_release_metadata_before_creating_a_stage() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let mut changed = plan.release.clone();
    changed.required_mcps = Some(vec!["new-prerequisite".into()]);
    let connection = rusqlite::Connection::open(repository.root().join("pilot.sqlite3")).unwrap();
    connection
        .execute(
            "UPDATE releases SET release_json=?1",
            [serde_json::to_string(&changed).unwrap()],
        )
        .unwrap();
    drop(connection);
    assert_eq!(
        CommitIntent::stage_install(&repository, &plan, &artifact)
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
    assert!(!plan
        .target
        .path
        .parent()
        .unwrap()
        .join(format!(".sds-stage-{}", plan.id))
        .exists());
    assert!(repository.pending_intents().unwrap().is_empty());
}

#[test]
fn a_destination_root_replaced_after_review_is_refused_before_staging() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let target_root = plan.target.path.parent().unwrap();
    std::fs::rename(target_root, root.path().join("original-agent-skills")).unwrap();
    std::fs::create_dir(target_root).unwrap();
    std::fs::write(target_root.join("user-file"), b"keep").unwrap();
    assert_eq!(
        CommitIntent::stage_install(&repository, &plan, &artifact)
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
    assert_eq!(std::fs::read_dir(target_root).unwrap().count(), 1);
    assert!(repository.pending_intents().unwrap().is_empty());
}

#[test]
fn uncommitted_stages_stay_outside_the_agent_skill_discovery_root() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    assert!(!intent.stage_path().starts_with(&intent.target_root));
    assert!(intent.stage_path().join("SKILL.md").is_file());
    assert_eq!(std::fs::read_dir(&intent.target_root).unwrap().count(), 0);
    assert_eq!(
        recover_pending(&repository, &[intent.target_root.clone()]).unwrap()[0].state,
        OperationState::Failed
    );
    assert!(intent.stage_path().is_dir());
    assert_eq!(std::fs::read_dir(&intent.target_root).unwrap().count(), 0);
}

#[test]
fn staging_remains_a_sibling_when_the_custom_target_uses_the_reserved_staging_name() {
    for name in [".sds-skills-manager-staging", ".SDS-SKILLS-MANAGER-STAGING"] {
        let root = tempfile::tempdir().unwrap();
        let (repository, plan, artifact) = prepared_named(root.path(), name);
        let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
        assert!(!intent.stage_path().starts_with(&intent.target_root));
        assert_eq!(std::fs::read_dir(&intent.target_root).unwrap().count(), 0);
    }
}

#[test]
fn a_second_staged_install_cannot_recreate_a_target_with_a_now_missing_deployment() {
    let root = tempfile::tempdir().unwrap();
    let (repository, first, artifact) = prepared(root.path());
    let mut second = first.clone();
    second.id = uuid::Uuid::new_v4().to_string();
    repository
        .create_operation(
            &second,
            &OperationResult {
                operation_id: second.id.clone(),
                state: OperationState::Prepared,
                stage: "review".into(),
                error: None,
            },
        )
        .unwrap();
    let first_intent = CommitIntent::stage_install(&repository, &first, &artifact).unwrap();
    let second_intent = CommitIntent::stage_install(&repository, &second, &artifact).unwrap();
    commit_install(&repository, &first, &first_intent).unwrap();
    assert!(first.target.path.starts_with(root.path()));
    std::fs::remove_dir_all(&first.target.path).unwrap();
    assert_eq!(
        commit_install(&repository, &second, &second_intent)
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
    assert!(!first.target.path.exists());
    assert!(second_intent.stage_path().is_dir());
    assert_eq!(repository.deployments().unwrap().len(), 1);
}

#[cfg(windows)]
#[test]
fn a_prepared_commit_rechecks_late_content_changes_before_rename() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    let ready =
        crate::sds::journal::PreparedInstallCommit::prepare(&repository, &plan, &intent).unwrap();
    std::fs::write(
        intent.stage_path().join("SKILL.md"),
        b"changed after validation",
    )
    .unwrap();
    std::fs::write(intent.stage_path().join("new-user-file"), b"keep").unwrap();
    assert_eq!(
        ready.publish(&repository).unwrap_err().code,
        "REVIEW_REQUIRED"
    );
    assert!(!plan.target.path.exists());
    assert_eq!(
        std::fs::read(intent.stage_path().join("new-user-file")).unwrap(),
        b"keep"
    );
    assert!(repository.deployments().unwrap().is_empty());
}

#[test]
fn recovery_does_not_approve_a_distinct_root_that_only_unicode_casefolds_equally() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    let alternate = root.path().join("AGENT-SKILLſ");
    std::fs::create_dir(&alternate).unwrap();
    assert_eq!(
        recover_pending(&repository, &[alternate]).unwrap()[0].state,
        OperationState::RecoveryRequired
    );
    assert!(intent.stage_path().is_dir());
    assert_eq!(repository.pending_intents().unwrap().len(), 1);
}

#[test]
fn oversized_durable_intents_are_reported_instead_of_disappearing_from_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    let connection = rusqlite::Connection::open(repository.root().join("pilot.sqlite3")).unwrap();
    connection
        .execute("UPDATE operations SET intent_json=?1", [" ".repeat(65_537)])
        .unwrap();
    drop(connection);
    assert!(repository.pending_intents().is_err());
    assert!(!plan.target.path.exists());
}

#[test]
fn a_database_write_failure_after_rename_keeps_the_intent_and_never_reports_success() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    let connection = rusqlite::Connection::open(repository.root().join("pilot.sqlite3")).unwrap();
    connection.execute_batch("CREATE TRIGGER synthetic_deployment_failure BEFORE INSERT ON deployments BEGIN SELECT RAISE(ABORT, 'synthetic write failure'); END;").unwrap();
    assert!(commit_install(&repository, &plan, &intent).is_err());
    assert_eq!(
        repository.operation_status(&plan.id).unwrap().state,
        OperationState::Committing
    );
    assert!(repository.deployments().unwrap().is_empty());
    assert_eq!(repository.pending_intents().unwrap().len(), 1);
    assert_eq!(
        fingerprint_tree(&plan.target.path).unwrap().1,
        artifact.fingerprint
    );
    connection
        .execute_batch("DROP TRIGGER synthetic_deployment_failure;")
        .unwrap();
    drop(connection);
    drop(repository);
    let repository = Repository::open(&root.path().join("sds")).unwrap();
    assert_eq!(
        recover_pending(&repository, &[intent.target_root]).unwrap()[0].state,
        OperationState::Completed
    );
    assert_eq!(repository.deployments().unwrap().len(), 1);
}

#[cfg(windows)]
#[test]
fn an_open_repository_pins_its_directory_until_the_sqlite_connection_closes() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("sds");
    let repository = Repository::open(&data).unwrap();
    assert!(std::fs::rename(&data, root.path().join("moved")).is_err());
    drop(repository);
    std::fs::rename(&data, root.path().join("moved")).unwrap();
}

#[test]
fn a_real_process_kill_at_each_commit_boundary_recovers_without_partial_library_success() {
    use std::process::{Command, Stdio};
    for boundary in ["before-rename", "after-rename", "after-db"] {
        let root = tempfile::tempdir().unwrap();
        let (repository, plan, artifact) = prepared(root.path());
        let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
        drop(repository);
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sds::tests::recovery::commit_crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("SDS_CRASH_CHILD_ROOT", root.path())
                .env("SDS_CRASH_CHILD_ID", &plan.id)
                .env("SDS_CRASH_CHILD_BOUNDARY", boundary)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !root.path().join("child-ready").exists() {
            if let Some(code) = child.try_wait().unwrap() {
                panic!("Owned crash child exited before its boundary: {code}");
            }
            if std::time::Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Owned crash child did not reach its boundary");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let busy_repository = Repository::open(&root.path().join("sds")).unwrap();
        if boundary == "before-rename" {
            assert_eq!(
                commit_install(&busy_repository, &plan, &intent)
                    .unwrap_err()
                    .code,
                "OPERATION_BUSY"
            );
        }
        drop(busy_repository);
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let repository = Repository::open(&root.path().join("sds")).unwrap();
        let recovered = recover_pending(&repository, &[intent.target_root.clone()]).unwrap();
        if boundary == "before-rename" {
            assert!(!plan.target.path.exists());
            assert!(intent.stage_path().is_dir());
            assert_eq!(recovered[0].state, OperationState::Failed);
            assert!(repository.deployments().unwrap().is_empty());
        } else {
            assert_eq!(
                std::fs::read(plan.target.path.join("SKILL.md")).unwrap(),
                b"# Synthetic recovery test"
            );
            assert_eq!(
                repository.operation_status(&plan.id).unwrap().state,
                OperationState::Completed
            );
            assert_eq!(repository.deployments().unwrap().len(), 1);
            assert!(repository.pending_intents().unwrap().is_empty());
        }
    }
}

#[test]
#[ignore = "Invoked only by the owned crash-boundary parent test with isolated temporary roots"]
fn commit_crash_child() {
    use crate::sds::journal::{commit_install_test_hook, CommitBoundary};
    let root = std::path::PathBuf::from(
        std::env::var_os("SDS_CRASH_CHILD_ROOT")
            .expect("Only the crash parent invokes this helper"),
    );
    let id = std::env::var("SDS_CRASH_CHILD_ID").unwrap();
    let boundary = std::env::var("SDS_CRASH_CHILD_BOUNDARY").unwrap();
    let repository = Repository::open(&root.join("sds")).unwrap();
    let plan = repository.load_operation(&id).unwrap();
    let (_, intent) = repository
        .pending_intents()
        .unwrap()
        .into_iter()
        .find(|(plan, _)| plan.id == id)
        .unwrap();
    commit_install_test_hook(&repository, &plan, &intent, |at| {
        let matches = matches!(
            (at, boundary.as_str()),
            (CommitBoundary::BeforeRename, "before-rename")
                | (CommitBoundary::AfterRename, "after-rename")
                | (CommitBoundary::AfterDatabase, "after-db")
        );
        if matches {
            let file = std::fs::File::create(root.join("child-ready")).unwrap();
            file.sync_all().unwrap();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Ok(())
    })
    .unwrap();
}

struct OwnedChild(std::process::Child);
impl std::ops::Deref for OwnedChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for OwnedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
