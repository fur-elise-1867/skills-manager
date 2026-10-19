use crate::sds::{
    cache_cleanup::{self, CacheBoundary},
    contracts::{OperationResult, OperationState, PreparedOperation},
    deployment::fingerprint_tree,
    error::SdsError,
    journal::{commit_install, CommitIntent},
    lifecycle::LifecycleOptions,
    removal, replacement,
    repository::Repository,
    snapshots::{SnapshotReason, SnapshotService},
};
use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

fn prepared_result(plan: &PreparedOperation) -> OperationResult {
    OperationResult {
        operation_id: plan.id.clone(),
        state: OperationState::Prepared,
        stage: "review".into(),
        error: None,
    }
}

// Catch publication after cleanup's reference check, rather than merely testing
// that an earlier reservation is visible to that check. The second connection
// is opened before the hook; SQLite startup is not part of the contested action.
fn pending_publication_during_cleanup(boundary: CacheBoundary) {
    let root = tempfile::tempdir().unwrap();
    let (repository, mut pending, _) = super::recovery::prepared(root.path());
    repository.expire_precommit("next-runtime").unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
    pending.id = uuid::Uuid::new_v4().to_string();
    let initial = prepared_result(&pending);
    let mut contender = Some(Repository::open(repository.root()).unwrap());
    let mut worker = None;
    let mut publication = None;
    let deletion =
        cache_cleanup::execute_test_hook(&repository, "cleanup-runtime", &delete.id, |at| {
            if at != boundary {
                return Ok(());
            }
            let other = contender.take().expect("selected boundary occurs once");
            let plan = pending.clone();
            let result = initial.clone();
            let (send, receive) = mpsc::channel();
            worker = Some(thread::spawn(move || {
                let outcome = other.create_operation(&plan, &result);
                send.send(outcome.clone()).unwrap();
                outcome
            }));
            // Cleanup remains inside the hook, owning cache.lock throughout
            // the contender's attempt. No scheduler sleep establishes ordering.
            publication = Some(receive.recv_timeout(Duration::from_secs(5)).map_err(|_| {
                SdsError::new(
                    "TEST_PUBLICATION_TIMEOUT",
                    "Publication did not return while cleanup held its lock",
                    false,
                )
            })?);
            Ok(())
        });
    // Join after execute returns, so a wrongly blocking implementation also
    // releases the lock and fails boundedly instead of deadlocking this test.
    worker
        .expect("cleanup reached the selected boundary")
        .join()
        .unwrap()
        .ok();
    let publication = publication.expect("publication completed at the boundary");
    assert!(
        publication.is_err(),
        "pending reference was published after cleanup's reference check at {boundary:?}; deletion={deletion:?}"
    );
    assert_eq!(publication.unwrap_err().code, "OPERATION_BUSY");
    assert_eq!(deletion.unwrap().state, OperationState::Completed);
    assert!(repository.load_operation(&pending.id).is_err());
    assert_eq!(repository.list_library().unwrap().len(), 1);
    assert!(!repository.root().join("releases").join(&key).exists());
    assert!(!repository
        .root()
        .join("releases")
        .join(format!(".deleted-{}", delete.id))
        .exists());
}

#[test]
fn pending_reference_cannot_publish_after_cleanup_intent() {
    pending_publication_during_cleanup(CacheBoundary::AfterIntent);
}

#[test]
fn pending_reference_cannot_publish_after_cache_rename() {
    pending_publication_during_cleanup(CacheBoundary::AfterRename);
}

#[test]
fn committed_reference_publication_obeys_the_same_cache_lock() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = super::recovery::prepared(root.path());
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    let cached = repository
        .cached_release(
            &plan.context.profile_id,
            &plan.release.skill_id,
            &plan.release.id,
        )
        .unwrap()
        .unwrap();
    let other = Repository::open(repository.root()).unwrap();
    let cleanup_lock = repository.cache_lock().unwrap();
    let contender_plan = plan.clone();
    let contender_intent = intent.clone();
    let (send, receive) = mpsc::channel();
    let worker = thread::spawn(move || {
        let result = other.finish_deployment_verified(&contender_plan, &contender_intent, &cached);
        send.send(result.clone()).unwrap();
        result
    });
    let result = receive.recv_timeout(Duration::from_secs(5));
    drop(cleanup_lock);
    worker.join().unwrap().ok();
    let result =
        result.expect("publication must fail promptly while cache.lock is owned elsewhere");
    assert!(
        result.is_err(),
        "committed reference publication bypassed cache.lock"
    );
    assert_eq!(result.unwrap_err().code, "OPERATION_BUSY");
    assert!(repository.deployments().unwrap().is_empty());
    assert_eq!(
        repository.operation_status(&plan.id).unwrap().state,
        OperationState::Committing
    );
    // A rejected publication must leave its durable intent usable after retry.
    commit_install(&repository, &plan, &intent).unwrap();
    assert_eq!(repository.deployments().unwrap().len(), 1);
    assert_eq!(
        fingerprint_tree(&plan.target.path).unwrap().1,
        artifact.fingerprint
    );
}

