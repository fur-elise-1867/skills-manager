use crate::sds::{
    contracts::{PreparedOperation, OperationResult, OperationState},
    deployment::fingerprint_tree,
    error::SdsError,
    journal::{commit_install, CommitIntent},
    packages::VerifiedArtifact,
    replacement::{self, ReplacementBoundary},
    repository::Repository,
    snapshots::{SnapshotBoundary, SnapshotReason, SnapshotService},
};
use std::{path::Path, sync::Arc};

fn installed(root: &Path) -> (Arc<Repository>, PreparedOperation) {
    let (repository, plan, artifact) = super::recovery::prepared(root);
    let repository = Arc::new(repository);
    let intent = CommitIntent::stage_install(&repository, &plan, &artifact).unwrap();
    commit_install(&repository, &plan, &intent).unwrap();
    (repository, plan)
}

#[test]
fn missing_snapshot_bytes_are_rejected_without_touching_current_edits() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan) = installed(root.path());
    let snapshots = SnapshotService::new(repository.clone());
    let before = fingerprint_tree(&plan.target.path).unwrap().1;
    let snapshot = snapshots.capture(&plan.id, &before, SnapshotReason::Manual).unwrap();
    let artifact = snapshots.artifact(&snapshot.id).unwrap();
    std::fs::remove_file(artifact.root.join("SKILL.md")).unwrap();
    std::fs::write(plan.target.path.join("SKILL.md"), "# Current edits kept").unwrap();
    let current = fingerprint_tree(&plan.target.path).unwrap().1;
    assert_eq!(snapshots.verify(&snapshot.id).unwrap_err().code, "SNAPSHOT_CORRUPT");
    assert_eq!(snapshots.artifact(&snapshot.id).unwrap_err().code, "SNAPSHOT_CORRUPT");
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, current);
}

#[test]
fn snapshot_destination_failure_preserves_target_and_publishes_no_record() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan) = installed(root.path());
    let snapshots = SnapshotService::new(repository.clone());
    let before = fingerprint_tree(&plan.target.path).unwrap().1;
    std::fs::write(repository.root().join("snapshots"), b"occupied by a file").unwrap();
    assert!(snapshots.capture(&plan.id, &before, SnapshotReason::BeforeUpdate).is_err());
    assert!(snapshots.list().unwrap().is_empty());
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, before);
}

#[test]
fn depth_limit_rejects_snapshot_without_truncating_or_removing_source() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan) = installed(root.path());
    let snapshots = SnapshotService::new(repository);
    let before = fingerprint_tree(&plan.target.path).unwrap().1;
    let mut directory = plan.target.path.clone();
    for _ in 0..33 { directory.push("d"); }
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("local.txt"), b"must survive").unwrap();
    assert!(snapshots.capture(&plan.id, &before, SnapshotReason::BeforeRemove).is_err());
    assert!(snapshots.list().unwrap().is_empty());
    assert_eq!(std::fs::read(directory.join("local.txt")).unwrap(), b"must survive");
    assert_eq!(std::fs::read(plan.target.path.join("SKILL.md")).unwrap(), b"# Synthetic recovery test");
}

#[test]
fn snapshot_ids_cannot_escape_the_owned_snapshot_directory() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan) = installed(root.path());
    let snapshots = SnapshotService::new(repository);
    for id in ["../agent-skills", "..\\agent-skills", "C:\\Windows", "not-a-uuid"] {
        assert_eq!(snapshots.verify(id).unwrap_err().code, "SNAPSHOT_CORRUPT");
    }
    assert_eq!(std::fs::read(plan.target.path.join("SKILL.md")).unwrap(), b"# Synthetic recovery test");
}

#[test]
fn source_changes_during_snapshot_copy_never_publish_a_completed_record() {
    for boundary in [SnapshotBoundary::BeforeCopy, SnapshotBoundary::AfterCopy] {
        let root = tempfile::tempdir().unwrap();
        let (repository, plan) = installed(root.path());
        let snapshots = SnapshotService::new(repository);
        let before = fingerprint_tree(&plan.target.path).unwrap().1;
        let result = snapshots.capture_test_hook(&plan.id, &before, SnapshotReason::Manual, |at| {
            if at == boundary { std::fs::write(plan.target.path.join("added.txt"), b"concurrent user content").unwrap(); }
            Ok(())
        });
        assert!(result.is_err());
        assert!(snapshots.list().unwrap().is_empty());
        assert_eq!(std::fs::read(plan.target.path.join("added.txt")).unwrap(), b"concurrent user content");
        assert_eq!(std::fs::read(plan.target.path.join("SKILL.md")).unwrap(), b"# Synthetic recovery test");
    }
}

