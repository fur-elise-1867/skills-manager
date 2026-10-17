use super::support::{Response, TestServer};
use crate::sds::{
    contracts::{CatalogMode, CatalogQuery, StoreProfile},
    credentials::CredentialVault,
    error::SdsError,
    profile::Environment,
    session::SessionService,
    store_client::{ReqwestTransport, StoreClient},
};
use std::sync::{Arc, Mutex};

struct VolatileVault;
impl CredentialVault for VolatileVault {
    fn read(&self, _: &str) -> Result<Option<String>, SdsError> {
        Ok(None)
    }
    fn write(&self, _: &str, _: &str) -> Result<(), SdsError> {
        Ok(())
    }
    fn clear(&self, _: &str) -> Result<(), SdsError> {
        Ok(())
    }
}
fn fixture(name: &str) -> serde_json::Value {
    serde_json::from_str(match name {
        "login" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"),
        "catalog" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/catalog.json"),
        "detail" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/detail.json"),
        "release" => include_str!("../../../../contracts/sds-desktop/v1/fixtures/release.json"),
        _ => include_str!("../../../../contracts/sds-desktop/v1/fixtures/taxonomy.json"),
    })
    .unwrap()
}
async fn client(server: &TestServer) -> (Arc<SessionService>, StoreClient) {
    let session = Arc::new(SessionService::new(
        Environment::Development,
        Some(StoreProfile::new(&server.base_url, Environment::Development).unwrap()),
        Arc::new(ReqwestTransport::new().unwrap()),
        Arc::new(VolatileVault),
    ));
    session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let client = StoreClient::new(session.clone());
    (session, client)
}

#[tokio::test]
async fn catalog_gateway_combines_filters_preserves_context_and_maps_flat_detail() {
    let paths = Arc::new(Mutex::new(Vec::new()));
    let observed = paths.clone();
    let server = TestServer::start(move |request| {
        let observed = observed.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer synthetic-access-one")
            );
            observed.lock().unwrap().push(request.path.clone());
            let name = if request.path.contains("/releases/") {
                "release"
            } else if request.path.ends_with("/skills/skill-123") {
                "detail"
            } else if request.path.ends_with("/taxonomy") {
                "taxonomy"
            } else {
                "catalog"
            };
            Response::json(200, fixture(name))
        }
    })
    .await;
    let (_, client) = client(&server).await;
    let taxonomy = client.taxonomy().await.unwrap();
    assert_eq!(taxonomy.sources[0].label, "Quality");
    let result = client
        .search(CatalogQuery {
            q: " Jira & report ".into(),
            mode: CatalogMode::AllTime,
            source_id: Some("quality".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(result.total_items, 1);
    assert_eq!(result.items[0].metadata.name, "Jira Weekly Report");
    assert!(result.items[0].metadata.compatible_agents.is_none());
    let detail = client.detail("skill-123").await.unwrap();
    assert_eq!(detail.summary.metadata.id, "skill-123");
    assert_eq!(
        detail.latest_release.unwrap().required_mcps.unwrap(),
        vec!["jira-mcp"]
    );
    let release = client.release("skill-123", "rel-003").await.unwrap();
    assert_eq!(release.id, "rel-003");
    let paths = paths.lock().unwrap();
    assert!(paths.iter().any(|path| path == "/skm/api/v1/desktop/skills?q=Jira+%26+report&mode=all_time&sourceId=quality&page=1&pageSize=20"), "unexpected paths: {paths:?}");
}

#[tokio::test]
async fn catalog_rejects_invalid_pagination_before_http_and_keeps_default_contract() {
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = count.clone();
    let server = TestServer::start(move |request| {
        let observed = observed.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Response::json(200, fixture("catalog"))
        }
    })
    .await;
    let (_, client) = client(&server).await;
    for (page, page_size) in [(0, 20), (1, 0), (1, 101)] {
        assert_eq!(
            client
                .search(CatalogQuery {
                    page,
                    page_size,
                    ..Default::default()
                })
                .await
                .unwrap_err()
                .code,
            "INVALID_REQUEST"
        );
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
    let query: CatalogQuery = serde_json::from_str("{}").unwrap();
    assert_eq!(
        (query.page, query.page_size, query.mode),
        (1, 20, CatalogMode::AllTime)
    );
}

#[tokio::test]
async fn catalog_distinguishes_unavailable_modes_expired_sessions_empty_results_and_outage() {
    for (status, expected) in [
        (422, "CATALOG_MODE_UNAVAILABLE"),
        (401, "SESSION_EXPIRED"),
        (503, "STORE_UNAVAILABLE"),
    ] {
        let server = TestServer::start(move |request| async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            Response::json(
                status,
                serde_json::json!({"success":false,"message":"synthetic-access-one"}),
            )
        })
        .await;
        let (_, client) = client(&server).await;
        let error = client.search(CatalogQuery::default()).await.unwrap_err();
        assert_eq!(error.code, expected);
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("synthetic-"));
    }
    let server = TestServer::start(|request| async move {
        Response::json(200, if request.path.ends_with("/auth/login") { fixture("login") } else { serde_json::json!({"success":true,"data":{"items":[],"page":1,"pageSize":20,"totalItems":0}}) })
    }).await;
    let (_, client) = client(&server).await;
    assert!(client
        .search(CatalogQuery::default())
        .await
        .unwrap()
        .items
        .is_empty());
}

