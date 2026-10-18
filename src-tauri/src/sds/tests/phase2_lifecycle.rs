use super::support::{Response, TestServer};
use crate::sds::{
    cache_cleanup,
    contracts::*,
    deployment::fingerprint_tree,
    fixtures::FixtureVault,
    operations::OperationService,
    profile::Environment,
    removal::RemoveRequest,
    repository::Repository,
    session::SessionService,
    snapshots::SnapshotService,
    store_client::ReqwestTransport,
    targets::TargetService,
    updates::{LocalEdits, UpdateRequest},
};
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
fn json(name: &str) -> serde_json::Value {
    serde_json::from_str(match name {
        "login" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"),
        "detail" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/detail.json"),
        _ => include_str!("../../../../contracts/sds-desktop/v1/fixtures/release.json"),
    })
    .unwrap()
}
struct Server {
    http: TestServer,
    mode: Arc<AtomicUsize>,
    downloads: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
}
async fn server() -> Server {
    let mode = Arc::new(AtomicUsize::new(0));
    let downloads = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let (m, d, n) = (mode.clone(), downloads.clone(), requests.clone());
    let old =
        include_bytes!("../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.zip")
            .to_vec();
    use std::io::{Cursor, Write};
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("SKILL.md", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"# Second immutable release").unwrap();
    let new = zip.finish().unwrap().into_inner();
    let http=TestServer::start(move|request|{let(m,d,n,old,new)=(m.clone(),d.clone(),n.clone(),old.clone(),new.clone());async move{
  n.fetch_add(1,Ordering::SeqCst);if request.path.ends_with("/auth/login"){return Response::json(200,json("login"));}
  let mode=m.load(Ordering::SeqCst);if mode==1{return Response::json(0,serde_json::Value::Null);}if [2,3,4].contains(&mode){return Response::json(if mode==2{403}else if mode==3{401}else{410},serde_json::json!({"success":false}));}
  let new=if mode==8{old.clone()}else{new};
  let is_new=request.path.contains("rel-004");
  if request.path.contains("package"){d.fetch_add(1,Ordering::SeqCst);return Response::binary(if is_new{new}else{old});}
  if request.path.contains("/releases/"){let mut body=json("release");let bytes=if is_new{&new}else{&old};let r=&mut body["data"];r["package"]["format"]="zip".into();r["package"]["fileName"]="synthetic.zip".into();r["package"]["sha256"]=hex::encode(Sha256::digest(bytes)).into();r["package"]["sizeBytes"]=(bytes.len() as u64).into();if is_new{r["id"]="rel-004".into();r["sequence"]=4.into();r["requiredMcps"]=serde_json::json!(["new-mcp"]);r["package"]["downloadPath"]="/api/v1/desktop/skills/skill-123/releases/rel-004/package".into();}if mode==5{r["installName"]="changed-name".into();}return Response::json(200,body);}
  if request.path.contains("updates/check")&&[6,7].contains(&mode){return Response::json(200,serde_json::json!({"success":true,"data":{"checkedAt":"2026-10-18T00:00:00Z","items":[{"skillId":"skill-123","currentReleaseId":"rel-003","status":if mode==6{"unavailable"}else{"unknown_release"},"latestRelease":null}]}}));}
  if request.path.contains("updates/check"){return Response::json(200,serde_json::json!({"success":true,"data":{"checkedAt":"2026-10-18T00:00:00Z","items":[{"skillId":"skill-123","currentReleaseId":"rel-003","status":"update_available","latestRelease":{"id":"rel-004","sequence":4,"version":"2"}}]}}));}
  Response::json(200,json("detail"))
 }}).await;
    Server {
        http,
        mode,
        downloads,
        requests,
    }
}
async fn setup(
    root: &std::path::Path,
    s: &Server,
) -> (
    Arc<SessionService>,
    Arc<Repository>,
    Arc<TargetService>,
    OperationService,
    Arc<FixtureVault>,
) {
    let vault = Arc::new(FixtureVault::default());
    let profile = StoreProfile::new(&s.http.base_url, Environment::Development).unwrap();
    let session = Arc::new(SessionService::new(
        Environment::Development,
        Some(profile),
        Arc::new(ReqwestTransport::fixture().unwrap()),
        vault.clone(),
    ));
    session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let targets = Arc::new(TargetService::fixture(root).unwrap());
    let repo = Arc::new(Repository::open(&root.join("pilot")).unwrap());
    let op = OperationService::new(session.clone(), targets.clone(), repo.clone());
    (session, repo, targets, op, vault)
}
async fn install(op: &OperationService, target: TargetSelection) -> PreparedOperation {
    let p = op
        .prepare_install(InstallRequest {
            skill_id: "skill-123".into(),
            release_id: "rel-003".into(),
            target,
        })
        .await
        .unwrap();
    assert_eq!(
        op.execute(&p.id, true).await.unwrap().state,
        OperationState::Completed
    );
    p
}
fn request(id: &str) -> UpdateRequest {
    UpdateRequest {
        deployment_id: id.into(),
        release_id: "rel-004".into(),
        local_edits: LocalEdits::BackupAndReplace,
    }
}
async fn same_bytes_custom_global_update(hold_reader: bool) {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    s.mode.store(8, Ordering::SeqCst);
    let (session, repo, _, initial, _) = setup(t.path(), &s).await;
    drop(initial);
    let target_root = t.path().join("previous-pilot/a10-target");
    std::fs::create_dir_all(&target_root).unwrap();
    let home = t.path().join("agent-home");
    let settings = t.path().join("target-settings");
    TargetService::new(&home, &settings)
        .set_override(Some(&target_root))
        .unwrap();
    let targets = Arc::new(TargetService::new(&home, &settings));
    let op = OperationService::new(session, targets, repo.clone());
    let installed = install(&op, Default::default()).await;
    let old = repo.deployments().unwrap().pop().unwrap();
    let update = op.prepare_update(request(&installed.id)).await.unwrap();
    assert_eq!(update.target.path.parent(), Some(target_root.as_path()));
    let reader = hold_reader
        .then(|| crate::sds::deployment::DirectoryPins::acquire(&installed.target.path).unwrap());
    let result = op.execute(&update.id, true).await.unwrap();
    if hold_reader {
        assert_eq!(result.state, OperationState::RecoveryRequired, "{result:?}");
        assert_eq!(result.error.unwrap().code, "REVIEW_REQUIRED");
        let (intent, committed) = repo.replacement(&update.id).unwrap().unwrap();
        assert!(!committed);
        assert!(update.target.path.exists());
        assert!(intent.stage_path().exists());
        assert!(!intent.rollback_path().exists());
        assert_eq!(repo.deployments().unwrap()[0].identity, old.identity);
        drop(reader);
        crate::sds::replacement::recover_pending(&repo, &[target_root]).unwrap();
    } else {
        assert_eq!(result.state, OperationState::Completed, "{result:?}");
    }
    assert_eq!(
        op.status(&update.id).unwrap().state,
        OperationState::Completed
    );
    let (intent, committed) = repo.replacement(&update.id).unwrap().unwrap();
    assert!(committed);
    assert_eq!(
        intent.old_fingerprint.as_ref(),
        Some(&intent.new_fingerprint)
    );
    let updated = repo.deployments().unwrap().pop().unwrap();
    assert_eq!(updated.id, old.id);
    assert_ne!(updated.identity, old.identity);
    assert_eq!(updated.fingerprint, old.fingerprint);
    assert_eq!(
        repo.cached_key(&updated.release_key).unwrap().release.id,
        "rel-004"
    );
    assert!(!intent.stage_path().exists());
    assert!(!intent.rollback_path().exists());
}
#[tokio::test]
async fn same_bytes_global_update_through_saved_custom_root_completes() {
    same_bytes_custom_global_update(false).await;
}
#[tokio::test]
async fn same_bytes_global_update_reader_pin_retains_intent_then_recovers_after_release() {
    same_bytes_custom_global_update(true).await;
}
#[tokio::test]
async fn selected_update_changes_only_one_of_global_and_two_projects_with_new_mcp_review() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, repo, targets, op, _) = setup(t.path(), &s).await;
    let global = install(&op, Default::default()).await;
    let mut locals = vec![];
    for name in ["project-a", "project-b"] {
        let root = t.path().join(name);
        std::fs::create_dir(&root).unwrap();
        let w = targets.add_workspace(&root).unwrap();
        locals.push(
            install(
                &op,
                TargetSelection {
                    agent_key: "aipro".into(),
                    workspace_id: w.id,
                },
            )
            .await,
        );
    }
    let before = fingerprint_tree(&global.target.path).unwrap().1;
    let new = op.prepare_update(request(&locals[0].id)).await.unwrap();
    assert_eq!(new.release.required_mcps.as_ref().unwrap(), &["new-mcp"]);
    assert_eq!(
        op.execute(&new.id, false).await.unwrap_err().code,
        "MCP_ACK_REQUIRED"
    );
    assert_eq!(
        op.execute(&new.id, true).await.unwrap().state,
        OperationState::Completed
    );
    assert_eq!(fingerprint_tree(&global.target.path).unwrap().1, before);
    assert_eq!(fingerprint_tree(&locals[1].target.path).unwrap().1, before);
    assert_ne!(fingerprint_tree(&locals[0].target.path).unwrap().1, before);
    let d = repo
        .deployments()
        .unwrap()
        .into_iter()
        .find(|d| d.id == locals[0].id)
        .unwrap();
    assert_eq!(
        repo.cached_key(&d.release_key).unwrap().release.id,
        "rel-004"
    );
    assert_eq!(repo.deployments().unwrap().len(), 3);
}
#[tokio::test]
async fn keep_current_does_no_network_and_revocation_or_snapshot_failure_keep_current_bytes() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, _, _, op, _) = setup(t.path(), &s).await;
    let installed = install(&op, Default::default()).await;
    let before = fingerprint_tree(&installed.target.path).unwrap().1;
    let count = s.requests.load(Ordering::SeqCst);
    let mut keep = request(&installed.id);
    keep.local_edits = LocalEdits::Keep;
    assert_eq!(
        op.prepare_update(keep).await.unwrap_err().code,
        "KEEP_CURRENT"
    );
    assert_eq!(s.requests.load(Ordering::SeqCst), count);
    let revoked = op.prepare_update(request(&installed.id)).await.unwrap();
    s.mode.store(2, Ordering::SeqCst);
    assert!(op.execute(&revoked.id, true).await.is_err());
    assert_eq!(fingerprint_tree(&installed.target.path).unwrap().1, before);
    op.cancel(&revoked.id).unwrap();
    s.mode.store(0, Ordering::SeqCst);
    std::fs::write(installed.target.path.join("local.txt"), "edited").unwrap();
    let edited = fingerprint_tree(&installed.target.path).unwrap().1;
    let new = op.prepare_update(request(&installed.id)).await.unwrap();
    std::fs::write(t.path().join("pilot/snapshots"), "blocked").unwrap();
    let result = op.execute(&new.id, true).await.unwrap();
    assert_eq!(result.state, OperationState::Failed);
    assert_eq!(fingerprint_tree(&installed.target.path).unwrap().1, edited);
}
#[tokio::test]
async fn install_name_changes_are_explicit_and_selected_success_is_not_replayed() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, _, _, op, _) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    s.mode.store(5, Ordering::SeqCst);
    assert_eq!(
        op.prepare_update(request(&p.id)).await.unwrap_err().code,
        "INSTALL_NAME_CHANGED"
    );
    s.mode.store(0, Ordering::SeqCst);
    let group = op
        .prepare_updates(vec![
            request(&p.id),
            request(&uuid::Uuid::new_v4().to_string()),
        ])
        .await
        .unwrap();
    assert!(group.items[1].error.is_some());
    let ids = group
        .items
        .iter()
        .filter_map(|i| i.operation_id.clone())
        .collect();
    let result = op.execute_updates(&group.selection_id, ids).await.unwrap();
    assert_eq!(result[0].state, OperationState::Completed);
    let count = s.downloads.load(Ordering::SeqCst);
    op.execute_updates(&group.selection_id, vec![])
        .await
        .unwrap();
    assert_eq!(s.downloads.load(Ordering::SeqCst), count);
}
#[tokio::test]
async fn cache_missing_after_snapshot_restore_store_install_downloads_verified_copy() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (session, repo, _, op, _) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    let baseline = fingerprint_tree(&p.target.path).unwrap().1;
    let key = repo.deployments().unwrap()[0].release_key.clone();
    assert_eq!(
        op.prepare_cache_delete(key.clone()).await.unwrap_err().code,
        "RELEASE_IN_USE"
    );
    std::fs::write(p.target.path.join("SKILL.md"), "# edited").unwrap();
    let edited = fingerprint_tree(&p.target.path).unwrap().1;
    let blocked = op
        .prepare_remove_request(RemoveRequest {
            deployment_id: p.id.clone(),
            backup: true,
            acknowledge_local_edits: false,
        })
        .await
        .unwrap();
    assert!(!blocked.can_execute);
    op.cancel(&blocked.id).unwrap();
    let removal = op
        .prepare_remove_request(RemoveRequest {
            deployment_id: p.id.clone(),
            backup: true,
            acknowledge_local_edits: true,
        })
        .await
        .unwrap();
    assert_eq!(
        op.execute(&removal.id, false).await.unwrap().state,
        OperationState::Completed
    );
    assert!(!p.target.path.exists());
    assert_eq!(repo.list_library().unwrap().len(), 1);
    let snapshots = SnapshotService::new(repo.clone());
    let snapshot = snapshots.list().unwrap().pop().unwrap();
    assert_eq!(snapshot.content_fingerprint, edited);
    let delete = op.prepare_cache_delete(key.clone()).await.unwrap();
    assert_eq!(
        op.execute_cache_delete(delete.id).await.unwrap().state,
        OperationState::Completed
    );
    assert!(repo.list_library().unwrap().is_empty());
    snapshots.verify(&snapshot.id).unwrap();
    let restore = op.prepare_restore(&snapshot.id, &p.id, true).await.unwrap();
    assert_eq!(
        op.execute(&restore.id, true).await.unwrap().state,
        OperationState::Completed
    );
    let d = repo.deployments().unwrap().pop().unwrap();
    assert_eq!(d.fingerprint, baseline);
    assert_eq!(fingerprint_tree(&d.target.path).unwrap().1, edited);
    assert_eq!(
        crate::sds::library::inspect_deployment(&d).status,
        DeploymentStatus::Modified
    );
    // Restore retains release metadata for the deployment, but not cached bytes.
    assert_eq!(repo.list_library().unwrap().len(), 1);
    assert!(!repo.root().join("releases").join(&key).exists());
    let fresh_targets = Arc::new(TargetService::fixture(&t.path().join("fresh-global")).unwrap());
    drop(op);
    let fresh = OperationService::new(session.clone(), fresh_targets, repo.clone());
    s.mode.store(1, Ordering::SeqCst);
    assert!(fresh.check_updates().await.is_err());
    assert_eq!(
        session.status().await.unwrap().status,
        SessionStatus::Offline
    );
    let requests = s.requests.load(Ordering::SeqCst);
    assert!(fresh
        .prepare_cached_install(&key, Default::default())
        .await
        .is_err());
    assert_eq!(s.requests.load(Ordering::SeqCst), requests);
    s.mode.store(0, Ordering::SeqCst);
    let downloads = s.downloads.load(Ordering::SeqCst);
    let plan = fresh
        .prepare_install(InstallRequest {
            skill_id: "skill-123".into(),
            release_id: "rel-003".into(),
            target: Default::default(),
        })
        .await
        .unwrap();
    assert_ne!(plan.target.path, d.target.path);
    assert!(!plan.target.path.exists());
    let result = fresh.execute(&plan.id, true).await.unwrap();
    assert_eq!(result.state, OperationState::Completed, "{result:?}");
    assert_eq!(s.downloads.load(Ordering::SeqCst), downloads + 1);
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, baseline);
    assert_eq!(fingerprint_tree(&d.target.path).unwrap().1, edited);
    assert_eq!(
        repo.cached_key(&key).unwrap().artifact.fingerprint,
        baseline
    );
    assert_eq!(repo.deployments().unwrap().len(), 2);
}

