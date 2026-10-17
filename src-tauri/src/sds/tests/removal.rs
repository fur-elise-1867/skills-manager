use super::operations::{package_server, request, setup};
use crate::sds::{
    commands::SdsServices,
    contracts::{OperationState, PreparedOperation},
    operations::OperationService,
    repository::Repository,
};
use std::{path::Path, sync::Arc};

pub(super) async fn installed(
    root: &Path,
) -> (
    SdsServices,
    Arc<Repository>,
    OperationService,
    PreparedOperation,
) {
    let server = package_server("zip", false).await;
    let (services, repository, operations) = setup(&server, root).await;
    let install = operations.prepare_install(request()).await.unwrap();
    assert_eq!(
        operations.execute(&install.id, true).await.unwrap().state,
        OperationState::Completed
    );
    services.session.logout().await.unwrap();
    drop(server);
    (services, repository, operations, install)
}

#[tokio::test]
async fn remove_preparation_is_local_freezes_the_deployment_and_refuses_modified_contents() {
    let root = tempfile::tempdir().unwrap();
    let (_, repository, operations, install) = installed(root.path()).await;
    let deployment = repository.deployments().unwrap().remove(0);
    let plan = operations.prepare_remove(&deployment.id).await.unwrap();
    assert_eq!(plan.kind, "remove");
    assert_eq!(plan.deployment_id.as_deref(), Some(deployment.id.as_str()));
    assert_eq!(plan.target, install.target);
    assert!(plan.can_execute);
    assert!(!plan.requires_mcp_ack);
    std::fs::write(plan.target.path.join("user-file"), b"keep").unwrap();
    let conflict = operations.prepare_remove(&deployment.id).await.unwrap();
    assert!(!conflict.can_execute);
    assert!(conflict.conflict.is_some());
    assert!(operations.execute(&plan.id, false).await.is_err());
    assert_eq!(
        std::fs::read(plan.target.path.join("user-file")).unwrap(),
        b"keep"
    );
    assert_eq!(repository.deployments().unwrap().len(), 1);
    assert!(operations.prepare_remove("../outside").await.is_err());
}

#[tokio::test]
async fn scoped_remove_keeps_library_mcp_configuration_and_another_destination() {
    let root = tempfile::tempdir().unwrap();
    let (services, repository, operations, install) = installed(root.path()).await;
    let skills = install.target.path.parent().unwrap();
    let other = skills.join("other-skill");
    std::fs::create_dir(&other).unwrap();
    std::fs::write(other.join("SKILL.md"), b"# Keep another skill").unwrap();
    let mcp = root.path().join("agent-home/.aipro/mcp.json");
    std::fs::write(&mcp, b"{\"mcpServers\":{}}").unwrap();
    let deployment = repository.deployments().unwrap().remove(0);
    let plan = operations.prepare_remove(&deployment.id).await.unwrap();
    let result = operations.execute(&plan.id, false).await.unwrap();
    assert_eq!(result.state, OperationState::Completed);
    assert!(!install.target.path.exists());
    assert_eq!(
        std::fs::read(other.join("SKILL.md")).unwrap(),
        b"# Keep another skill"
    );
    assert_eq!(std::fs::read(&mcp).unwrap(), b"{\"mcpServers\":{}}");
    assert!(repository.deployments().unwrap().is_empty());
    assert_eq!(repository.list_library().unwrap().len(), 1);
    assert!(repository.pending_intents().unwrap().is_empty());
    assert!(operations.library().await.unwrap()[0]
        .deployments
        .is_empty());
    assert_eq!(
        operations.execute(&plan.id, false).await.unwrap().state,
        OperationState::Completed
    );
    drop(operations);
    let restarted = OperationService::new(
        services.session.clone(),
        services.targets.clone(),
        repository.clone(),
    );
    assert_eq!(
        restarted.status(&plan.id).unwrap().state,
        OperationState::Completed
    );
}

#[tokio::test]
async fn a_missing_target_can_retire_only_its_record_and_cancel_preserves_a_healthy_target() {
    let root = tempfile::tempdir().unwrap();
    let (_, repository, operations, install) = installed(root.path()).await;
    let deployment = repository.deployments().unwrap().remove(0);
    let cancelled = operations.prepare_remove(&deployment.id).await.unwrap();
    assert_eq!(
        operations.cancel(&cancelled.id).unwrap().state,
        OperationState::Cancelled
    );
    assert_eq!(
        operations
            .execute(&cancelled.id, false)
            .await
            .unwrap()
            .state,
        OperationState::Cancelled
    );
    assert!(install.target.path.join("SKILL.md").is_file());
    let saved = root.path().join("moved-by-user");
    std::fs::rename(&install.target.path, &saved).unwrap();
    let missing = operations.prepare_remove(&deployment.id).await.unwrap();
    assert!(missing.can_execute);
    assert!(missing.expected_fingerprint.is_none());
    assert_eq!(
        operations.execute(&missing.id, false).await.unwrap().state,
        OperationState::Completed
    );
    assert!(repository.deployments().unwrap().is_empty());
    assert!(saved.join("SKILL.md").is_file());
    assert_eq!(repository.list_library().unwrap().len(), 1);
}

