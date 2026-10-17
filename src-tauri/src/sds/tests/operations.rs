use super::support::{Response, TestServer};
use crate::sds::{
    commands::SdsServices,
    contracts::{InstallRequest, OperationState},
    operations::OperationService,
    repository::Repository,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn fixture(name: &str) -> serde_json::Value {
    serde_json::from_str(match name {
        "login" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"),
        "detail" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/detail.json"),
        _ => include_str!("../../../../contracts/sds-desktop/v1/fixtures/release.json"),
    })
    .unwrap()
}
fn request() -> InstallRequest {
    InstallRequest {
        skill_id: "skill-123".into(),
        release_id: "rel-003".into(),
    }
}

async fn setup(
    server: &TestServer,
    root: &std::path::Path,
) -> (SdsServices, Arc<Repository>, OperationService) {
    let services = SdsServices::new_fixture(root, &server.base_url).unwrap();
    services
        .session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let repo = Arc::new(Repository::open(&root.join("repository")).unwrap());
    let operations = OperationService::new(
        services.session.clone(),
        services.targets.clone(),
        repo.clone(),
    );
    (services, repo, operations)
}
async fn server() -> TestServer {
    TestServer::start(|request| async move {
        Response::json(
            200,
            fixture(if request.path.ends_with("/auth/login") {
                "login"
            } else if request.path.contains("/releases/") {
                "release"
            } else {
                "detail"
            }),
        )
    })
    .await
}

#[tokio::test]
async fn preparation_freezes_backend_release_session_and_destination_without_creating_skills() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let (services, repo, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    assert!(plan.can_execute);
    assert!(plan.requires_mcp_ack);
    assert_eq!(plan.release.required_mcps.as_ref().unwrap(), &["jira-mcp"]);
    assert_eq!(
        plan.target.path,
        root.path()
            .join("agent-home/.aipro/skills/jira-weekly-report")
    );
    assert!(!plan.target.path.parent().unwrap().exists());
    assert_eq!(plan.context, services.session.scope().await.unwrap().1);
    assert_eq!(
        repo.load_operation(&plan.id).unwrap().reviewed_root,
        plan.reviewed_root
    );
    assert_eq!(
        operations.status(&plan.id).unwrap().state,
        OperationState::Prepared
    );
    assert_eq!(
        operations
            .reviewed_install(&plan.id, false)
            .await
            .unwrap_err()
            .code,
        "MCP_ACK_REQUIRED"
    );
    assert!(operations.reviewed_install(&plan.id, true).await.is_ok());
    let other = operations.prepare_install(request()).await.unwrap();
    assert_ne!(other.id, plan.id);
    assert_eq!(
        operations
            .reviewed_install(&other.id, false)
            .await
            .unwrap_err()
            .code,
        "MCP_ACK_REQUIRED"
    );
    drop(operations);
    let restarted = OperationService::new(services.session.clone(), services.targets.clone(), repo);
    assert_eq!(
        restarted
            .reviewed_install(&plan.id, true)
            .await
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
}

#[tokio::test]
async fn a_new_unmanaged_destination_is_reviewed_as_a_conflict_and_preserves_its_files() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let (_, _, operations) = setup(&server, root.path()).await;
    let first = operations.prepare_install(request()).await.unwrap();
    std::fs::create_dir_all(&first.target.path).unwrap();
    std::fs::write(first.target.path.join("user.txt"), b"keep").unwrap();
    assert_eq!(
        operations
            .reviewed_install(&first.id, true)
            .await
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
    let conflict = operations.prepare_install(request()).await.unwrap();
    assert!(!conflict.can_execute);
    assert!(conflict.conflict.is_some());
    assert_eq!(
        std::fs::read(conflict.target.path.join("user.txt")).unwrap(),
        b"keep"
    );
}

#[tokio::test]
async fn execution_review_rechecks_current_acl_and_immutable_release_even_when_prepared() {
    let denied = Arc::new(AtomicBool::new(false));
    let changed = Arc::new(AtomicBool::new(false));
    let (acl, metadata) = (denied.clone(), changed.clone());
    let server = TestServer::start(move |request| {
        let (acl, metadata) = (acl.clone(), metadata.clone());
        async move {
            let name = if request.path.ends_with("/auth/login") {
                "login"
            } else if request.path.contains("/releases/") {
                "release"
            } else {
                "detail"
            };
            let mut body = fixture(name);
            if name == "detail" && acl.load(Ordering::Acquire) {
                body["data"]["canDownload"] = false.into();
            }
            if name == "release" && metadata.load(Ordering::Acquire) {
                body["data"]["requiredMcps"] = serde_json::json!(["other-mcp"]);
            }
            Response::json(200, body)
        }
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let (_, _, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    denied.store(true, Ordering::Release);
    assert_eq!(
        operations
            .reviewed_install(&plan.id, true)
            .await
            .unwrap_err()
            .code,
        "DOWNLOAD_FORBIDDEN"
    );
    denied.store(false, Ordering::Release);
    changed.store(true, Ordering::Release);
    assert_eq!(
        operations
            .reviewed_install(&plan.id, true)
            .await
            .unwrap_err()
            .code,
        "RELEASE_CHANGED"
    );
    assert!(!plan.target.path.parent().unwrap().exists());
}

async fn package_server(format: &'static str, corrupt: bool) -> TestServer {
    use sha2::{Digest, Sha256};
    let bytes = if format == "zip" {
        include_bytes!("../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.zip")
            .to_vec()
    } else {
        include_bytes!("../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.7z")
            .to_vec()
    };
    let digest = hex::encode(Sha256::digest(&bytes));
    TestServer::start(move |request| {
        let (bytes, digest) = (bytes.clone(), digest.clone());
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer synthetic-access-one")
            );
            if request.path.ends_with("/package") {
                let mut body = bytes;
                if corrupt {
                    body[0] ^= 1;
                }
                return Response::binary(body);
            }
            if request.path.contains("/releases/") {
                let mut body = fixture("release");
                body["data"]["package"]["format"] = format.into();
                body["data"]["package"]["fileName"] = format!("synthetic.{format}").into();
                body["data"]["package"]["sizeBytes"] = bytes.len().into();
                body["data"]["package"]["sha256"] = digest.into();
                return Response::json(200, body);
            }
            Response::json(200, fixture("detail"))
        }
    })
    .await
}

#[tokio::test]
async fn both_formats_execute_through_authenticated_http_validation_cache_and_durable_commit() {
    for format in ["zip", "7z"] {
        let server = package_server(format, false).await;
        let root = tempfile::tempdir().unwrap();
        let (_, repo, operations) = setup(&server, root.path()).await;
        let plan = operations.prepare_install(request()).await.unwrap();
        assert_eq!(
            operations.execute(&plan.id, true).await.unwrap().state,
            OperationState::Completed
        );
        assert!(plan.target.path.join("SKILL.md").is_file());
        assert_eq!(repo.deployments().unwrap().len(), 1);
        assert_eq!(repo.list_library().unwrap().len(), 1);
        assert!(repo.pending_intents().unwrap().is_empty());
        assert_eq!(
            operations.execute(&plan.id, true).await.unwrap().state,
            OperationState::Completed
        );
    }
}

#[tokio::test]
async fn corrupted_download_and_preexecution_cancel_never_create_a_target_or_verified_cache() {
    let server = package_server("zip", true).await;
    let root = tempfile::tempdir().unwrap();
    let (_, repo, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    let failed = operations.execute(&plan.id, true).await.unwrap();
    assert_eq!(failed.state, OperationState::Failed);
    assert_eq!(failed.error.unwrap().code, "PACKAGE_INTEGRITY");
    assert!(!plan.target.path.exists());
    assert!(repo.list_library().unwrap().is_empty());
    let plan = operations.prepare_install(request()).await.unwrap();
    assert_eq!(
        operations.cancel(&plan.id).unwrap().state,
        OperationState::Cancelled
    );
    assert_eq!(
        operations.execute(&plan.id, true).await.unwrap().state,
        OperationState::Cancelled
    );
    assert!(!plan.target.path.exists());
    assert!(repo.deployments().unwrap().is_empty());
}

#[tokio::test]
async fn cancelling_a_running_download_refuses_double_execute_and_does_not_publish_cache() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (seen, wait) = (entered.clone(), release.clone());
    let server = TestServer::start(move |request| {
        let (seen, wait) = (seen.clone(), wait.clone());
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            if request.path.ends_with("/package") {
                seen.notify_one();
                wait.notified().await;
                return Response::binary(vec![]);
            }
            Response::json(
                200,
                fixture(if request.path.contains("/releases/") {
                    "release"
                } else {
                    "detail"
                }),
            )
        }
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let (_, repo, operations) = setup(&server, root.path()).await;
    let operations = Arc::new(operations);
    let plan = operations.prepare_install(request()).await.unwrap();
    let (running, id) = (operations.clone(), plan.id.clone());
    let task = tokio::spawn(async move { running.execute(&id, true).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    assert_eq!(
        operations.execute(&plan.id, true).await.unwrap_err().code,
        "OPERATION_BUSY"
    );
    assert_eq!(
        operations.cancel(&plan.id).unwrap().state,
        OperationState::Downloading
    );
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    release.notify_one();
    assert_eq!(result.state, OperationState::Cancelled);
    assert!(!plan.target.path.exists());
    assert!(repo.list_library().unwrap().is_empty());
    assert!(repo.deployments().unwrap().is_empty());
}

#[tokio::test]
async fn mcp_changes_require_a_new_review_instead_of_reusing_an_old_acknowledgment() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let (_, _, operations) = setup(&server, root.path()).await;
    let config = root.path().join("agent-home/.aipro/mcp.json");
    std::fs::write(
        &config,
        br#"{"mcpServers":{"jira-mcp":{"command":"synthetic"}}}"#,
    )
    .unwrap();
    let plan = operations.prepare_install(request()).await.unwrap();
    assert!(!plan.requires_mcp_ack);
    std::fs::write(&config, br#"{"mcpServers":{}}"#).unwrap();
    assert_eq!(
        operations
            .reviewed_install(&plan.id, true)
            .await
            .unwrap_err()
            .code,
        "REVIEW_REQUIRED"
    );
    let updated = operations.prepare_install(request()).await.unwrap();
    assert_ne!(updated.id, plan.id);
    assert!(updated.requires_mcp_ack);
    assert_eq!(
        operations
            .reviewed_install(&updated.id, false)
            .await
            .unwrap_err()
            .code,
        "MCP_ACK_REQUIRED"
    );
}

#[tokio::test]
async fn preparing_the_exact_managed_release_is_idempotent_and_local_edits_block_it() {
    let server = package_server("zip", false).await;
    let root = tempfile::tempdir().unwrap();
    let (_, repo, operations) = setup(&server, root.path()).await;
    let first = operations.prepare_install(request()).await.unwrap();
    assert_eq!(
        operations.execute(&first.id, true).await.unwrap().state,
        OperationState::Completed
    );
    let repeat = operations.prepare_install(request()).await.unwrap();
    assert!(repeat.can_execute, "{:?}", repeat.conflict);
    assert!(repeat.expected_fingerprint.is_some());
    assert_eq!(
        operations.execute(&repeat.id, true).await.unwrap().state,
        OperationState::Completed
    );
    assert_eq!(repo.deployments().unwrap().len(), 1);
    std::fs::write(first.target.path.join("local-addition.txt"), b"keep").unwrap();
    let changed = operations.prepare_install(request()).await.unwrap();
    assert!(!changed.can_execute);
    assert_eq!(
        std::fs::read(first.target.path.join("local-addition.txt")).unwrap(),
        b"keep"
    );
}

#[tokio::test]
async fn only_one_coordinator_may_recover_or_mutate_a_repository_at_a_time() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let (services, repo, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    let other = OperationService::new(
        services.session.clone(),
        services.targets.clone(),
        repo.clone(),
    );
    assert_eq!(
        other.prepare_install(request()).await.unwrap_err().code,
        "OPERATION_BUSY"
    );
    assert_eq!(
        repo.operation_status(&plan.id).unwrap().state,
        OperationState::Prepared
    );
    drop(other);
    drop(operations);
    let restarted = OperationService::new(
        services.session.clone(),
        services.targets.clone(),
        repo.clone(),
    );
    assert_eq!(
        restarted.status(&plan.id).unwrap().state,
        OperationState::Failed
    );
    assert_eq!(
        restarted.status(&plan.id).unwrap().error.unwrap().code,
        "REVIEW_REQUIRED"
    );
    assert!(restarted.prepare_install(request()).await.is_ok());
}

#[tokio::test]
async fn coordinator_startup_reconciles_durable_intents_before_accepting_new_operations() {
    use crate::sds::{
        journal::CommitIntent,
        packages::{prepare_package, PackageLimits},
    };
    let server = package_server("zip", false).await;
    let root = tempfile::tempdir().unwrap();
    let (services, repo, operations) = setup(&server, root.path()).await;
    let plan = operations.prepare_install(request()).await.unwrap();
    let archive = root.path().join("fixture.zip");
    std::fs::write(
        &archive,
        include_bytes!("../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.zip"),
    )
    .unwrap();
    let artifact = prepare_package(
        &archive,
        &plan.release,
        &root.path().join("verified"),
        &PackageLimits::default(),
    )
    .unwrap();
    repo.cache_release(&plan.context.profile_id, &plan.release, &artifact)
        .unwrap();
    let intent = CommitIntent::stage_install(&repo, &plan, &artifact).unwrap();
    drop(operations);
    let restarted = OperationService::new(
        services.session.clone(),
        services.targets.clone(),
        repo.clone(),
    );
    assert_eq!(
        restarted.status(&plan.id).unwrap().state,
        OperationState::Failed
    );
    assert_eq!(
        restarted.status(&plan.id).unwrap().error.unwrap().code,
        "INSTALL_INTERRUPTED"
    );
    assert!(intent.stage_path().is_dir());
    assert!(!plan.target.path.exists());
    assert!(repo.pending_intents().unwrap().is_empty());
    assert!(restarted.prepare_install(request()).await.is_ok());
}

#[tokio::test]
async fn runtime_services_initialize_and_recover_only_their_own_pilot_repository() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let services = SdsServices::new_fixture(root.path(), &server.base_url).unwrap();
    services
        .session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let plan = services
        .operations
        .prepare_install(request())
        .await
        .unwrap();
    assert!(root.path().join("pilot/pilot.sqlite3").is_file());
    drop(services);
    let restarted = SdsServices::new_fixture(root.path(), &server.base_url).unwrap();
    assert_eq!(
        restarted.operations.status(&plan.id).unwrap().state,
        OperationState::Failed
    );
    assert_eq!(
        restarted
            .operations
            .status(&plan.id)
            .unwrap()
            .error
            .unwrap()
            .code,
        "REVIEW_REQUIRED"
    );
    assert!(!plan.target.path.exists());
}

#[tokio::test]
async fn a_missing_managed_target_is_a_review_conflict_before_any_install_write() {
    let server = package_server("zip", false).await;
    let root = tempfile::tempdir().unwrap();
    let (_, repo, operations) = setup(&server, root.path()).await;
    let first = operations.prepare_install(request()).await.unwrap();
    assert_eq!(
        operations.execute(&first.id, true).await.unwrap().state,
        OperationState::Completed
    );
    assert!(first.target.path.starts_with(root.path()));
    std::fs::remove_dir_all(&first.target.path).unwrap();
    let missing = operations.prepare_install(request()).await.unwrap();
    assert!(!missing.can_execute);
    assert!(missing.conflict.is_some());
    assert!(operations.execute(&missing.id, true).await.is_err());
    assert!(!first.target.path.exists());
    assert!(repo.pending_intents().unwrap().is_empty());
    assert_eq!(repo.deployments().unwrap().len(), 1);
}

#[tokio::test]
async fn copied_cache_is_not_published_after_logout_or_cancellation() {
    use crate::sds::packages::{prepare_package, PackageLimits};
    for sign_out in [false, true] {
        let server = package_server("zip", false).await;
        let root = tempfile::tempdir().unwrap();
        let (services, repo, operations) = setup(&server, root.path()).await;
        let plan = operations.prepare_install(request()).await.unwrap();
        let archive = root.path().join("fixture.zip");
        std::fs::write(
            &archive,
            include_bytes!(
                "../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.zip"
            ),
        )
        .unwrap();
        let artifact = prepare_package(
            &archive,
            &plan.release,
            &root.path().join("verified"),
            &PackageLimits::default(),
        )
        .unwrap();
        let prepared = repo
            .prepare_cache(&plan.context.profile_id, &plan.release, &artifact)
            .unwrap();
        assert!(repo.list_library().unwrap().is_empty());
        let cancel = Arc::new(AtomicBool::new(!sign_out));
        if sign_out {
            services.session.logout().await.unwrap();
        }
        let error = operations
            .publish_cache(&plan.context, prepared, cancel)
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            if sign_out {
                "REVIEW_REQUIRED"
            } else {
                "CANCELLED"
            }
        );
        assert!(repo.list_library().unwrap().is_empty());
        assert_eq!(
            std::fs::read_dir(repo.root().join("releases"))
                .unwrap()
                .count(),
            0
        );
        assert!(!plan.target.path.exists());
    }
}
