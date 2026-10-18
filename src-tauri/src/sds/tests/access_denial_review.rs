//! The IPC detail command delegates to OperationService::skill_detail. These tests
//! deliberately use that same entrypoint, not StoreClient::detail directly.
use super::support::{Response, TestServer};
use crate::sds::{
    contracts::{InstallRequest, OperationState, SessionStatus, StoreProfile, TargetSelection},
    deployment::fingerprint_tree,
    fixtures::FixtureVault,
    operations::OperationService,
    profile::Environment,
    repository::Repository,
    session::SessionService,
    store_client::ReqwestTransport,
    targets::TargetService,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc,
    },
};
use tokio::sync::Notify;

const DENIED_SKILL: &str = "skill-123";
const OTHER_SKILL: &str = "skill-456";

#[derive(Default)]
struct StoreState {
    // 0: allowed; 200: detail succeeds with canDownload=false; otherwise HTTP denial.
    denial: AtomicU16,
    offline: AtomicBool,
    delay_denial: AtomicBool,
    entered: Notify,
    resume: Notify,
}

async fn server(state: Arc<StoreState>) -> TestServer {
    let package =
        include_bytes!("../../../../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.zip")
            .to_vec();
    TestServer::start(move |request| {
        let (state, package) = (state.clone(), package.clone());
        async move {
            if request.path.ends_with("/auth/login") {
                let mut login: Value = serde_json::from_str(include_str!(
                    "../../../../contracts/sds-desktop/v1/fixtures/auth-login.json"
                ))
                .unwrap();
                if request.body["email"] == "other@example.invalid" {
                    login["data"]["userId"] = json!(1002);
                    login["data"]["accessToken"] = json!("synthetic-access-other");
                    login["data"]["refreshToken"] = json!("synthetic-refresh-other");
                }
                return Response::json(200, login);
            }
            if state.offline.load(Ordering::SeqCst) {
                return Response::json(0, Value::Null);
            }
            let skill = if request.path.contains(OTHER_SKILL) {
                OTHER_SKILL
            } else {
                DENIED_SKILL
            };
            let name = if skill == OTHER_SKILL {
                "unrelated-skill"
            } else {
                "jira-weekly-report"
            };
            let detail_path = format!("/skm/api/v1/desktop/skills/{skill}");
            let detail = request.path == detail_path;
            let denial = state.denial.load(Ordering::SeqCst);
            let denied = detail
                && skill == DENIED_SKILL
                && denial != 0
                && request.authorization.as_deref() == Some("Bearer synthetic-access-one");
            if denied {
                if state.delay_denial.swap(false, Ordering::SeqCst) {
                    state.entered.notify_one();
                    tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        state.resume.notified(),
                    )
                    .await
                    .expect("test must release delayed detail response");
                }
                if denial != 200 {
                    return Response::json(denial, json!({"success":false}));
                }
            }
            if request.path.ends_with("/package") {
                return Response::binary(package);
            }
            let mut release: Value = serde_json::from_str(include_str!(
                "../../../../contracts/sds-desktop/v1/fixtures/release.json"
            ))
            .unwrap();
            let r = &mut release["data"];
            r["skillId"] = json!(skill);
            r["installName"] = json!(name);
            r["package"]["format"] = json!("zip");
            r["package"]["fileName"] = json!("synthetic.zip");
            r["package"]["sha256"] = json!(hex::encode(Sha256::digest(&package)));
            r["package"]["sizeBytes"] = json!(package.len() as u64);
            r["package"]["downloadPath"] = json!(format!(
                "/api/v1/desktop/skills/{skill}/releases/rel-003/package"
            ));
            if !detail {
                return Response::json(200, release);
            }
            let mut body: Value = serde_json::from_str(include_str!(
                "../../../../contracts/sds-desktop/v1/fixtures/detail.json"
            ))
            .unwrap();
            body["data"]["id"] = json!(skill);
            body["data"]["canDownload"] = json!(!denied);
            body["data"]["latestRelease"] = release["data"].clone();
            Response::json(200, body)
        }
    })
    .await
}

struct Harness {
    _http: TestServer,
    state: Arc<StoreState>,
    session: Arc<SessionService>,
    operations: Arc<OperationService>,
    repository: Arc<Repository>,
    project: TargetSelection,
    project_root: PathBuf,
    skills_root: PathBuf,
    before: String,
}

impl Harness {
    async fn new(root: &Path) -> Self {
        let state = Arc::new(StoreState::default());
        let http = server(state.clone()).await;
        let session = Arc::new(SessionService::new(
            Environment::Development,
            Some(StoreProfile::new(&http.base_url, Environment::Development).unwrap()),
            Arc::new(ReqwestTransport::fixture().unwrap()),
            Arc::new(FixtureVault::default()),
        ));
        let targets = Arc::new(TargetService::fixture(root).unwrap());
        let repository = Arc::new(Repository::open(&root.join("pilot")).unwrap());
        let operations = Arc::new(OperationService::new(
            session.clone(),
            targets.clone(),
            repository.clone(),
        ));
        // Establish genuine prior authorization for both accounts and skills via
        // the normal download/verified-cache/commit flow, never SQL-seeded grants.
        for email in ["other@example.invalid", "employee@example.invalid"] {
            session.login(email, "synthetic-password").await.unwrap();
            for skill in [DENIED_SKILL, OTHER_SKILL] {
                let plan = operations
                    .prepare_install(InstallRequest {
                        skill_id: skill.into(),
                        release_id: "rel-003".into(),
                        target: Default::default(),
                    })
                    .await
                    .unwrap();
                assert_eq!(
                    operations.execute(&plan.id, true).await.unwrap().state,
                    OperationState::Completed
                );
            }
        }
        let project_root = root.join("selected-project");
        std::fs::create_dir(&project_root).unwrap();
        std::fs::write(
            project_root.join("keep.txt"),
            b"project files stay unchanged",
        )
        .unwrap();
        let workspace = targets.add_workspace(&project_root).unwrap();
        let project = TargetSelection {
            agent_key: "aipro".into(),
            workspace_id: workspace.id,
        };
        let skills_root = root.join("agent-home/.aipro/skills");
        let before = fingerprint_tree(&skills_root).unwrap().1;
        Self {
            _http: http,
            state,
            session,
            operations,
            repository,
            project,
            project_root,
            skills_root,
            before,
        }
    }