#[test]
fn prepared_cache_publication_keeps_its_existing_lock_without_reentering_it() {
    for fresh_release in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (repository, plan, artifact) = super::recovery::prepared(root.path());
        repository.expire_precommit("next-runtime").unwrap();
        let key = repository.list_library().unwrap()[0].id.clone();
        let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
        let other = Repository::open(repository.root()).unwrap();
        let mut release = plan.release.clone();
        if fresh_release {
            release.id = "fresh-cache-release".into();
            release.sequence += 1;
        }
        let prepared = other
            .prepare_cache(&plan.context.profile_id, &release, &artifact)
            .unwrap();
        assert_eq!(
            cache_cleanup::execute(&repository, "cleanup-runtime", &delete.id)
                .unwrap_err()
                .code,
            "OPERATION_BUSY"
        );
        // PreparedCache already owns the lock. Adding another acquisition to publish
        // would reject its own guard (or deadlock if changed to a blocking lock).
        let cached = prepared.publish(&other).unwrap();
        assert_eq!(cached.artifact.fingerprint, artifact.fingerprint);
        assert!(cached.artifact.root.join("SKILL.md").is_file());
        assert_eq!(
            cache_cleanup::execute(&repository, "cleanup-runtime", &delete.id)
                .unwrap()
                .state,
            OperationState::Completed
        );
        if fresh_release {
            assert!(cached.artifact.root.join("SKILL.md").is_file());
            assert_eq!(repository.list_library().unwrap().len(), 2);
        } else {
            assert_eq!(repository.list_library().unwrap().len(), 1);
        }
    }
}

#[test]
fn cache_publication_is_refused_while_cleanup_owns_the_lock() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = super::recovery::prepared(root.path());
    repository.expire_precommit("next-runtime").unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
    let mut other = Some(Repository::open(repository.root()).unwrap());
    let mut attempted = false;
    let result =
        cache_cleanup::execute_test_hook(&repository, "cleanup-runtime", &delete.id, |at| {
            if at == CacheBoundary::AfterRename {
                attempted = true;
                let other = other.take().unwrap();
                let result = thread::scope(|scope| {
                    scope
                        .spawn(|| {
                            other.cache_release(&plan.context.profile_id, &plan.release, &artifact)
                        })
                        .join()
                        .unwrap()
                });
                assert_eq!(result.unwrap_err().code, "OPERATION_BUSY");
            }
            Ok(())
        })
        .unwrap();
    assert!(attempted);
    assert_eq!(result.state, OperationState::Completed);
    assert_eq!(repository.list_library().unwrap().len(), 1);
}

#[test]
fn snapshot_restore_can_publish_a_deployment_after_original_cache_deletion() {
    let root = tempfile::tempdir().unwrap();
    let (repository, installed, artifact) = super::recovery::prepared(root.path());
    let repository = Arc::new(repository);
    let intent = CommitIntent::stage_install(&repository, &installed, &artifact).unwrap();
    commit_install(&repository, &installed, &intent).unwrap();
    let snapshots = SnapshotService::new(repository.clone());
    let snapshot = snapshots
        .capture(&installed.id, &artifact.fingerprint, SnapshotReason::Manual)
        .unwrap();
    let remove = removal::prepare(&repository, "remove-runtime", &installed.id).unwrap();
    let intent = removal::stage_remove(&repository, &remove).unwrap();
    removal::commit_remove(&repository, &remove, &intent).unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
    cache_cleanup::execute(&repository, "cleanup-runtime", &delete.id).unwrap();
    assert!(!repository.root().join("releases").join(&key).exists());

    let mut restore = installed.clone();
    restore.id = uuid::Uuid::new_v4().to_string();
    restore.kind = "restore".into();
    restore.deployment_id = Some(installed.id.clone());
    restore.runtime_epoch = "restore-runtime".into();
    repository
        .create_operation(&restore, &prepared_result(&restore))
        .unwrap();
    repository
        .save_options(
            &restore.id,
            &LifecycleOptions {
                snapshot_id: Some(snapshot.id.clone()),
                ..Default::default()
            },
        )
        .unwrap();
    let source = snapshots.artifact(&snapshot.id).unwrap();
    let intent = replacement::stage(&repository, &restore, &source).unwrap();
    replacement::commit(&repository, &restore, &intent).unwrap();
    assert_eq!(
        repository.operation_status(&restore.id).unwrap().state,
        OperationState::Completed
    );
    let deployments = repository.deployments().unwrap();
    assert_eq!(deployments.len(), 1);
    assert_eq!(deployments[0].id, installed.id);
    assert_eq!(
        fingerprint_tree(&deployments[0].target.path).unwrap().1,
        artifact.fingerprint
    );
    assert!(!repository.root().join("releases").join(&key).exists());
    snapshots.verify(&snapshot.id).unwrap();
}