#[test]
fn snapshot_write_fault_keeps_current_target_and_publishes_no_record() {
    let root = tempfile::tempdir().unwrap();
    let (repository, plan) = installed(root.path());
    let snapshots = SnapshotService::new(repository);
    let before = fingerprint_tree(&plan.target.path).unwrap().1;
    let result = snapshots.capture_test_hook(&plan.id, &before, SnapshotReason::BeforeRestore, |at| {
        if at == SnapshotBoundary::AfterCopy { return Err(SdsError::new("SNAPSHOT_FAILED", "Injected full snapshot destination", false)); }
        Ok(())
    });
    assert!(result.is_err());
    assert!(snapshots.list().unwrap().is_empty());
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, before);
}

fn replacement_ready(root: &Path) -> (Arc<Repository>, PreparedOperation, replacement::ReplacementIntent) {
    let (repository, original) = installed(root);
    let source = root.join("new-content");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), b"# New immutable release").unwrap();
    let (manifest, fingerprint) = fingerprint_tree(&source).unwrap();
    let artifact = VerifiedArtifact {root:source,manifest,fingerprint};
    let mut plan = original.clone();
    plan.id=uuid::Uuid::new_v4().to_string();
    plan.kind="update".into();
    plan.deployment_id=Some(original.id);
    plan.release.id="rel-004".into();
    plan.release.sequence=4;
    plan.release.version="2.0.0".into();
    plan.expected_fingerprint=Some(fingerprint_tree(&plan.target.path).unwrap().1);
    repository.cache_release(&plan.context.profile_id,&plan.release,&artifact).unwrap();
    repository.create_operation(&plan,&OperationResult {operation_id:plan.id.clone(),state:OperationState::Prepared,stage:"review".into(),error:None}).unwrap();
    let intent = replacement::stage(&repository,&plan,&artifact).unwrap();
    (repository,plan,intent)
}

#[test]
fn recovery_preserves_unmanaged_collision_and_retained_old_copy() {
    let root=tempfile::tempdir().unwrap();
    let (repository,plan,intent)=replacement_ready(root.path());
    assert!(replacement::commit_test_hook(&repository,&plan,&intent,|at| {
        if at==ReplacementBoundary::OldRenamed {return Err(SdsError::new("INJECTED","Stopped after old rename",false));} Ok(())
    }).is_err());
    assert!(!plan.target.path.exists());
    std::fs::create_dir(&plan.target.path).unwrap();
    std::fs::write(plan.target.path.join("unmanaged.txt"),b"retain unmanaged collision").unwrap();
    let current=fingerprint_tree(&plan.target.path).unwrap().1;
    let old=fingerprint_tree(&intent.rollback_path()).unwrap().1;
    for _ in 0..2 {
        replacement::recover_pending(&repository,&[plan.target.path.parent().unwrap().into()]).unwrap();
        assert_eq!(repository.operation_status(&plan.id).unwrap().state,OperationState::RecoveryRequired);
        assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1,current);
        assert_eq!(fingerprint_tree(&intent.rollback_path()).unwrap().1,old);
        assert!(intent.stage_path().join("SKILL.md").exists());
    }
}

#[test]
fn replacement_cannot_write_into_a_recreated_root_after_review() {
    let root=tempfile::tempdir().unwrap();
    let (repository,plan,intent)=replacement_ready(root.path());
    let parent=plan.target.path.parent().unwrap();
    let moved=root.path().join("moved-agent-skills");
    std::fs::rename(parent,&moved).unwrap();
    std::fs::create_dir(parent).unwrap();
    std::fs::write(parent.join("unmanaged.txt"),b"replacement root").unwrap();
    assert!(replacement::commit(&repository,&plan,&intent).is_err());
    replacement::recover_pending(&repository,&[parent.into()]).unwrap();
    assert_eq!(repository.operation_status(&plan.id).unwrap().state,OperationState::RecoveryRequired);
    assert_eq!(std::fs::read(parent.join("unmanaged.txt")).unwrap(),b"replacement root");
    assert_eq!(std::fs::read(moved.join("jira-weekly-report/SKILL.md")).unwrap(),b"# Synthetic recovery test");
    assert!(intent.stage_path().exists());
}

