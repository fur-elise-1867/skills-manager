use super::support::{Response, TestServer};
use crate::sds::store_client::join_api;
use crate::sds::store_client::HttpRequest;
use crate::sds::{
    contracts::{SessionStatus, StoreProfile},
    credentials::CredentialVault,
    error::SdsError,
    session::SessionService,
    store_client::ReqwestTransport,
};
use crate::sds::{profile::Environment, store_client::canonical_base_url};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

#[derive(Default)]
struct MemoryVault(Mutex<HashMap<String, String>>);
impl CredentialVault for MemoryVault {
    fn read(&self, id: &str) -> Result<Option<String>, SdsError> {
        Ok(self.0.lock().unwrap().get(id).cloned())
    }
    fn write(&self, id: &str, value: &str) -> Result<(), SdsError> {
        self.0.lock().unwrap().insert(id.into(), value.into());
        Ok(())
    }
    fn clear(&self, id: &str) -> Result<(), SdsError> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
}

#[tokio::test]
async fn real_login_payload_maps_identity_without_expires_in_and_never_exports_tokens() {
    let server = TestServer::start(|request| async move {
        assert_eq!(request.path, "/skm/api/v1/auth/login");
        assert_eq!(request.body["email"], "employee@example.invalid");
        Response::json(
            200,
            serde_json::from_str(include_str!(
                "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
            ))
            .unwrap(),
        )
    })
    .await;
    let profile = StoreProfile::new(&server.base_url, Environment::Development).unwrap();
    let service = SessionService::new(
        Environment::Development,
        Some(profile),
        Arc::new(ReqwestTransport::new().unwrap()),
        Arc::new(MemoryVault::default()),
    );
    let view = service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    assert_eq!(view.status, SessionStatus::Authenticated);
    assert_eq!(view.user.as_ref().unwrap().id, "1001");
    assert_eq!(view.user.as_ref().unwrap().display_name, "Example Employee");
    let ipc = serde_json::to_string(&view).unwrap();
    assert!(!ipc.contains("synthetic-") && !ipc.contains("Token"));
    assert_eq!(service.status().await.unwrap(), view);
}