#[tokio::test]
async fn missing_release_and_compatibility_remain_unknown_instead_of_verified_or_installed() {
    let server = TestServer::start(|request| async move {
        if request.path.ends_with("/auth/login") {
            return Response::json(200, fixture("login"));
        }
        let mut detail = fixture("detail");
        detail["data"]
            .as_object_mut()
            .unwrap()
            .remove("latestRelease");
        detail["data"]
            .as_object_mut()
            .unwrap()
            .remove("compatibleAgents");
        Response::json(200, detail)
    })
    .await;
    let (_, client) = client(&server).await;
    let detail = client.detail("skill-123").await.unwrap();
    assert!(detail.latest_release.is_none());
    assert!(detail.summary.latest_release.is_none());
    assert!(detail.summary.metadata.compatible_agents.is_none());
}

#[tokio::test]
async fn captured_account_context_cannot_send_after_logout_and_new_signin() {
    use crate::sds::store_client::{join_api, HttpRequest};
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = count.clone();
    let server = TestServer::start(move |request| {
        let observed = observed.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            if request.path.contains("/desktop/") {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Response::json(200, serde_json::json!({"success":true,"data":null}))
        }
    })
    .await;
    let (session, _) = client(&server).await;
    let (_, context) = session.scope().await.unwrap();
    session.logout().await.unwrap();
    session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let request = HttpRequest::get(join_api(&server.base_url, "api/v1/desktop/taxonomy").unwrap());
    assert_eq!(
        session
            .request_for(&context, request)
            .await
            .err()
            .unwrap()
            .code,
        "REVIEW_REQUIRED"
    );
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn catalog_accepts_unicode_keywords_within_character_limit_and_page_size_100() {
    let server = TestServer::start(|request| async move {
        if request.path.ends_with("/auth/login") {
            return Response::json(200, fixture("login"));
        }
        let mut page = fixture("catalog");
        page["data"]["pageSize"] = 100.into();
        Response::json(200, page)
    })
    .await;
    let (_, client) = client(&server).await;
    let page = client
        .search(CatalogQuery {
            q: "報".repeat(500),
            page_size: 100,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.page_size, 100);
}

#[tokio::test]
async fn a_late_catalog_response_is_rejected_after_account_generation_changes() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let observed = entered.clone();
    let wait = release.clone();
    let server = TestServer::start(move |request| {
        let observed = observed.clone();
        let wait = wait.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(200, fixture("login"));
            }
            if request.path.contains("/desktop/") {
                observed.notify_one();
                wait.notified().await;
                return Response::json(200, fixture("catalog"));
            }
            Response::json(200, serde_json::json!({"success":true,"data":null}))
        }
    })
    .await;
    let (session, client) = client(&server).await;
    let request = tokio::spawn(async move { client.search(CatalogQuery::default()).await });
    entered.notified().await;
    session.logout().await.unwrap();
    session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    release.notify_one();
    assert_eq!(request.await.unwrap().unwrap_err().code, "REVIEW_REQUIRED");
}

#[tokio::test]
#[cfg(debug_assertions)]
async fn debug_fixture_services_are_loopback_locked_ephemeral_and_do_not_persist_credentials() {
    use crate::sds::commands::SdsServices;
    let root = tempfile::tempdir().unwrap();
    assert!(SdsServices::new_fixture(root.path(), "https://store.example.invalid/skm").is_err());
    let server = TestServer::start(|request| async move {
        Response::json(
            200,
            if request.path.ends_with("/auth/login") {
                fixture("login")
            } else {
                fixture("catalog")
            },
        )
    })
    .await;
    let services = SdsServices::new_fixture(root.path(), &server.base_url).unwrap();
    services
        .session
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    assert_eq!(
        services.session.status().await.unwrap().status,
        crate::sds::contracts::SessionStatus::Authenticated
    );
    assert!(services
        .set_connection("https://store.example.invalid/skm")
        .await
        .is_err());
    assert_eq!(
        services.connection().await.unwrap().unwrap().base_url,
        server.base_url
    );
    let restarted = SdsServices::new_fixture(root.path(), &server.base_url).unwrap();
    assert_eq!(
        restarted.session.status().await.unwrap().status,
        crate::sds::contracts::SessionStatus::SignedOut
    );
    let entries: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, [std::ffi::OsString::from("agent-home")]);
    assert_eq!(
        std::fs::read_dir(root.path().join("agent-home/.aipro"))
            .unwrap()
            .count(),
        0
    );
}