#[tokio::test]
async fn cache_missing_fix_keeps_tamper_and_offline_deploy_fail_closed() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (session, repo, _, op, _) = setup(t.path(), &s).await;
    let installed = install(&op, Default::default()).await;
    let key = repo.deployments().unwrap()[0].release_key.clone();
    let cache_root = repo.cached_key(&key).unwrap().artifact.root;
    std::fs::write(cache_root.join("SKILL.md"), "# tampered cache").unwrap();
    let fresh_targets = Arc::new(TargetService::fixture(&t.path().join("fresh-global")).unwrap());
    drop(op);
    let fresh = OperationService::new(session.clone(), fresh_targets, repo.clone());
    let plan = fresh
        .prepare_install(InstallRequest {
            skill_id: "skill-123".into(),
            release_id: "rel-003".into(),
            target: Default::default(),
        })
        .await
        .unwrap();
    let downloads = s.downloads.load(Ordering::SeqCst);
    let result = fresh.execute(&plan.id, true).await.unwrap();
    assert_eq!(result.state, OperationState::Failed);
    assert_eq!(result.error.unwrap().code, "CACHE_CORRUPT");
    assert_eq!(s.downloads.load(Ordering::SeqCst), downloads);
    assert!(!plan.target.path.exists());
    assert!(installed.target.path.exists());
    assert_eq!(repo.deployments().unwrap().len(), 1);
    s.mode.store(1, Ordering::SeqCst);
    assert!(fresh.check_updates().await.is_err());
    assert_eq!(
        session.status().await.unwrap().status,
        SessionStatus::Offline
    );
    let requests = s.requests.load(Ordering::SeqCst);
    // Unlike the snapshot restore case, this account still has its download grant.
    assert!(fresh
        .prepare_cached_install(&key, Default::default())
        .await
        .is_err());
    std::fs::rename(&cache_root, t.path().join("quarantined-cache")).unwrap();
    assert!(fresh
        .prepare_cached_install(&key, Default::default())
        .await
        .is_err());
    assert_eq!(s.requests.load(Ordering::SeqCst), requests);
    assert!(!plan.target.path.exists());
}
#[test]
fn cache_cleanup_restarts_after_each_boundary_without_deleting_newly_published_content() {
    use cache_cleanup::CacheBoundary as B;
    for boundary in [
        B::AfterIntent,
        B::AfterRename,
        B::AfterDatabase,
        B::AfterCleanupEntry,
        B::AfterCleanup,
    ] {
        let t = tempfile::tempdir().unwrap();
        let (repo, _, _) = super::recovery::prepared(t.path());
        repo.expire_precommit("next-runtime").unwrap();
        let cache = repo.list_library().unwrap().pop().unwrap();
        let plan = cache_cleanup::prepare(&repo, "runtime", &cache.id).unwrap();
        assert!(
            cache_cleanup::execute_test_hook(&repo, "runtime", &plan.id, |at| if at == boundary {
                Err(crate::sds::error::SdsError::invalid("crash"))
            } else {
                Ok(())
            })
            .is_err()
        );
        let data = repo.root().to_path_buf();
        drop(repo);
        let repo = Repository::open(&data).unwrap();
        cache_cleanup::recover_pending(&repo).unwrap();
        cache_cleanup::recover_pending(&repo).unwrap();
        assert!(repo.list_library().unwrap().is_empty(), "{boundary:?}");
    }
}
#[tokio::test]
async fn offline_cached_deploy_has_zero_http_and_logout_expiry_403_and_restart_are_distinct() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (session, repo, targets, op, vault) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    let key = repo.deployments().unwrap()[0].release_key.clone();
    let project = t.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let w = targets.add_workspace(&project).unwrap();
    let selection = TargetSelection {
        agent_key: "aipro".into(),
        workspace_id: w.id,
    };
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    assert_eq!(
        session.status().await.unwrap().status,
        SessionStatus::Offline
    );
    let plan = op
        .prepare_cached_install(&key, selection.clone())
        .await
        .unwrap();
    let count = s.requests.load(Ordering::SeqCst);
    assert_eq!(
        op.execute(&plan.id, true).await.unwrap().state,
        OperationState::Completed
    );
    assert_eq!(s.requests.load(Ordering::SeqCst), count);
    assert_eq!(
        fingerprint_tree(&p.target.path).unwrap().1,
        fingerprint_tree(&plan.target.path).unwrap().1
    );
    s.mode.store(2, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    assert_eq!(
        session.status().await.unwrap().status,
        SessionStatus::Authenticated
    );
    assert!(op
        .prepare_cached_install(&key, selection.clone())
        .await
        .is_err());
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    drop(op);
    drop(session);
    let profile = StoreProfile::new(&s.http.base_url, Environment::Development).unwrap();
    let resumed = Arc::new(SessionService::new(
        Environment::Development,
        Some(profile),
        Arc::new(ReqwestTransport::fixture().unwrap()),
        vault,
    ));
    assert_eq!(
        resumed.status().await.unwrap().status,
        SessionStatus::Offline
    );
    let op = OperationService::new(resumed.clone(), targets, repo.clone());
    assert!(
        op.prepare_cached_install(&key, selection.clone())
            .await
            .unwrap()
            .can_execute
    );
    resumed.logout().await.unwrap();
    assert_eq!(
        op.prepare_cached_install(&key, selection)
            .await
            .unwrap_err()
            .code,
        "SESSION_EXPIRED"
    );
    assert!(p.target.path.exists());
}