#[tokio::test]
async fn unauthorized_request_refreshes_once_but_forbidden_does_not_refresh() {
    for status in [401, 403] {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let seen = refreshes.clone();
        let server = TestServer::start(move |request| {
            let seen = seen.clone();
            async move {
                if request.path.ends_with("/auth/login") { return Response::json(200, serde_json::from_str(include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-login.json")).unwrap()); }
                if request.path.ends_with("/auth/refresh") {
                    assert_eq!(request.body["refreshToken"], "synthetic-refresh-one");
                    seen.fetch_add(1, Ordering::SeqCst);
                    return Response::json(200, serde_json::from_str(include_str!("../../../../contracts/sds-desktop/v1/fixtures/auth-refresh.json")).unwrap());
                }
                if status == 401 && request.authorization.as_deref() == Some("Bearer synthetic-access-two") { Response::json(200, serde_json::json!({"success":true,"data":{"id":"visible"}})) }
                else { Response::json(status, serde_json::json!({"success":false,"message":"synthetic-password synthetic-refresh-one"})) }
            }
        }).await;
        let profile = StoreProfile::new(&server.base_url, Environment::Development).unwrap();
        let service = SessionService::new(
            Environment::Development,
            Some(profile),
            Arc::new(ReqwestTransport::new().unwrap()),
            Arc::new(MemoryVault::default()),
        );
        service
            .login("employee@example.invalid", "synthetic-password")
            .await
            .unwrap();
        let result = service
            .request(HttpRequest::get(
                join_api(&server.base_url, "api/v1/desktop/probe").unwrap(),
            ))
            .await;
        if status == 401 {
            assert_eq!(result.unwrap().status, 200);
            assert_eq!(refreshes.load(Ordering::SeqCst), 1);
        } else {
            let error = result.err().unwrap();
            assert_eq!(error.code, "ACCESS_DENIED");
            assert!(!format!("{error:?}").contains("synthetic-"));
            assert_eq!(refreshes.load(Ordering::SeqCst), 0);
        }
        assert_eq!(
            service.status().await.unwrap().status,
            SessionStatus::Authenticated
        );
    }
}

#[tokio::test]
async fn logout_clears_identity_and_saved_credentials_even_when_revoke_fails() {
    let server = TestServer::start(|request| async move {
        if request.path.ends_with("/auth/login") {
            Response::json(
                200,
                serde_json::from_str(include_str!(
                    "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                ))
                .unwrap(),
            )
        } else {
            Response::json(503, serde_json::json!({"success":false}))
        }
    })
    .await;
    let profile = StoreProfile::new(&server.base_url, Environment::Development).unwrap();
    let profile_id = profile.id.clone();
    let vault = Arc::new(MemoryVault::default());
    let service = SessionService::new(
        Environment::Development,
        Some(profile),
        Arc::new(ReqwestTransport::new().unwrap()),
        vault.clone(),
    );
    service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    service.logout().await.unwrap();
    let view = service.status().await.unwrap();
    assert_eq!(view.status, SessionStatus::SignedOut);
    assert!(view.user.is_none());
    assert!(vault.read(&profile_id).unwrap().is_none());
}

#[tokio::test]
async fn a_late_refresh_cannot_restore_logout_or_a_replaced_connection() {
    for replace_connection in [false, true] {
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let (started_server, release_server) = (started.clone(), release.clone());
        let server = TestServer::start(move |request| {
            let (started, release) = (started_server.clone(), release_server.clone());
            async move {
                if request.path.ends_with("/auth/login") {
                    return Response::json(
                        200,
                        serde_json::from_str(include_str!(
                            "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                        ))
                        .unwrap(),
                    );
                }
                if request.path.ends_with("/auth/logout") {
                    return Response::json(200, serde_json::json!({"success":true,"data":null}));
                }
                if request.path.ends_with("/auth/refresh") {
                    started.notify_one();
                    release.notified().await;
                    return Response::json(
                        200,
                        serde_json::from_str(include_str!(
                            "../../../../contracts/sds-desktop/v1/fixtures/auth-refresh.json"
                        ))
                        .unwrap(),
                    );
                }
                if request.authorization.as_deref() == Some("Bearer synthetic-access-two") {
                    Response::json(200, serde_json::json!({"success":true,"data":{}}))
                } else {
                    Response::json(401, serde_json::json!({"success":false}))
                }
            }
        })
        .await;
        let profile = StoreProfile::new(&server.base_url, Environment::Development).unwrap();
        let id = profile.id.clone();
        let vault = Arc::new(MemoryVault::default());
        let service = Arc::new(SessionService::new(
            Environment::Development,
            Some(profile),
            Arc::new(ReqwestTransport::new().unwrap()),
            vault.clone(),
        ));
        service
            .login("employee@example.invalid", "synthetic-password")
            .await
            .unwrap();
        let pending_service = service.clone();
        let url = join_api(&server.base_url, "api/v1/desktop/probe").unwrap();
        let pending =
            tokio::spawn(async move { pending_service.request(HttpRequest::get(url)).await });
        tokio::time::timeout(std::time::Duration::from_secs(3), started.notified())
            .await
            .unwrap();
        if replace_connection {
            service
                .set_connection("http://127.0.0.1:9/other")
                .await
                .unwrap();
        } else {
            service.logout().await.unwrap();
        }
        release.notify_one();
        let error = pending
            .await
            .unwrap()
            .err()
            .expect("stale account response must be rejected");
        assert_eq!(error.code, "REVIEW_REQUIRED");
        let view = service.status().await.unwrap();
        assert_eq!(view.status, SessionStatus::SignedOut);
        assert!(view.user.is_none());
        assert!(vault.read(&id).unwrap().is_none());
    }
}

#[tokio::test]
async fn concurrent_unauthorized_requests_share_one_refresh() {
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let refreshes = Arc::new(AtomicUsize::new(0));
    let count = refreshes.clone();
    let server = TestServer::start(move |request| {
        let barrier = barrier.clone();
        let count = count.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                    ))
                    .unwrap(),
                );
            }
            if request.path.ends_with("/auth/refresh") {
                count.fetch_add(1, Ordering::SeqCst);
                return Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-refresh.json"
                    ))
                    .unwrap(),
                );
            }
            if request.authorization.as_deref() == Some("Bearer synthetic-access-one") {
                barrier.wait().await;
                Response::json(401, serde_json::json!({"success":false}))
            } else {
                Response::json(200, serde_json::json!({"success":true,"data":{}}))
            }
        }
    })
    .await;
    let service = SessionService::new(
        Environment::Development,
        Some(StoreProfile::new(&server.base_url, Environment::Development).unwrap()),
        Arc::new(ReqwestTransport::new().unwrap()),
        Arc::new(MemoryVault::default()),
    );
    service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let url = join_api(&server.base_url, "api/v1/desktop/probe").unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        tokio::join!(
            service.request(HttpRequest::get(url.clone())),
            service.request(HttpRequest::get(url))
        )
    })
    .await
    .unwrap();
    assert_eq!(result.0.unwrap().status, 200);
    assert_eq!(result.1.unwrap().status, 200);
    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn network_failure_retains_offline_identity_but_rejected_refresh_expires_it() {
    let phase = Arc::new(AtomicUsize::new(0));
    let observed = phase.clone();
    let server = TestServer::start(move |request| {
        let phase = observed.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                    ))
                    .unwrap(),
                );
            }
            if phase.load(Ordering::SeqCst) == 0 {
                Response::json(0, serde_json::Value::Null)
            } else {
                Response::json(401, serde_json::json!({"success":false}))
            }
        }
    })
    .await;
    let profile = StoreProfile::new(&server.base_url, Environment::Development).unwrap();
    let id = profile.id.clone();
    let vault = Arc::new(MemoryVault::default());
    let service = SessionService::new(
        Environment::Development,
        Some(profile),
        Arc::new(ReqwestTransport::new().unwrap()),
        vault.clone(),
    );
    service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let url = join_api(&server.base_url, "api/v1/desktop/probe").unwrap();
    assert_eq!(
        service
            .request(HttpRequest::get(url.clone()))
            .await
            .err()
            .unwrap()
            .code,
        "STORE_UNAVAILABLE"
    );
    let offline = service.status().await.unwrap();
    assert_eq!(offline.status, SessionStatus::Offline);
    assert_eq!(offline.user.unwrap().id, "1001");
    phase.store(1, Ordering::SeqCst);
    assert_eq!(
        service
            .request(HttpRequest::get(url))
            .await
            .err()
            .unwrap()
            .code,
        "SESSION_EXPIRED"
    );
    assert_eq!(
        service.status().await.unwrap().status,
        SessionStatus::Expired
    );
    assert!(vault.read(&id).unwrap().is_none());
}

