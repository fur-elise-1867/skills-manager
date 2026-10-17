use crate::sds::{
    contracts::{DeploymentStatus, OperationResult, OperationState},
    deployment::fingerprint_tree,
    journal::{commit_install, CommitIntent},
    lifecycle::LifecycleOptions,
    packages::VerifiedArtifact,
    replacement::{self, ReplacementBoundary as B},
    snapshots::{SnapshotReason, SnapshotService},
};
use std::sync::Arc;
pub(super) fn fixture(
    root: &std::path::Path,
) -> (
    Arc<crate::sds::repository::Repository>,
    crate::sds::contracts::PreparedOperation,
    VerifiedArtifact,
) {
    let (repo, install, artifact) = super::recovery::prepared(root);
    let repo = Arc::new(repo);
    let intent = CommitIntent::stage_install(&repo, &install, &artifact).unwrap();
    commit_install(&repo, &install, &intent).unwrap();
    let mut release = install.release.clone();
    release.id = "release-new".into();
    release.sequence += 1;
    let source = root.join("new-content");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "# New release").unwrap();
    let (manifest, fingerprint) = fingerprint_tree(&source).unwrap();
    let new = VerifiedArtifact {
        root: source,
        manifest,
        fingerprint,
    };
    repo.cache_release(&install.context.profile_id, &release, &new)
        .unwrap();
    let mut plan = install.clone();
    plan.id = uuid::Uuid::new_v4().to_string();
    plan.kind = "update".into();
    plan.deployment_id = Some(install.id);
    plan.release = release;
    plan.expected_fingerprint = Some(artifact.fingerprint);
    plan.reviewed_root =
        crate::sds::contracts::ReviewedRoot::capture(plan.target.path.parent().unwrap()).unwrap();
    repo.create_operation(
        &plan,
        &OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Prepared,
            stage: "review".into(),
            error: None,
        },
    )
    .unwrap();
    (repo, plan, new)
}
#[test]
fn replacement_reconciles_every_crash_boundary_and_second_restart_is_idempotent() {
    for point in [
        B::AfterIntent,
        B::OldRenamed,
        B::NewRenamed,
        B::DatabaseCommitted,
        B::CleanupEntry,
        B::CleanupFinished,
    ] {
        let t = tempfile::tempdir().unwrap();
        let (repo, plan, new) = fixture(t.path());
        let old = plan.expected_fingerprint.clone().unwrap();
        let intent = replacement::stage(&repo, &plan, &new).unwrap();
        assert!(
            replacement::commit_test_hook(&repo, &plan, &intent, |b| if b == point {
                Err(crate::sds::error::SdsError::invalid("injected crash"))
            } else {
                Ok(())
            })
            .is_err()
        );
        assert!([&plan.target.path, &intent.rollback_path()]
            .iter()
            .any(|p| fingerprint_tree(p)
                .ok()
                .is_some_and(|(_, f)| f == old || f == new.fingerprint)));
        let data = repo.root().to_path_buf();
        drop(repo);
        let restarted = crate::sds::repository::Repository::open(&data).unwrap();
        let roots = vec![plan.target.path.parent().unwrap().to_path_buf()];
        replacement::recover_pending(&restarted, &roots).unwrap();
        assert_eq!(
            fingerprint_tree(&plan.target.path).unwrap().1,
            new.fingerprint,
            "{point:?}"
        );
        assert_eq!(
            restarted.operation_status(&plan.id).unwrap().state,
            OperationState::Completed,
            "{point:?}"
        );
        replacement::recover_pending(&restarted, &roots).unwrap();
        assert_eq!(
            fingerprint_tree(&plan.target.path).unwrap().1,
            new.fingerprint
        );
    }
}
#[test]
fn edited_snapshot_restore_keeps_original_baseline() {
    let t = tempfile::tempdir().unwrap();
    let (repo, mut plan, _) = fixture(t.path());
    let original = plan.expected_fingerprint.clone().unwrap();
    std::fs::write(plan.target.path.join("SKILL.md"), "# My edits").unwrap();
    let edited = fingerprint_tree(&plan.target.path).unwrap().1;
    let snapshots = SnapshotService::new(repo.clone());
    let snapshot = snapshots
        .capture(
            plan.deployment_id.as_ref().unwrap(),
            &edited,
            SnapshotReason::Manual,
        )
        .unwrap();
    plan.id = uuid::Uuid::new_v4().to_string();
    plan.kind = "restore".into();
    plan.release = snapshot.release.clone();
    plan.expected_fingerprint = Some(edited.clone());
    repo.create_operation(
        &plan,
        &OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Prepared,
            stage: "review".into(),
            error: None,
        },
    )
    .unwrap();
    repo.save_options(
        &plan.id,
        &LifecycleOptions {
            snapshot_id: Some(snapshot.id.clone()),
            protect_current: true,
            ..Default::default()
        },
    )
    .unwrap();
    let artifact = snapshots.artifact(&snapshot.id).unwrap();
    let intent = replacement::stage(&repo, &plan, &artifact).unwrap();
    replacement::commit(&repo, &plan, &intent).unwrap();
    let d = repo.deployments().unwrap().pop().unwrap();
    assert_eq!(d.fingerprint, original);
    assert_eq!(
        crate::sds::library::inspect_deployment(&d).status,
        DeploymentStatus::Modified
    );
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, edited);
}
