use super::{contracts::ReleaseRef, error::SdsError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct KnownRelease {
    pub skill_id: String,
    pub release_id: String,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus { UpdateAvailable, UpToDate, Unavailable, UnknownRelease }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItem {
    pub skill_id: String,
    pub current_release_id: String,
    pub status: UpdateStatus,
    pub latest_release: Option<ReleaseRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub checked_at: String,
    pub items: Vec<UpdateItem>,
}

pub(crate) fn invalid_response() -> SdsError {
    SdsError::new("INVALID_RESPONSE", "Skills Store returned an invalid update check", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sds::{
        contracts::StoreProfile, credentials::CredentialVault, profile::Environment,
        session::SessionService, store_client::{HttpFuture, HttpRequest, HttpResponse, HttpTransport, StoreClient},
    };
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    struct Vault;
    impl CredentialVault for Vault {
        fn read(&self, _: &str) -> Result<Option<String>, SdsError> { Ok(None) }
        fn write(&self, _: &str, _: &str) -> Result<(), SdsError> { Ok(()) }
        fn clear(&self, _: &str) -> Result<(), SdsError> { Ok(()) }
    }
    #[derive(Default)]
    struct Transport {
        requests: Mutex<Vec<HttpRequest>>,
        mode: Mutex<String>,
        started: tokio::sync::Notify,
        resume: tokio::sync::Notify,
    }
    impl HttpTransport for Transport {
        fn send(&self, request: HttpRequest) -> HttpFuture<'_> {
            Box::pin(async move {
                self.requests.lock().unwrap().push(request.clone());
                if request.url.path().ends_with("/auth/login") {
                    return Ok(HttpResponse { status: 200, body: include_bytes!("../../../contracts/sds-desktop/v1/fixtures/auth-login.json").to_vec() });
                }
                if request.url.path().ends_with("/auth/logout") {
                    return Ok(HttpResponse {status:200,body:b"{}".to_vec()});
                }
                assert_eq!(request.url.path(), "/skm/api/v1/desktop/updates/check");
                assert_eq!(request.method, reqwest::Method::POST);
                let mode = self.mode.lock().unwrap().clone();
                if mode == "delayed" {
                    self.started.notify_one();
                    self.resume.notified().await;
                }
                if mode == "forbidden" { return Ok(HttpResponse {status: 403, body: vec![]}); }
                let items = request.body.unwrap()["items"].as_array().unwrap().clone();
                assert!(items.len() <= 100);
                let mut results: Vec<Value> = items.iter().map(|item| json!({
                    "skillId":item["skillId"], "currentReleaseId":item["releaseId"],
                    "status":"update_available", "latestRelease":{"id":"release-new","sequence":2,"version":"2.0"}
                })).collect();
                match mode.as_str() {
                    "missing" => { results.pop(); },
                    "duplicate" => { results.push(results[0].clone()); },
                    "mismatch" => { results[0]["skillId"] = json!("another-skill"); },
                    "leaked" => { results[0]["status"] = json!("unavailable"); },
                    "missing_latest" => { results[0]["latestRelease"] = Value::Null; },
                    "invalid_sequence" => { results[0]["latestRelease"]["sequence"] = json!(0); },
                    "bad_time" => {},
                    _ => {}
                }
                let time = if mode == "bad_time" {"yesterday"} else {"2026-10-17T14:00:00Z"};
                Ok(HttpResponse { status:200, body:serde_json::to_vec(&json!({"success":true,"data":{"checkedAt":time,"items":results}})).unwrap() })
            })
        }
    }
    async fn client(mode: &str) -> (StoreClient, Arc<Transport>) {
        let transport = Arc::new(Transport::default());
        *transport.mode.lock().unwrap() = mode.into();
        let session = Arc::new(SessionService::new(Environment::Development,
            Some(StoreProfile::new("http://127.0.0.1:8080/skm", Environment::Development).unwrap()),
            transport.clone(), Arc::new(Vault)));
        session.login("employee@example.invalid", "synthetic-password").await.unwrap();
        transport.requests.lock().unwrap().clear();
        (StoreClient::new(session), transport)
    }
    fn known(index: usize) -> KnownRelease {
        KnownRelease {skill_id:format!("skill-{index}"),release_id:"release-old".into()}
    }
    #[test]
    fn contract_fixture_domain_shapes() {
        let fixture: Value=serde_json::from_str(include_str!("../../../contracts/sds-desktop/v1/fixtures/updates.json")).unwrap();
        let check: UpdateCheck=serde_json::from_value(fixture["data"].clone()).unwrap();
        assert_eq!(check.items.len(),4);
        assert_eq!(check.items[2].status,UpdateStatus::Unavailable);
        assert!(check.items[2].latest_release.is_none());
        println!("SDS_CONTRACT_UPDATE_DOMAIN={}",serde_json::to_string(&check).unwrap());
    }
    #[tokio::test]
    async fn empty_check_does_not_send_a_request() {
        let (client, transport) = client("").await;
        assert!(client.check_updates(&[]).await.unwrap().items.is_empty());
        assert!(transport.requests.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn check_deduplicates_chunks_and_never_downloads_packages() {
        let (client, transport) = client("").await;
        let mut input: Vec<_> = (0..101).map(known).collect();
        input.push(known(0));
        let check = client.check_updates(&input).await.unwrap();
        assert_eq!(check.items.len(),101);
        assert_eq!(check.items[0].status,UpdateStatus::UpdateAvailable);
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests.len(),2);
        assert_eq!(requests[0].body.as_ref().unwrap()["items"].as_array().unwrap().len(),100);
        assert_eq!(requests[1].body.as_ref().unwrap()["items"].as_array().unwrap().len(),1);
        assert!(!serde_json::to_string(requests[0].body.as_ref().unwrap()).unwrap().contains("target"));
    }
    #[tokio::test]
    async fn same_skill_different_current_releases_remain_separate() {
        let (client, _) = client("").await;
        let mut second=known(0); second.release_id="release-older".into();
        assert_eq!(client.check_updates(&[known(0),second]).await.unwrap().items.len(),2);
    }
    #[tokio::test]
    async fn invalid_pairs_and_denied_metadata_are_never_reported_as_current() {
        for mode in ["missing","duplicate","mismatch","leaked","missing_latest","invalid_sequence","bad_time"] {
            let (client, _) = client(mode).await;
            assert_eq!(client.check_updates(&[known(0)]).await.unwrap_err().code,"INVALID_RESPONSE","{mode}");
        }
        let (client, _) = client("forbidden").await;
        assert_eq!(client.check_updates(&[known(0)]).await.unwrap_err().code,"ACCESS_DENIED");
    }
    #[tokio::test]
    async fn invalid_ids_fail_before_network() {
        let (client,transport)=client("").await;
        assert_eq!(client.check_updates(&[KnownRelease{skill_id:String::new(),release_id:"r".into()}]).await.unwrap_err().code,"INVALID_REQUEST");
        assert!(transport.requests.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn late_response_after_logout_cannot_publish_update_metadata() {
        let transport=Arc::new(Transport::default());
        *transport.mode.lock().unwrap()="delayed".into();
        let session=Arc::new(SessionService::new(Environment::Development,
            Some(StoreProfile::new("http://127.0.0.1:8080/skm",Environment::Development).unwrap()),transport.clone(),Arc::new(Vault)));
        session.login("employee@example.invalid","synthetic-password").await.unwrap();
        let client=StoreClient::new(session.clone());
        let checking=tokio::spawn(async move {client.check_updates(&[known(0)]).await});
        tokio::time::timeout(std::time::Duration::from_secs(5),transport.started.notified()).await.unwrap();
        session.logout().await.unwrap();
        transport.resume.notify_one();
        assert_eq!(checking.await.unwrap().unwrap_err().code,"REVIEW_REQUIRED");
    }
}