#[tokio::test]
async fn protected_requests_and_redirects_never_send_tokens_to_another_origin() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let foreign = TestServer::start(move |_| {
        let observed = observed.clone();
        async move {
            observed.fetch_add(1, Ordering::SeqCst);
            Response::json(200, serde_json::json!({"success":true,"data":{}}))
        }
    })
    .await;
    let destination = format!("{}/api/v1/desktop/probe", foreign.base_url);
    let server = TestServer::start(move |request| {
        let destination = destination.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                    ))
                    .unwrap(),
                )
            } else {
                Response::redirect(destination)
            }
        }
    })
    .await;
    let service = SessionService::new(
        Environment::Development,
        Some(StoreProfile::new(&server.base_url, Environment::Development).unwrap()),
        Arc::new(ReqwestTransport::new().unwrap()),
        Arc::new(MemoryVault::default()),
    );
    service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    assert_eq!(
        service
            .request(HttpRequest::get(
                join_api(&foreign.base_url, "api/v1/desktop/probe").unwrap()
            ))
            .await
            .err()
            .unwrap()
            .code,
        "INVALID_CONFIGURATION"
    );
    assert_eq!(
        service
            .request(HttpRequest::get(
                join_api(&server.base_url, "api/v1/desktop/probe").unwrap()
            ))
            .await
            .err()
            .unwrap()
            .code,
        "INVALID_RESPONSE"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn saved_session_restores_identity_and_rotates_refresh_without_another_password() {
    let server = TestServer::start(|request| async move {
        if request.path.ends_with("/auth/login") {
            Response::json(
                200,
                serde_json::from_str(include_str!(
                    "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                ))
                .unwrap(),
            )
        } else {
            Response::json(
                200,
                serde_json::from_str(include_str!(
                    "../../../../contracts/sds-desktop/v1/fixtures/auth-refresh.json"
                ))
                .unwrap(),
            )
        }
    })
    .await;
    let profile = StoreProfile::new(&server.base_url, Environment::Development).unwrap();
    let vault = Arc::new(MemoryVault::default());
    let first = SessionService::new(
        Environment::Development,
        Some(profile.clone()),
        Arc::new(ReqwestTransport::new().unwrap()),
        vault.clone(),
    );
    first
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    let restored = SessionService::new(
        Environment::Development,
        Some(profile),
        Arc::new(ReqwestTransport::new().unwrap()),
        vault,
    );
    let view = restored.status().await.unwrap();
    assert_eq!(view.status, SessionStatus::Authenticated);
    assert_eq!(view.user.unwrap().id, "1001");
}

#[tokio::test]
async fn connection_settings_survive_restart_through_the_service_boundary() {
    let root = tempfile::tempdir().unwrap();
    let services =
        crate::sds::commands::SdsServices::new(root.path(), Environment::Development).unwrap();
    let saved = services
        .set_connection("http://127.0.0.1:18321/skm/")
        .await
        .unwrap();
    let reopened =
        crate::sds::commands::SdsServices::new(root.path(), Environment::Development).unwrap();
    let restored = reopened
        .session
        .connection()
        .await
        .expect("configured Store must survive restart");
    assert_eq!(restored.base_url, "http://127.0.0.1:18321/skm");
    assert_eq!(restored.id, saved.id);
}

#[tokio::test]
async fn credential_failure_never_exports_secret_diagnostics_or_falls_back_to_plaintext() {
    struct DeniedVault;
    impl CredentialVault for DeniedVault {
        fn read(&self, _: &str) -> Result<Option<String>, SdsError> {
            Ok(None)
        }
        fn write(&self, _: &str, _: &str) -> Result<(), SdsError> {
            Err(SdsError::new(
                "CREDENTIAL_UNAVAILABLE",
                "synthetic-password synthetic-refresh-one",
                false,
            ))
        }
        fn clear(&self, _: &str) -> Result<(), SdsError> {
            Ok(())
        }
    }
    let server = TestServer::start(|_| async {
        Response::json(
            200,
            serde_json::from_str(include_str!(
                "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
            ))
            .unwrap(),
        )
    })
    .await;
    let service = SessionService::new(
        Environment::Development,
        Some(StoreProfile::new(&server.base_url, Environment::Development).unwrap()),
        Arc::new(ReqwestTransport::new().unwrap()),
        Arc::new(DeniedVault),
    );
    let error = service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap_err();
    assert_eq!(error.code, "CREDENTIAL_UNAVAILABLE");
    assert!(!serde_json::to_string(&error)
        .unwrap()
        .contains("synthetic-"));
    let view = service.status().await.unwrap();
    assert_eq!(view.status, SessionStatus::SignedOut);
    assert!(view.user.is_none());
}

#[cfg(windows)]
#[test]
#[ignore = "Opt-in integration against Windows Credential Manager with a temporary synthetic entry"]
fn windows_credential_vault_round_trip_is_scoped_and_writes_only_a_signout_marker() {
    use crate::sds::credentials::WindowsCredentialVault;
    use sha2::{Digest, Sha256};
    struct Cleanup<'a>(&'a dyn CredentialVault, &'a str);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            let _ = self.0.clear(self.1);
        }
    }
    let root = tempfile::tempdir().unwrap();
    let id = hex::encode(Sha256::digest(
        format!("native-vault-test:{}", uuid::Uuid::new_v4()).as_bytes(),
    ));
    let vault = WindowsCredentialVault::new(root.path());
    let _cleanup = Cleanup(&vault, &id);
    vault.write(&id, "synthetic-vault-secret").unwrap();
    assert_eq!(
        vault.read(&id).unwrap().as_deref(),
        Some("synthetic-vault-secret")
    );
    vault.clear(&id).unwrap();
    assert!(vault.read(&id).unwrap().is_none());
    for file in std::fs::read_dir(root.path().join("credential-state")).unwrap() {
        assert_eq!(std::fs::read(file.unwrap().path()).unwrap(), b"signed-out");
    }
}

#[tokio::test]
async fn existing_store_bad_credentials_http400_maps_to_auth_failure_without_enumeration() {
    for code in ["USER_NOT_FOUND", "INVALID_PASSWORD"] {
        let server = TestServer::start(move |_| async move { Response::json(400, serde_json::json!({"success":false,"errorCode":code,"message":"synthetic-password"})) }).await;
        let service = SessionService::new(
            Environment::Development,
            Some(StoreProfile::new(&server.base_url, Environment::Development).unwrap()),
            Arc::new(ReqwestTransport::new().unwrap()),
            Arc::new(MemoryVault::default()),
        );
        let error = service
            .login("employee@example.invalid", "synthetic-password")
            .await
            .unwrap_err();
        assert_eq!(error.code, "AUTH_FAILED");
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("synthetic-"));
    }
}