#[test]
fn pending_publication_reuses_its_handles_lease_but_cleanup_cannot_borrow_it() {
    let root = tempfile::tempdir().unwrap();
    let (repository, mut pending, _) = super::recovery::prepared(root.path());
    repository.expire_precommit("next-runtime").unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
    let lease = repository.cache_lock().unwrap();
    pending.id = uuid::Uuid::new_v4().to_string();
    // The cached-install caller already holds this lease across its session
    // guard. The nested reference write must not acquire the file lock again.
    repository
        .create_operation(&pending, &prepared_result(&pending))
        .unwrap();
    assert_eq!(
        cache_cleanup::execute(&repository, "cleanup-runtime", &delete.id)
            .unwrap_err()
            .code,
        "OPERATION_BUSY"
    );
    drop(lease);
    assert_eq!(
        cache_cleanup::execute(&repository, "cleanup-runtime", &delete.id)
            .unwrap_err()
            .code,
        "RELEASE_IN_USE"
    );
    assert!(repository
        .root()
        .join("releases")
        .join(&key)
        .join("SKILL.md")
        .is_file());
}

#[test]
fn interrupted_cache_rename_blocks_new_references_until_recovery() {
    let root = tempfile::tempdir().unwrap();
    let (repository, mut pending, artifact) = super::recovery::prepared(root.path());
    repository.expire_precommit("next-runtime").unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
    assert!(
        cache_cleanup::execute_test_hook(&repository, "cleanup-runtime", &delete.id, |at| {
            if at == CacheBoundary::AfterRename {
                Err(SdsError::invalid("interrupted after cache rename"))
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    let other = Repository::open(repository.root()).unwrap();
    pending.id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        other
            .create_operation(&pending, &prepared_result(&pending))
            .unwrap_err()
            .code,
        "OPERATION_BUSY"
    );
    assert_eq!(
        other
            .cache_release(&pending.context.profile_id, &pending.release, &artifact)
            .unwrap_err()
            .code,
        "OPERATION_BUSY"
    );
    assert!(other.load_operation(&pending.id).is_err());
    cache_cleanup::recover_pending(&repository).unwrap();
    assert_eq!(repository.list_library().unwrap().len(), 1);
    assert!(!repository
        .root()
        .join("releases")
        .join(format!(".deleted-{}", delete.id))
        .exists());
    // Online preparation can reserve a release before downloading it again.
    other
        .create_operation(&pending, &prepared_result(&pending))
        .unwrap();
}

#[test]
fn committed_cache_deletion_can_clean_old_tombstone_after_republication() {
    let root = tempfile::tempdir().unwrap();
    let (repository, mut pending, artifact) = super::recovery::prepared(root.path());
    repository.expire_precommit("next-runtime").unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repository, "cleanup-runtime", &key).unwrap();
    assert!(
        cache_cleanup::execute_test_hook(&repository, "cleanup-runtime", &delete.id, |at| {
            if at == CacheBoundary::AfterDatabase {
                Err(SdsError::invalid("interrupted after cache deletion commit"))
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    let other = Repository::open(repository.root()).unwrap();
    other
        .cache_release(&pending.context.profile_id, &pending.release, &artifact)
        .unwrap();
    pending.id = uuid::Uuid::new_v4().to_string();
    other
        .create_operation(&pending, &prepared_result(&pending))
        .unwrap();
    cache_cleanup::recover_pending(&repository).unwrap();
    assert!(repository
        .root()
        .join("releases")
        .join(&key)
        .join("SKILL.md")
        .is_file());
    assert_eq!(
        repository.operation_status(&pending.id).unwrap().state,
        OperationState::Prepared
    );
    let (completed,): (u32,) = repository
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT committed FROM cache_cleanup WHERE id=?1",
            [&delete.id],
            |row| Ok((row.get(0)?,)),
        )
        .unwrap();
    assert_eq!(completed, 2);
    assert!(!repository
        .root()
        .join("releases")
        .join(format!(".deleted-{}", delete.id))
        .exists());
}

#[test]
fn durable_install_intent_keeps_cache_reserved_even_with_terminal_progress() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan, artifact) = super::recovery::prepared(root.path());
    CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    repository
        .record_result(&OperationResult {
            operation_id: plan.id,
            state: OperationState::Failed,
            stage: "interrupted-result".into(),
            error: None,
        })
        .unwrap();
    let key = repository.list_library().unwrap()[0].id.clone();
    assert_eq!(
        cache_cleanup::prepare(&repository, "cleanup-runtime", &key)
            .unwrap_err()
            .code,
        "RELEASE_IN_USE"
    );
    assert!(repository
        .root()
        .join("releases")
        .join(&key)
        .join("SKILL.md")
        .is_file());
}