#[test]
fn changed_staged_bytes_are_rejected_before_old_target_rename() {
    let root=tempfile::tempdir().unwrap();
    let (repository,plan,intent)=replacement_ready(root.path());
    let before=fingerprint_tree(&plan.target.path).unwrap().1;
    std::fs::write(intent.stage_path().join("SKILL.md"),b"tampered stage").unwrap();
    assert!(replacement::commit(&repository,&plan,&intent).is_err());
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1,before);
    assert!(!intent.rollback_path().exists());
}

#[test]
fn repeating_a_completed_replacement_keeps_its_record_and_content() {
    let root=tempfile::tempdir().unwrap();
    let (repository,plan,intent)=replacement_ready(root.path());
    replacement::commit(&repository,&plan,&intent).unwrap();
    let fingerprint=fingerprint_tree(&plan.target.path).unwrap().1;
    replacement::commit(&repository,&plan,&intent).unwrap();
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1,fingerprint);
    assert_eq!(repository.operation_status(&plan.id).unwrap().state,OperationState::Completed);
    assert_eq!(repository.deployments().unwrap().len(),1);
}

#[test]
#[ignore = "owned subprocess helper; executed by real_process_termination_recovers_each_replacement_boundary"]
fn replacement_crash_child() {
    let root=std::path::PathBuf::from(std::env::var_os("SDS_PHASE2_CRASH_ROOT").expect("owned crash root"));
    assert_eq!(std::fs::read(root.join("phase2-crash-owner")).unwrap(),b"phase2-extra-owned-crash-test");
    let boundary=std::env::var("SDS_PHASE2_CRASH_BOUNDARY").unwrap();
    let kind=std::env::var("SDS_PHASE2_CRASH_KIND").unwrap();
    let (repository,plan,intent)=if kind=="restore" {restore_ready(&root)} else {replacement_ready(&root)};
    let evidence=serde_json::json!({"operationId":plan.id,"newFingerprint":intent.new_fingerprint,"baselineFingerprint":intent.deployment.fingerprint,"rollbackPath":intent.rollback_path()});
    std::fs::write(root.join("crash-evidence.json"),serde_json::to_vec(&evidence).unwrap()).unwrap();
    replacement::commit_test_hook(&repository,&plan,&intent,|at| {
        if format!("{at:?}")==boundary { std::process::exit(73); }
        Ok(())
    }).unwrap();
    panic!("requested crash boundary was not reached");
}

#[test]
fn real_process_termination_recovers_each_replacement_boundary() {
    use std::process::{Command,Stdio};
    for kind in ["update","restore"] {
    for boundary in ["AfterIntent","OldRenamed","NewRenamed","DatabaseCommitted","CleanupEntry","CleanupFinished"] {
        let root=tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("phase2-crash-owner"),b"phase2-extra-owned-crash-test").unwrap();
        let mut child=Command::new(std::env::current_exe().unwrap())
            .args(["--exact","sds::tests::phase2_extra::replacement_crash_child","--ignored","--nocapture"])
            .env("SDS_PHASE2_CRASH_ROOT",root.path()).env("SDS_PHASE2_CRASH_BOUNDARY",boundary).env("SDS_PHASE2_CRASH_KIND",kind)
            .stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap();
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(20);
        let exit=loop {
            if let Some(status)=child.try_wait().unwrap() {break status;}
            if std::time::Instant::now()>deadline {let _=child.kill();let _=child.wait();panic!("owned crash child timed out at {boundary}");}
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(exit.code(),Some(73),"{boundary}");
        let evidence:serde_json::Value=serde_json::from_slice(&std::fs::read(root.path().join("crash-evidence.json")).unwrap()).unwrap();
        let repository=Repository::open(&root.path().join("sds")).unwrap();
        let plan=repository.load_operation(evidence["operationId"].as_str().unwrap()).unwrap();
        let new=evidence["newFingerprint"].as_str().unwrap();
        let rollback=std::path::PathBuf::from(evidence["rollbackPath"].as_str().unwrap());
        assert!([plan.target.path.as_path(),rollback.as_path()].iter().any(|path| {
            fingerprint_tree(path).ok().is_some_and(|(_,fingerprint)|Some(&fingerprint)==plan.expected_fingerprint.as_ref() || fingerprint==new)
        }),"old or new copy must survive actual exit at {boundary}");
        for _ in 0..2 {
            replacement::recover_pending(&repository,&[plan.target.path.parent().unwrap().into()]).unwrap();
            assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1,new,"{boundary}");
            assert_eq!(repository.operation_status(&plan.id).unwrap().state,OperationState::Completed,"{boundary}");
            let deployment=repository.deployments().unwrap().into_iter().find(|item|Some(&item.id)==plan.deployment_id.as_ref()).unwrap();
            assert_eq!(deployment.fingerprint,evidence["baselineFingerprint"].as_str().unwrap());
            if kind=="restore" {assert_eq!(crate::sds::library::inspect_deployment(&deployment).status,crate::sds::contracts::DeploymentStatus::Modified);}
        }
    }
    }
}