#[tokio::test]
async fn repeated_unauthorized_after_refresh_expires_without_a_second_refresh() {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let server = TestServer::start(move |request| {
        let observed = observed.clone();
        async move {
            if request.path.ends_with("/auth/login") {
                return Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                    ))
                    .unwrap(),
                );
            }
            if request.path.ends_with("/auth/refresh") {
                observed.fetch_add(1, Ordering::SeqCst);
                return Response::json(
                    200,
                    serde_json::from_str(include_str!(
                        "../../../../contracts/sds-desktop/v1/fixtures/auth-refresh.json"
                    ))
                    .unwrap(),
                );
            }
            Response::json(401, serde_json::json!({"success":false}))
        }
    })
    .await;
    let service = SessionService::new(
        Environment::Development,
        Some(StoreProfile::new(&server.base_url, Environment::Development).unwrap()),
        Arc::new(ReqwestTransport::new().unwrap()),
        Arc::new(MemoryVault::default()),
    );
    service
        .login("employee@example.invalid", "synthetic-password")
        .await
        .unwrap();
    assert_eq!(
        service
            .request(HttpRequest::get(
                join_api(&server.base_url, "api/v1/desktop/probe").unwrap()
            ))
            .await
            .err()
            .unwrap()
            .code,
        "SESSION_EXPIRED"
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(
        service.status().await.unwrap().status,
        SessionStatus::Expired
    );
}