#[tokio::test]
async fn new_files_in_quarantine_are_preserved_and_original_target_is_restored_before_db_removal() {
    use crate::sds::removal::{commit_remove_test_hook, stage_remove, RemoveBoundary};
    let root = tempfile::tempdir().unwrap();
    let (_, repository, operations, install) = installed(root.path()).await;
    let deployment = repository.deployments().unwrap().remove(0);
    let plan = operations.prepare_remove(&deployment.id).await.unwrap();
    let intent = stage_remove(&repository, &plan).unwrap();
    commit_remove_test_hook(&repository, &plan, &intent, |at| {
        if matches!(at, RemoveBoundary::AfterRename) {
            std::fs::write(intent.stage_path().join("user-new.txt"), b"keep").unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        repository.operation_status(&plan.id).unwrap().state,
        OperationState::Failed
    );
    assert_eq!(
        std::fs::read(install.target.path.join("user-new.txt")).unwrap(),
        b"keep"
    );
    assert_eq!(repository.deployments().unwrap().len(), 1);
}

#[tokio::test]
async fn new_files_after_db_removal_are_kept_in_quarantine_and_a_new_target_is_never_deleted() {
    use crate::sds::removal::{commit_remove_test_hook, stage_remove, RemoveBoundary};
    let root = tempfile::tempdir().unwrap();
    let (_, repository, operations, install) = installed(root.path()).await;
    let deployment = repository.deployments().unwrap().remove(0);
    let plan = operations.prepare_remove(&deployment.id).await.unwrap();
    let intent = stage_remove(&repository, &plan).unwrap();
    commit_remove_test_hook(&repository, &plan, &intent, |at| {
        if matches!(at, RemoveBoundary::AfterDatabase) {
            let in_progress = repository.operation_status(&plan.id).unwrap();
            assert_eq!(in_progress.state, OperationState::Completed);
            assert_eq!(in_progress.stage, "removal-cleanup");
            std::fs::write(intent.stage_path().join("user-new.txt"), b"keep quarantine").unwrap();
            std::fs::create_dir(&install.target.path).unwrap();
            std::fs::write(install.target.path.join("user-new.txt"), b"keep target").unwrap();
        }
        Ok(())
    })
    .unwrap();
    let result = repository.operation_status(&plan.id).unwrap();
    assert_eq!(result.state, OperationState::Completed);
    assert_eq!(result.error.unwrap().code, "CLEANUP_REVIEW_REQUIRED");
    assert_eq!(
        std::fs::read(intent.stage_path().join("user-new.txt")).unwrap(),
        b"keep quarantine"
    );
    assert_eq!(
        std::fs::read(install.target.path.join("user-new.txt")).unwrap(),
        b"keep target"
    );
    assert!(repository.deployments().unwrap().is_empty());
}

#[tokio::test]
async fn removal_recovers_real_process_termination_before_after_rename_db_and_partial_cleanup() {
    use crate::sds::{journal::recover_pending, removal::stage_remove};
    use std::process::{Command, Stdio};
    for boundary in [
        "before-rename",
        "after-rename",
        "after-db",
        "partial-cleanup",
        "after-cleanup",
    ] {
        let root = tempfile::tempdir().unwrap();
        let (_, repository, operations, install) = installed(root.path()).await;
        let deployment = repository.deployments().unwrap().remove(0);
        let plan = operations.prepare_remove(&deployment.id).await.unwrap();
        let intent = stage_remove(&repository, &plan).unwrap();
        drop(operations);
        drop(repository);
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sds::tests::removal::remove_crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("SDS_REMOVE_CHILD_ROOT", root.path())
                .env("SDS_REMOVE_CHILD_BOUNDARY", boundary)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !root.path().join("child-ready").exists() {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "child exited before {boundary}"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "child missed {boundary}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        child.0.kill().unwrap();
        assert!(!child.0.wait().unwrap().success());
        let repository = Repository::open(&root.path().join("repository")).unwrap();
        let recovered = recover_pending(&repository, &[intent.target_root.clone()]).unwrap();
        assert_eq!(recovered.len(), 1);
        if boundary == "before-rename" {
            assert_eq!(recovered[0].state, OperationState::Failed);
            assert!(install.target.path.join("SKILL.md").exists());
            assert_eq!(repository.deployments().unwrap().len(), 1);
        } else {
            assert_eq!(
                recovered[0].state,
                OperationState::Completed,
                "{boundary}: {:?}",
                recovered[0]
            );
            assert!(repository.deployments().unwrap().is_empty());
            assert!(!install.target.path.exists());
            assert!(!intent.stage_path().exists(), "{boundary}");
        }
        assert!(
            repository.pending_intents().unwrap().is_empty(),
            "{boundary}"
        );
        assert_eq!(repository.list_library().unwrap().len(), 1);
    }
}
#[test]
#[ignore = "Only the owned process-termination parent invokes this helper"]
fn remove_crash_child() {
    use crate::sds::removal::{commit_remove_test_hook, RemoveBoundary};
    let root = std::path::PathBuf::from(std::env::var_os("SDS_REMOVE_CHILD_ROOT").unwrap());
    let boundary = std::env::var("SDS_REMOVE_CHILD_BOUNDARY").unwrap();
    let repository = Repository::open(&root.join("repository")).unwrap();
    let (plan, intent) = repository.pending_intents().unwrap().remove(0);
    commit_remove_test_hook(&repository, &plan, &intent, |at| {
        let hit = matches!(
            (at, boundary.as_str()),
            (RemoveBoundary::BeforeRename, "before-rename")
                | (RemoveBoundary::AfterRename, "after-rename")
                | (RemoveBoundary::AfterDatabase, "after-db")
                | (RemoveBoundary::AfterCleanupEntry, "partial-cleanup")
                | (RemoveBoundary::AfterCleanup, "after-cleanup")
        );
        if hit {
            std::fs::File::create(root.join("child-ready"))
                .unwrap()
                .sync_all()
                .unwrap();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Ok(())
    })
    .unwrap();
}
#[tokio::test]
async fn final_cleanup_database_failure_reports_recovery_instead_of_waiting_forever() {
    let root = tempfile::tempdir().unwrap();
    let (_, repository, operations, _) = installed(root.path()).await;
    let deployment = repository.deployments().unwrap().remove(0);
    let plan = operations.prepare_remove(&deployment.id).await.unwrap();
    let connection = rusqlite::Connection::open(repository.root().join("pilot.sqlite3")).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_cleanup BEFORE UPDATE OF result_json ON operations WHEN json_extract(NEW.result_json,'$.stage')='removed' BEGIN SELECT RAISE(FAIL,'synthetic cleanup failure'); END;").unwrap();
    assert!(operations.execute(&plan.id, false).await.is_err());
    let status = operations.status(&plan.id).unwrap();
    assert_eq!(status.state, OperationState::RecoveryRequired);
    assert_eq!(status.error.unwrap().code, "RECOVERY_REQUIRED");
    assert!(repository.deployments().unwrap().is_empty());
    assert_eq!(repository.pending_intents().unwrap().len(), 1);
    connection
        .execute_batch("DROP TRIGGER fail_cleanup;")
        .unwrap();
    let recovered = crate::sds::journal::recover_pending(&repository, &[]).unwrap();
    assert_eq!(recovered[0].state, OperationState::Completed);
    assert!(repository.pending_intents().unwrap().is_empty());
    assert_eq!(repository.list_library().unwrap().len(), 1);
}

struct OwnedChild(std::process::Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn removal_sqlite_failure_keeps_a_recoverable_intent_and_cache_corruption_does_not_block_local_remove(
) {
    use crate::sds::{
        journal::recover_pending,
        removal::{commit_remove, stage_remove},
    };
    let root = tempfile::tempdir().unwrap();
    let (_, repository, operations, install) = installed(root.path()).await;
    let deployment = repository.deployments().unwrap().remove(0);
    let cached = repository
        .cached_release(
            &install.context.profile_id,
            &install.release.skill_id,
            &install.release.id,
        )
        .unwrap()
        .unwrap();
    std::fs::write(cached.artifact.root.join("SKILL.md"), b"local cache edit").unwrap();
    let plan = operations.prepare_remove(&deployment.id).await.unwrap();
    let intent = stage_remove(&repository, &plan).unwrap();
    let connection = rusqlite::Connection::open(repository.root().join("pilot.sqlite3")).unwrap();
    connection.execute_batch("CREATE TRIGGER synthetic_remove_failure BEFORE DELETE ON deployments BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(commit_remove(&repository, &plan, &intent).is_err());
    assert_eq!(
        repository.operation_status(&plan.id).unwrap().state,
        OperationState::Committing
    );
    assert_eq!(repository.deployments().unwrap().len(), 1);
    assert!(intent.stage_path().join("SKILL.md").exists());
    assert!(!install.target.path.exists());
    connection
        .execute_batch("DROP TRIGGER synthetic_remove_failure;")
        .unwrap();
    drop(connection);
    assert_eq!(
        recover_pending(&repository, &[intent.target_root.clone()]).unwrap()[0].state,
        OperationState::Completed
    );
    assert!(repository.deployments().unwrap().is_empty());
    assert_eq!(
        std::fs::read(cached.artifact.root.join("SKILL.md")).unwrap(),
        b"local cache edit"
    );
}