fn restore_ready(root: &Path) -> (Arc<Repository>, PreparedOperation, replacement::ReplacementIntent) {
    let (repository,original)=installed(root);
    std::fs::write(original.target.path.join("SKILL.md"),b"# Saved local edits").unwrap();
    let saved=fingerprint_tree(&original.target.path).unwrap().1;
    let snapshots=SnapshotService::new(repository.clone());
    let snapshot=snapshots.capture(&original.id,&saved,SnapshotReason::Manual).unwrap();
    std::fs::write(original.target.path.join("SKILL.md"),b"# More recent local edits").unwrap();
    let mut plan=original.clone();
    plan.id=uuid::Uuid::new_v4().to_string();plan.kind="restore".into();plan.deployment_id=Some(original.id);
    plan.expected_fingerprint=Some(fingerprint_tree(&plan.target.path).unwrap().1);
    repository.create_operation(&plan,&OperationResult{operation_id:plan.id.clone(),state:OperationState::Prepared,stage:"review".into(),error:None}).unwrap();
    repository.save_options(&plan.id,&crate::sds::lifecycle::LifecycleOptions{snapshot_id:Some(snapshot.id.clone()),protect_current:true,..Default::default()}).unwrap();
    let artifact=snapshots.artifact(&snapshot.id).unwrap();
    let intent=replacement::stage(&repository,&plan,&artifact).unwrap();
    (repository,plan,intent)
}

#[test]
fn failed_schema_step_rolls_back_its_new_objects_and_version() {
    let root=tempfile::tempdir().unwrap();
    let path=root.path().join("pilot.sqlite3");
    let connection=rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch(include_str!("../migrations/001_pilot.sql")).unwrap();
    connection.execute_batch(include_str!("../migrations/002_workspaces.sql")).unwrap();
    connection.execute_batch(include_str!("../migrations/003_snapshots.sql")).unwrap();
    // The second CREATE in migration004 must fail after the first CREATE ran.
    connection.execute_batch("CREATE TABLE replacement_intents(sentinel TEXT); INSERT INTO replacement_intents VALUES('keep');").unwrap();
    drop(connection);
    assert!(Repository::open(root.path()).is_err());
    let connection=rusqlite::Connection::open(&path).unwrap();
    let version:u32=connection.pragma_query_value(None,"user_version",|row|row.get(0)).unwrap();
    assert_eq!(version,3);
    let new_objects:u32=connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name='lifecycle_options'",[],|row|row.get(0)).unwrap();
    assert_eq!(new_objects,0);
    assert_eq!(connection.query_row::<String,_,_>("SELECT sentinel FROM replacement_intents",[],|row|row.get(0)).unwrap(),"keep");
}

#[test]
fn migrations_preserve_pilot_release_deployment_and_pending_journal_bytes() {
    let root=tempfile::tempdir().unwrap();
    let path=root.path().join("pilot.sqlite3");
    let connection=rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch(include_str!("../migrations/001_pilot.sql")).unwrap();
    connection.execute("INSERT INTO releases VALUES('cache','profile','skill','release','release-bytes','manifest-bytes','fingerprint','created')",[]).unwrap();
    connection.execute("INSERT INTO deployments VALUES('deployment','cache','target-key','target-bytes','baseline-bytes','fingerprint','mcp-bytes','installed')",[]).unwrap();
    connection.execute("INSERT INTO operations VALUES('operation','frozen-bytes','result-bytes','pending-v1-intent-bytes','epoch','updated')",[]).unwrap();
    drop(connection);
    for _ in 0..2 { drop(Repository::open(root.path()).unwrap()); }
    let connection=rusqlite::Connection::open(&path).unwrap();
    assert_eq!(connection.query_row::<String,_,_>("SELECT release_json FROM releases WHERE cache_key='cache'",[],|row|row.get(0)).unwrap(),"release-bytes");
    assert_eq!(connection.query_row::<String,_,_>("SELECT target_json FROM deployments WHERE deployment_id='deployment'",[],|row|row.get(0)).unwrap(),"target-bytes");
    assert_eq!(connection.query_row::<String,_,_>("SELECT intent_json FROM operations WHERE operation_id='operation'",[],|row|row.get(0)).unwrap(),"pending-v1-intent-bytes");
    assert_eq!(connection.pragma_query_value::<u32,_>(None,"user_version",|row|row.get(0)).unwrap(),7);
}