#[test]
fn store_routes_keep_context_path_for_auth_and_package_payloads() {
    for route in [
        "api/v1/auth/login",
        "/api/v1/desktop/skills/1/releases/2/package",
    ] {
        let url = join_api("http://127.0.0.1:8080/skm", route).unwrap();
        assert_eq!(
            url.path(),
            format!("/skm/{}", route.trim_start_matches('/'))
        );
    }
}

#[test]
fn configured_store_url_enforces_transport_and_rejects_credentials_or_traversal() {
    assert_eq!(
        canonical_base_url("https://STORE.example.invalid/skm/", Environment::Internal).unwrap(),
        "https://store.example.invalid/skm"
    );
    assert!(canonical_base_url("http://127.0.0.1:8080/skm", Environment::Development).is_ok());
    assert!(canonical_base_url("http://[::1]:8080/skm", Environment::Development).is_ok());
    for url in [
        "http://store.example.invalid/skm",
        "https://name:password@store.example.invalid/skm",
        "https://store.example.invalid/skm?q=x",
        "https://store.example.invalid/skm#fragment",
        "https://store.example.invalid/a/../skm",
        "https://store.example.invalid/%2e%2e/skm",
    ] {
        assert!(
            canonical_base_url(url, Environment::Development).is_err(),
            "unsafe base accepted: {url}"
        );
    }
    assert!(canonical_base_url("http://127.0.0.1:8080/skm", Environment::Internal).is_err());
}

#[test]
fn remote_routes_cannot_override_origin_or_escape_store_context() {
    for route in [
        "https://other.invalid/api/v1/skills",
        "//other.invalid/api/v1/skills",
        "api/v1/../auth/login",
        "api/v1/%2e%2e/auth",
        "api/v1/a%2fb",
        "api/v1/a\\b",
        "api/v1/auth/login?password=secret",
        "/../api/v1/auth/login",
    ] {
        assert!(
            join_api("http://127.0.0.1:8080/skm", route).is_err(),
            "unsafe route accepted: {route}"
        );
    }
}