#[tokio::test]
async fn cached_review_rejects_reconnection_logout_corruption_and_unknown_legacy_grants() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (session, repo, targets, op, _) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    let key = repo.deployments().unwrap()[0].release_key.clone();
    let root = t.path().join("cached-project");
    std::fs::create_dir(&root).unwrap();
    let w = targets.add_workspace(&root).unwrap();
    let target = TargetSelection {
        agent_key: "aipro".into(),
        workspace_id: w.id,
    };
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    let review = op
        .prepare_cached_install(&key, target.clone())
        .await
        .unwrap();
    s.mode.store(0, Ordering::SeqCst);
    assert!(op.check_updates().await.is_ok());
    assert!(op.execute(&review.id, true).await.is_err());
    assert!(!review.target.path.exists());
    op.cancel(&review.id).unwrap();
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    let review = op
        .prepare_cached_install(&key, target.clone())
        .await
        .unwrap();
    session.logout().await.unwrap();
    assert!(op.execute(&review.id, true).await.is_err());
    assert!(!review.target.path.exists());
    op.cancel(&review.id).unwrap();
    s.mode.store(0, Ordering::SeqCst);
    session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    repo.connection
        .lock()
        .unwrap()
        .execute("DELETE FROM cache_access", [])
        .unwrap();
    assert_eq!(
        op.prepare_cached_install(&key, target.clone())
            .await
            .unwrap_err()
            .code,
        "CACHE_NOT_AUTHORIZED"
    );
    let context = session.scope().await.unwrap().1;
    repo.grant_cache(&context, &key).unwrap();
    std::fs::write(
        repo.root().join("releases").join(&key).join("SKILL.md"),
        "tampered",
    )
    .unwrap();
    assert_eq!(
        op.prepare_cached_install(&key, target)
            .await
            .unwrap_err()
            .code,
        "CACHE_CORRUPT"
    );
    assert!(p.target.path.exists());
}
#[tokio::test]
async fn item_access_denial_then_outage_cannot_reuse_old_cached_grant() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, repo, _, op, _) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    let key = repo.deployments().unwrap()[0].release_key.clone();
    s.mode.store(2, Ordering::SeqCst);
    assert!(op.prepare_update(request(&p.id)).await.is_err());
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    assert_eq!(
        op.prepare_cached_install(&key, Default::default())
            .await
            .unwrap_err()
            .code,
        "CACHE_NOT_AUTHORIZED"
    );
}
#[tokio::test]
async fn snapshot_failure_blocks_modified_removal_and_cache_plan_rechecks_new_reference() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, repo, _, op, _) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    std::fs::write(p.target.path.join("mine.txt"), "mine").unwrap();
    let before = fingerprint_tree(&p.target.path).unwrap().1;
    let review = op
        .prepare_remove_request(RemoveRequest {
            deployment_id: p.id.clone(),
            backup: true,
            acknowledge_local_edits: true,
        })
        .await
        .unwrap();
    std::fs::write(repo.root().join("snapshots"), "block").unwrap();
    assert_eq!(
        op.execute(&review.id, false).await.unwrap().state,
        OperationState::Failed
    );
    assert_eq!(fingerprint_tree(&p.target.path).unwrap().1, before);
    assert_eq!(repo.deployments().unwrap().len(), 1);
    let root = tempfile::tempdir().unwrap();
    let (repo, plan, _) = super::recovery::prepared(root.path());
    repo.expire_precommit("next").unwrap();
    let key = repo.list_library().unwrap()[0].id.clone();
    let delete = cache_cleanup::prepare(&repo, "epoch", &key).unwrap();
    let mut reserve = plan;
    reserve.id = uuid::Uuid::new_v4().to_string();
    repo.create_operation(
        &reserve,
        &OperationResult {
            operation_id: reserve.id.clone(),
            state: OperationState::Prepared,
            stage: "review".into(),
            error: None,
        },
    )
    .unwrap();
    assert_eq!(
        cache_cleanup::execute(&repo, "epoch", &delete.id)
            .unwrap_err()
            .code,
        "RELEASE_IN_USE"
    );
    assert!(repo.root().join("releases").join(key).exists());
}