    fn key(&self, skill: &str) -> String {
        self.repository
            .list_library()
            .unwrap()
            .into_iter()
            .find(|entry| entry.release.skill_id == skill)
            .unwrap()
            .id
    }

    async fn enter_outage(&self) {
        self.state.offline.store(true, Ordering::SeqCst);
        assert_eq!(
            self.operations
                .skill_detail(OTHER_SKILL)
                .await
                .unwrap_err()
                .code,
            "STORE_UNAVAILABLE"
        );
        assert_eq!(
            self.session.status().await.unwrap().status,
            SessionStatus::Offline
        );
    }

    async fn can_prepare(&self, skill: &str) {
        let plan = self
            .operations
            .prepare_cached_install(&self.key(skill), self.project.clone())
            .await
            .expect("an unrelated or current-account grant must remain usable");
        assert!(plan.can_execute);
        self.operations.cancel(&plan.id).unwrap();
    }

    fn assert_no_target_writes(&self) {
        assert_eq!(fingerprint_tree(&self.skills_root).unwrap().1, self.before);
        assert!(!self.project_root.join(".aipro").exists());
        assert_eq!(
            std::fs::read(self.project_root.join("keep.txt")).unwrap(),
            b"project files stay unchanged"
        );
        assert_eq!(std::fs::read_dir(&self.project_root).unwrap().count(), 1);
        assert_eq!(self.repository.deployments().unwrap().len(), 2);
    }
}

async fn denial_then_outage(denial: u16, expected_error: Option<&str>) {
    let root = tempfile::tempdir().unwrap();
    let h = Harness::new(root.path()).await;
    h.state.denial.store(denial, Ordering::SeqCst);
    // Same product entrypoint as commands::sds_skill_detail.
    let detail = h.operations.skill_detail(DENIED_SKILL).await;
    if let Some(code) = expected_error {
        assert_eq!(detail.unwrap_err().code, code);
    } else {
        // Keep detail's successful wire response; only its offline grant changes.
        assert!(!detail.unwrap().summary.metadata.can_download);
    }
    h.enter_outage().await;
    let denied = h
        .operations
        .prepare_cached_install(&h.key(DENIED_SKILL), h.project.clone())
        .await;
    h.can_prepare(OTHER_SKILL).await;
    // A valid denial for account A must not remove account B's grant to that skill.
    h.session
        .login("other@example.invalid", "synthetic-password")
        .await
        .unwrap();
    h.enter_outage().await;
    h.can_prepare(DENIED_SKILL).await;
    h.assert_no_target_writes();
    assert_eq!(
        denied
            .expect_err("detail denial must revoke this account's cached deployment grant")
            .code,
        "CACHE_NOT_AUTHORIZED"
    );
}

#[tokio::test]
async fn ui_detail_403_then_outage_denies_cached_preparation_without_target_writes() {
    denial_then_outage(403, Some("ACCESS_DENIED")).await;
}

#[tokio::test]
async fn ui_detail_404_then_outage_denies_cached_preparation_without_target_writes() {
    denial_then_outage(404, Some("NOT_FOUND")).await;
}

#[tokio::test]
async fn ui_detail_410_then_outage_denies_cached_preparation_without_target_writes() {
    denial_then_outage(410, Some("RELEASE_WITHDRAWN")).await;
}

#[tokio::test]
async fn ui_detail_can_download_false_then_outage_denies_cached_preparation_without_target_writes()
{
    denial_then_outage(200, None).await;
}

// Guard against applying a late denial to the newly signed-in account, or
// consuming stale detail data as current authority for its original account.
#[tokio::test]
async fn stale_ui_detail_denial_after_account_switch_preserves_both_accounts_grants() {
    for denial in [403, 404, 410, 200] {
        let root = tempfile::tempdir().unwrap();
        let h = Harness::new(root.path()).await;
        h.state.denial.store(denial, Ordering::SeqCst);
        h.state.delay_denial.store(true, Ordering::SeqCst);
        let operations = h.operations.clone();
        let pending = tokio::spawn(async move { operations.skill_detail(DENIED_SKILL).await });
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            h.state.entered.notified(),
        )
        .await
        .expect("detail request must reach the delayed denial");
        h.session
            .login("other@example.invalid", "synthetic-password")
            .await
            .unwrap();
        h.state.resume.notify_one();
        assert_eq!(pending.await.unwrap().unwrap_err().code, "REVIEW_REQUIRED");
        h.enter_outage().await;
        h.can_prepare(DENIED_SKILL).await;
        h.can_prepare(OTHER_SKILL).await;
        h.session
            .login("employee@example.invalid", "synthetic-password")
            .await
            .unwrap();
        h.enter_outage().await;
        h.can_prepare(DENIED_SKILL).await;
        h.assert_no_target_writes();
    }
}