#[test]
fn contract_fixture_domain_shapes() {
    use crate::sds::{contracts::{InstallRequest,TargetSelection,WorkspaceRef}, cache_cleanup::CacheDeletePlan,
        updates::{PreparedUpdateSelection,PreparedUpdateItem,UpdateSelectionResult,UpdateSelectionItem}};
    let root=tempfile::tempdir().unwrap();
    let (repository,plan)=installed(root.path());
    let snapshots=SnapshotService::new(repository);
    let fingerprint=fingerprint_tree(&plan.target.path).unwrap().1;
    let snapshot=snapshots.capture(&plan.id,&fingerprint,SnapshotReason::Manual).unwrap();
    let legacy:InstallRequest=serde_json::from_value(serde_json::json!({"skillId":"skill-123","releaseId":"rel-003"})).unwrap();
    let mut frozen_update=plan.clone();frozen_update.kind="update".into();frozen_update.deployment_id=Some(plan.id.clone());
    let selection=PreparedUpdateSelection{selection_id:uuid::Uuid::new_v4().to_string(),items:vec![PreparedUpdateItem{deployment_id:plan.id.clone(),operation_id:Some(plan.id.clone()),operation:Some(frozen_update),error:None}]};
    let result=UpdateSelectionResult{selection_id:selection.selection_id.clone(),items:vec![UpdateSelectionItem{deployment_id:plan.id.clone(),operation_id:Some(plan.id.clone()),result:Some(OperationResult{operation_id:plan.id.clone(),state:OperationState::Completed,stage:"completed".into(),error:None}),preflight_error:None}]};
    println!("SDS_CONTRACT_LIFECYCLE_DOMAIN={}",serde_json::json!({
        "workspace":WorkspaceRef{id:uuid::Uuid::new_v4().to_string(),root:root.path().into()},
        "targetSelection":TargetSelection::default(),"snapshot":snapshot,"prepared":plan,
        "cacheDelete":CacheDeletePlan{id:uuid::Uuid::new_v4().to_string(),release_key:"a".repeat(64),expected_fingerprint:fingerprint,runtime_epoch:"contract-fixture".into()},
        "preparedSelection":selection,"selectionResult":result,"legacyInstall":legacy
    }));
}

#[test]
fn competing_update_and_remove_have_only_one_commit_winner() {
    for _ in 0..4 {
        let root=tempfile::tempdir().unwrap();
        let (repository,update,intent)=replacement_ready(root.path());
        let removal=crate::sds::removal::prepare(&repository,"race-runtime",update.deployment_id.as_deref().unwrap()).unwrap();
        let removal_intent=crate::sds::removal::stage_remove(&repository,&removal).unwrap();
        let barrier=Arc::new(std::sync::Barrier::new(3));
        let (repo,ready,frozen,prepared)=(repository.clone(),barrier.clone(),update.clone(),intent.clone());
        let updating=std::thread::spawn(move||{ready.wait();replacement::commit(&repo,&frozen,&prepared)});
        let (repo,ready)=(repository.clone(),barrier.clone());
        let removing=std::thread::spawn(move||{ready.wait();crate::sds::removal::commit_remove(&repo,&removal,&removal_intent)});
        barrier.wait();
        let update_result=updating.join().unwrap();
        let remove_result=removing.join().unwrap();
        assert_ne!(update_result.is_ok(),remove_result.is_ok(),"Exactly one reviewed mutation can win this target");
        if update_result.is_ok() {
            assert_eq!(fingerprint_tree(&update.target.path).unwrap().1,intent.new_fingerprint);
            assert_eq!(repository.deployments().unwrap().len(),1);
        } else {
            assert!(!update.target.path.exists());
            assert!(repository.deployments().unwrap().is_empty());
            assert!(intent.stage_path().join("SKILL.md").exists(),"The losing replacement retains its verified new content");
        }
    }
}