#[tokio::test]
async fn metadata_unavailable_or_unknown_release_revokes_only_that_items_prior_grant() {
    for mode in [6, 7] {
        let t = tempfile::tempdir().unwrap();
        let s = server().await;
        let (_, repo, _, op, _) = setup(t.path(), &s).await;
        install(&op, Default::default()).await;
        let key = repo.deployments().unwrap()[0].release_key.clone();
        s.mode.store(mode, Ordering::SeqCst);
        let checked = op.check_updates().await.unwrap();
        assert_eq!(checked.items.len(), 1);
        s.mode.store(1, Ordering::SeqCst);
        assert!(op.check_updates().await.is_err());
        assert_eq!(
            op.prepare_cached_install(&key, Default::default())
                .await
                .unwrap_err()
                .code,
            "CACHE_NOT_AUTHORIZED"
        );
    }
}
#[tokio::test]
async fn repeated_offline_cached_install_confirms_existing_managed_release_without_network() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, repo, _, op, _) = setup(t.path(), &s).await;
    let p = install(&op, Default::default()).await;
    let key = repo.deployments().unwrap()[0].release_key.clone();
    s.mode.store(1, Ordering::SeqCst);
    assert!(op.check_updates().await.is_err());
    let review = op
        .prepare_cached_install(&key, Default::default())
        .await
        .unwrap();
    let count = s.requests.load(Ordering::SeqCst);
    assert_eq!(
        op.execute(&review.id, true).await.unwrap().state,
        OperationState::Completed
    );
    assert_eq!(s.requests.load(Ordering::SeqCst), count);
    assert_eq!(repo.deployments().unwrap().len(), 1);
    assert!(p.target.path.exists());
}
#[tokio::test]
async fn failed_selected_target_retries_with_new_review_without_replaying_success() {
    let t = tempfile::tempdir().unwrap();
    let s = server().await;
    let (_, repo, targets, op, _) = setup(t.path(), &s).await;
    let mut installs = vec![];
    for name in ["first", "second"] {
        let root = t.path().join(name);
        std::fs::create_dir(&root).unwrap();
        let w = targets.add_workspace(&root).unwrap();
        installs.push(
            install(
                &op,
                TargetSelection {
                    agent_key: "aipro".into(),
                    workspace_id: w.id,
                },
            )
            .await,
        );
    }
    std::fs::write(installs[1].target.path.join("edited.txt"), "local").unwrap();
    let edited = fingerprint_tree(&installs[1].target.path).unwrap().1;
    let group = op
        .prepare_updates(installs.iter().map(|p| request(&p.id)).collect())
        .await
        .unwrap();
    assert!(group
        .items
        .iter()
        .all(|i| i.operation.as_ref().unwrap().id == i.operation_id.as_ref().unwrap().as_str()));
    std::fs::write(repo.root().join("snapshots"), "block").unwrap();
    let results = op
        .execute_updates(
            &group.selection_id,
            group
                .items
                .iter()
                .filter_map(|i| i.operation_id.clone())
                .collect(),
        )
        .await
        .unwrap();
    assert_eq!(results[0].state, OperationState::Completed);
    assert_eq!(results[1].state, OperationState::Failed);
    assert_eq!(
        fingerprint_tree(&installs[1].target.path).unwrap().1,
        edited
    );
    let first = repo
        .deployments()
        .unwrap()
        .into_iter()
        .find(|d| d.id == installs[0].id)
        .unwrap();
    std::fs::remove_file(repo.root().join("snapshots")).unwrap();
    let retry = op
        .prepare_updates(vec![request(&installs[1].id)])
        .await
        .unwrap();
    let result = op
        .execute_updates(
            &retry.selection_id,
            retry
                .items
                .iter()
                .filter_map(|i| i.operation_id.clone())
                .collect(),
        )
        .await
        .unwrap();
    assert_eq!(result[0].state, OperationState::Completed);
    let first_after = repo
        .deployments()
        .unwrap()
        .into_iter()
        .find(|d| d.id == installs[0].id)
        .unwrap();
    assert_eq!(first.identity, first_after.identity);
    assert_eq!(first.installed_at, first_after.installed_at);
    assert_eq!(SnapshotService::new(repo).list().unwrap().len(), 1);
}
#[test]
#[ignore = "owned cache cleanup process helper; parent runs each boundary"]
fn cache_crash_child() {
    let root = std::path::PathBuf::from(std::env::var_os("SDS_PHASE2_CACHE_ROOT").unwrap());
    assert_eq!(std::fs::read(root.join("owner")).unwrap(), b"cache-owned");
    let boundary = std::env::var("SDS_PHASE2_CACHE_BOUNDARY").unwrap();
    let (repo, _, _) = super::recovery::prepared(&root);
    repo.expire_precommit("new").unwrap();
    let key = repo.list_library().unwrap()[0].id.clone();
    let plan = cache_cleanup::prepare(&repo, "epoch", &key).unwrap();
    std::fs::write(root.join("plan-id"), &plan.id).unwrap();
    cache_cleanup::execute_test_hook(&repo, "epoch", &plan.id, |at| {
        if format!("{at:?}") == boundary {
            std::process::exit(73);
        }
        Ok(())
    })
    .unwrap();
    panic!("missing boundary");
}
#[test]
fn cache_cleanup_survives_actual_process_exit_at_every_durable_boundary() {
    use std::process::{Command, Stdio};
    for boundary in [
        "AfterIntent",
        "AfterRename",
        "AfterDatabase",
        "AfterCleanupEntry",
        "AfterCleanup",
    ] {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("owner"), "cache-owned").unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sds::tests::phase2_lifecycle::cache_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("SDS_PHASE2_CACHE_ROOT", t.path())
            .env("SDS_PHASE2_CACHE_BOUNDARY", boundary)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() > deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("crash child timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(73), "{boundary}");
        let repo = Repository::open(&t.path().join("sds")).unwrap();
        for _ in 0..2 {
            cache_cleanup::recover_pending(&repo).unwrap();
            assert!(repo.list_library().unwrap().is_empty(), "{boundary}");
        }
    }
}

