use crate::sds::{
    deployment::fingerprint_tree,
    journal::{commit_install, CommitIntent},
    snapshots::{SnapshotReason, SnapshotService},
};
use std::sync::Arc;
#[test]
fn snapshot_preserves_added_changed_deleted_files_and_original_baseline() {
    let t = tempfile::tempdir().unwrap();
    let (repo, plan, artifact) = super::recovery::prepared(t.path());
    let repo = Arc::new(repo);
    let intent = CommitIntent::stage_install(&repo, &plan, &artifact).unwrap();
    commit_install(&repo, &plan, &intent).unwrap();
    std::fs::remove_file(plan.target.path.join("SKILL.md")).unwrap();
    std::fs::write(plan.target.path.join("added.txt"), "local edits").unwrap();
    let current = fingerprint_tree(&plan.target.path).unwrap().1;
    let service = SnapshotService::new(repo.clone());
    let snapshot = service
        .capture(&plan.id, &current, SnapshotReason::Manual)
        .unwrap();
    assert_eq!(snapshot.content_fingerprint, current);
    assert_eq!(snapshot.baseline_fingerprint, artifact.fingerprint);
    assert_eq!(
        service
            .verify(&snapshot.id)
            .unwrap()
            .content_manifest
            .entries
            .len(),
        1
    );
    assert!(!service
        .artifact(&snapshot.id)
        .unwrap()
        .root
        .join("SKILL.md")
        .exists());
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, current);
    std::fs::write(
        service
            .artifact(&snapshot.id)
            .unwrap()
            .root
            .join("added.txt"),
        "corrupt",
    )
    .unwrap();
    assert_eq!(
        service.verify(&snapshot.id).unwrap_err().code,
        "SNAPSHOT_CORRUPT"
    );
}
#[test]
fn changed_source_review_does_not_publish_snapshot() {
    let t = tempfile::tempdir().unwrap();
    let (repo, plan, artifact) = super::recovery::prepared(t.path());
    let repo = Arc::new(repo);
    let intent = CommitIntent::stage_install(&repo, &plan, &artifact).unwrap();
    commit_install(&repo, &plan, &intent).unwrap();
    let service = SnapshotService::new(repo);
    std::fs::write(plan.target.path.join("SKILL.md"), "changed").unwrap();
    assert!(service
        .capture(&plan.id, &artifact.fingerprint, SnapshotReason::Manual)
        .is_err());
    assert!(service.list().unwrap().is_empty());
}