#[test]
#[ignore = "owned preintent crash helper"]
fn replacement_preintent_child() {
    let root = std::path::PathBuf::from(std::env::var_os("SDS_PHASE2_PREINTENT_ROOT").unwrap());
    assert_eq!(
        std::fs::read(root.join("owner")).unwrap(),
        b"preintent-owned"
    );
    let (repo, plan, artifact) = super::replacement::fixture(&root);
    crate::sds::replacement::stage_prepared(&repo, &plan, &artifact).unwrap();
    assert!(!repo.has_intent(&plan.id).unwrap());
    std::fs::write(root.join("operation-id"), &plan.id).unwrap();
    std::process::exit(73);
}
#[test]
fn process_exit_before_replacement_intent_keeps_current_target_and_expires_review() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("owner"), "preintent-owned").unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sds::tests::phase2_lifecycle::replacement_preintent_child",
            "--ignored",
        ])
        .env("SDS_PHASE2_PREINTENT_ROOT", t.path())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    let repo = Repository::open(&t.path().join("sds")).unwrap();
    let id = std::fs::read_to_string(t.path().join("operation-id")).unwrap();
    let plan = repo.load_operation(&id).unwrap();
    let before = fingerprint_tree(&plan.target.path).unwrap().1;
    assert_eq!(Some(&before), plan.expected_fingerprint.as_ref());
    repo.expire_precommit("next-epoch").unwrap();
    crate::sds::replacement::recover_pending(&repo, &[plan.target.path.parent().unwrap().into()])
        .unwrap();
    assert_eq!(
        repo.operation_status(&id).unwrap().state,
        OperationState::Failed
    );
    assert_eq!(fingerprint_tree(&plan.target.path).unwrap().1, before);
}
