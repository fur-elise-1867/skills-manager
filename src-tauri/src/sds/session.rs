use super::{
    contracts::{SessionContext, SessionStatus, SessionView, StoreProfile, UserView},
    credentials::CredentialVault,
    error::SdsError,
    profile::Environment,
    store_client::{join_api, HttpRequest, HttpResponse, HttpTransport},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

struct SessionState {
    profile: Option<StoreProfile>,
    generation: u64,
    revision: u64,
    user: Option<UserView>,
    status: SessionStatus,
    access: Option<String>,
    refresh: Option<String>,
    restored: bool,
}
impl SessionState {
    fn view(&self) -> SessionView {
        SessionView {
            status: self.status,
            user: self.user.clone(),
        }
    }
    fn invalidate(&mut self) {
        self.generation += 1;
        self.revision += 1;
        self.user = None;
        self.status = SessionStatus::SignedOut;
        self.access = None;
        self.refresh = None;
        self.restored = true;
    }
}

// Secret payloads have no Debug implementation and are never exported through IPC.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginData {
    user_id: serde_json::Value,
    user_name: String,
    access_token: String,
    refresh_token: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshData {
    access_token: Option<String>,
    token: Option<String>,
    refresh_token: String,
}
#[derive(Deserialize)]
struct Envelope<T> {
    success: bool,
    data: T,
}
#[derive(Serialize, Deserialize)]
struct RememberedSession {
    user: UserView,
    refresh: String,
}

pub struct SessionService {
    state: Mutex<SessionState>,
    environment: Environment,
    transport: Arc<dyn HttpTransport>,
    vault: Arc<dyn CredentialVault>,
    credential_gate: Mutex<()>,
    refresh_gate: Mutex<()>,
    connection_gate: Mutex<()>,
    restore_gate: Mutex<()>,
}
impl SessionService {
    pub fn new(
        environment: Environment,
        profile: Option<StoreProfile>,
        transport: Arc<dyn HttpTransport>,
        vault: Arc<dyn CredentialVault>,
    ) -> Self {
        Self {
            state: Mutex::new(SessionState {
                profile,
                generation: 0,
                revision: 0,
                user: None,
                status: SessionStatus::SignedOut,
                access: None,
                refresh: None,
                restored: false,
            }),
            environment,
            transport,
            vault,
            credential_gate: Mutex::new(()),
            refresh_gate: Mutex::new(()),
            connection_gate: Mutex::new(()),
            restore_gate: Mutex::new(()),
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<SessionView, SdsError> {
        let (profile, generation) = {
            let mut state = self.state.lock().await;
            let profile = state.profile.clone().ok_or_else(no_connection)?;
            state.invalidate();
            (profile, state.generation)
        };
        {
            let _gate = self.credential_gate.lock().await;
            self.check_generation(generation).await?;
            let vault = self.vault.clone();
            let id = profile.id.clone();
            tokio::task::spawn_blocking(move || vault.clear(&id))
                .await
                .map_err(|_| credential_error())?
                .map_err(|_| credential_error())?;
        }
        let request = HttpRequest::post(
            join_api(&profile.base_url, "api/v1/auth/login")?,
            serde_json::json!({"email": email.trim(), "password": password}),
        );
        let response = self.transport.send(request).await?;
        if response.status != 200 {
            // Existing Store reports bad credentials as HTTP 400. Whitelist its
            // machine codes; never echo the diagnostic message or distinguish a
            // missing account from a wrong password at the desktop boundary.
            if response.status == 400 {
                let code = serde_json::from_slice::<serde_json::Value>(&response.body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("errorCode")
                            .and_then(|code| code.as_str())
                            .map(str::to_owned)
                    });
                if matches!(code.as_deref(), Some("USER_NOT_FOUND" | "INVALID_PASSWORD")) {
                    return Err(http_error(401, true));
                }
            }
            return Err(http_error(response.status, true));
        }
        let data: LoginData = parse_envelope(&response)?;
        let id = match data.user_id {
            serde_json::Value::Number(number) => number
                .as_u64()
                .filter(|id| *id > 0)
                .map(|id| id.to_string()),
            serde_json::Value::String(id) if !id.is_empty() => Some(id),
            _ => None,
        }
        .ok_or_else(invalid_response)?;
        if data.user_name.trim().is_empty()
            || data.access_token.is_empty()
            || data.refresh_token.is_empty()
        {
            return Err(invalid_response());
        }
        let user = UserView {
            id,
            display_name: data.user_name,
        };
        let remembered = serde_json::to_string(&RememberedSession {
            user: user.clone(),
            refresh: data.refresh_token.clone(),
        })
        .map_err(|_| invalid_response())?;
        let _gate = self.credential_gate.lock().await;
        self.check_generation(generation).await?;
        let vault = self.vault.clone();
        let id = profile.id.clone();
        tokio::task::spawn_blocking(move || vault.write(&id, &remembered))
            .await
            .map_err(|_| credential_error())?
            .map_err(|_| credential_error())?;
        let mut state = self.state.lock().await;
        if state.generation != generation {
            return Err(stale_context());
        }
        state.user = Some(user);
        state.access = Some(data.access_token);
        state.refresh = Some(data.refresh_token);
        state.status = SessionStatus::Authenticated;
        state.revision += 1;
        Ok(state.view())
    }

    pub async fn status(&self) -> Result<SessionView, SdsError> {
        self.ensure_restored().await?;
        Ok(self.state.lock().await.view())
    }
    async fn ensure_restored(&self) -> Result<(), SdsError> {
        let _restore = self.restore_gate.lock().await;
        let (profile, generation) = {
            let mut state = self.state.lock().await;
            if state.restored {
                return Ok(());
            }
            state.restored = true;
            (state.profile.clone(), state.generation)
        };
        let Some(profile) = profile else {
            return Ok(());
        };
        let saved = {
            let _gate = self.credential_gate.lock().await;
            self.check_generation(generation).await?;
            let vault = self.vault.clone();
            let id = profile.id;
            tokio::task::spawn_blocking(move || vault.read(&id))
                .await
                .map_err(|_| credential_error())?
                .map_err(|_| credential_error())?
        };
        self.check_generation(generation).await?;
        let Some(saved) = saved else {
            return Ok(());
        };
        let remembered: RememberedSession =
            serde_json::from_str(&saved).map_err(|_| credential_error())?;
        if remembered.refresh.is_empty()
            || remembered.user.id.is_empty()
            || remembered.user.display_name.is_empty()
        {
            return Err(credential_error());
        }
        let revision = {
            let mut state = self.state.lock().await;
            if state.generation != generation {
                return Err(stale_context());
            }
            state.user = Some(remembered.user);
            state.refresh = Some(remembered.refresh);
            state.status = SessionStatus::Offline;
            state.revision
        };
        match self.refresh_once(generation, revision).await {
            Ok(()) => Ok(()),
            Err(error)
                if ["STORE_UNAVAILABLE", "SESSION_EXPIRED", "RATE_LIMITED"]
                    .contains(&error.code.as_str()) =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    pub async fn logout(&self) -> Result<(), SdsError> {
        let (profile, refresh, access, generation) = {
            let mut state = self.state.lock().await;
            let snapshot = (
                state.profile.clone(),
                state.refresh.clone(),
                state.access.clone(),
            );
            state.invalidate();
            (snapshot.0, snapshot.1, snapshot.2, state.generation)
        };
        if let Some(profile) = profile {
            {
                let _gate = self.credential_gate.lock().await;
                if self.check_generation(generation).await.is_ok() {
                    let vault = self.vault.clone();
                    let id = profile.id.clone();
                    tokio::task::spawn_blocking(move || vault.clear(&id))
                        .await
                        .map_err(|_| credential_error())?
                        .map_err(|_| credential_error())?;
                }
            }
            if let Some(refresh) = refresh {
                let mut request = HttpRequest::post(
                    join_api(&profile.base_url, "api/v1/auth/logout")?,
                    serde_json::json!({"refreshToken":refresh}),
                );
                request.bearer = access;
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    self.transport.send(request),
                )
                .await;
            }
        }
        Ok(())
    }
    pub async fn request(&self, request: HttpRequest) -> Result<HttpResponse, SdsError> {
        self.request_inner(None, request, None, true).await
    }

    pub async fn scope(&self) -> Result<(StoreProfile, SessionContext), SdsError> {
        self.ensure_restored().await?;
        let state = self.state.lock().await;
        let user = state
            .user
            .as_ref()
            .filter(|_| {
                !matches!(
                    state.status,
                    SessionStatus::SignedOut | SessionStatus::Expired
                )
            })
            .ok_or_else(|| http_error(401, false))?;
        let profile = state.profile.clone().ok_or_else(no_connection)?;
        let context = SessionContext {
            profile_id: profile.id.clone(),
            account_id: user.id.clone(),
            generation: state.generation,
        };
        Ok((profile, context))
    }
    pub async fn request_for(
        &self,
        context: &SessionContext,
        request: HttpRequest,
    ) -> Result<HttpResponse, SdsError> {
        self.request_inner(Some(context), request, None, true).await
    }
    pub async fn request_raw_for(
        &self,
        context: &SessionContext,
        request: HttpRequest,
    ) -> Result<HttpResponse, SdsError> {
        self.request_inner(Some(context), request, None, false).await
    }
    pub async fn mark_offline_for(&self, context: &SessionContext) {
        self.mark_offline(context.generation).await;
    }
    /// Linearize a short filesystem/DB publication against logout and profile replacement.
    /// The action must not await, copy content, or call back into this service.
    pub async fn with_current_context<T>(
        &self,
        context: &SessionContext,
        publish: impl FnOnce() -> Result<T, SdsError>,
    ) -> Result<T, SdsError> {
        let state = self.state.lock().await;
        if state.generation != context.generation
            || state.profile.as_ref().map(|profile| &profile.id) != Some(&context.profile_id)
            || state.user.as_ref().map(|user| &user.id) != Some(&context.account_id)
            || matches!(
                state.status,
                SessionStatus::SignedOut | SessionStatus::Expired
            )
        {
            return Err(stale_context());
        }
        publish()
    }
    /// Keep account/profile changes queued until the bounded filesystem/SQLite
    /// commit finishes. Content verification runs on a blocking worker, not on
    /// the async executor. Extraction and copying must finish before this call.
    #[cfg(feature = "sds")]
    pub(crate) async fn commit_for<T: Send + 'static>(
        self: &Arc<Self>,
        context: &SessionContext,
        publish: impl FnOnce() -> Result<T, SdsError> + Send + 'static,
    ) -> Result<T, SdsError> {
        let session = self.clone();
        let context = context.clone();
        tokio::task::spawn_blocking(move || {
            let state = session.state.blocking_lock();
            if state.generation != context.generation
                || state.profile.as_ref().map(|profile| &profile.id) != Some(&context.profile_id)
                || state.user.as_ref().map(|user| &user.id) != Some(&context.account_id)
                || state.status != SessionStatus::Authenticated
            {
                return Err(stale_context());
            }
            let result = publish();
            drop(state);
            result
        })
        .await
        .map_err(|_| {
            SdsError::new(
                "COMMIT_INTERRUPTED",
                "The installation commit could not finish. Restart SDS to reconcile its journal.",
                false,
            )
        })?
    }
    #[cfg(feature = "sds")]
    pub(crate) async fn commit_cached_for<T: Send + 'static>(
        self: &Arc<Self>,
        context: &SessionContext,
        publish: impl FnOnce() -> Result<T, SdsError> + Send + 'static,
    ) -> Result<T, SdsError> {
        let session = self.clone();
        let context = context.clone();
        tokio::task::spawn_blocking(move || {
            let state = session.state.blocking_lock();
            if state.status != SessionStatus::Offline
                || state.generation != context.generation
                || state.profile.as_ref().map(|p| &p.id) != Some(&context.profile_id)
                || state.user.as_ref().map(|u| &u.id) != Some(&context.account_id)
            {
                return Err(stale_context());
            }
            let result = publish();
            drop(state);
            result
        })
        .await
        .map_err(|_| stale_context())?
    }
    pub async fn download_for(
        &self,
        context: &SessionContext,
        request: HttpRequest,
        sink: &mut super::store_client::DownloadSink<'_>,
    ) -> Result<(), SdsError> {
        self.request_inner(Some(context), request, Some(sink), true)
            .await
            .map(|_| ())
    }
    async fn request_inner(
        &self,
        context: Option<&SessionContext>,
        mut request: HttpRequest,
        mut sink: Option<&mut super::store_client::DownloadSink<'_>>,
        fail_on_http_error: bool,
    ) -> Result<HttpResponse, SdsError> {
        let (profile, generation, revision, access) = {
            let state = self.state.lock().await;
            if context.is_some_and(|context| {
                context.generation != state.generation
                    || state.profile.as_ref().map(|profile| &profile.id)
                        != Some(&context.profile_id)
                    || state.user.as_ref().map(|user| &user.id) != Some(&context.account_id)
            }) {
                return Err(stale_context());
            }
            if state.user.is_none()
                || matches!(
                    state.status,
                    SessionStatus::SignedOut | SessionStatus::Expired
                )
            {
                return Err(http_error(401, false));
            }
            (
                state.profile.clone().ok_or_else(no_connection)?,
                state.generation,
                state.revision,
                state.access.clone(),
            )
        };
        super::store_client::validate_protected_url(&profile.base_url, &request.url)?;
        if access.is_none() {
            self.refresh_once(generation, revision).await?;
        }
        request.bearer = self.state.lock().await.access.clone();
        self.check_generation(generation).await?;
        let mut response = self
            .send_scoped(request.clone(), generation, sink.as_deref_mut())
            .await?;
        if response.status == 401 {
            self.refresh_once(generation, revision).await?;
            request.bearer = self.state.lock().await.access.clone();
            self.check_generation(generation).await?;
            response = self.send_scoped(request, generation, sink).await?;
            if response.status == 401 {
                self.expire(generation).await?;
                return Err(http_error(401, false));
            }
        }
        if fail_on_http_error {
            if response.status >= 500 {
                self.mark_offline(generation).await;
            } else {
                let mut state = self.state.lock().await;
                if state.generation == generation {
                    state.status = SessionStatus::Authenticated;
                }
            }
            if !(200..300).contains(&response.status) {
                return Err(http_error(response.status, false));
            }
        } else if (200..300).contains(&response.status) {
            let mut state = self.state.lock().await;
            if state.generation == generation {
                state.status = SessionStatus::Authenticated;
            }
        }
        Ok(response)
    }

    async fn send_scoped(
        &self,
        request: HttpRequest,
        generation: u64,
        sink: Option<&mut super::store_client::DownloadSink<'_>>,
    ) -> Result<HttpResponse, SdsError> {
        let response = match sink {
            Some(sink) => self.transport.download(request, sink).await,
            None => self.transport.send(request).await,
        };
        self.check_generation(generation).await?;
        if response
            .as_ref()
            .err()
            .is_some_and(|error| error.code == "STORE_UNAVAILABLE")
        {
            self.mark_offline(generation).await;
        }
        response
    }
    async fn mark_offline(&self, generation: u64) {
        let mut state = self.state.lock().await;
        if state.generation == generation && state.user.is_some() {
            state.status = SessionStatus::Offline;
        }
    }
    async fn expire(&self, generation: u64) -> Result<(), SdsError> {
        let (profile, expired_generation) = {
            let mut state = self.state.lock().await;
            if state.generation != generation {
                return Err(stale_context());
            }
            let profile = state.profile.clone().ok_or_else(no_connection)?;
            let user = state.user.clone();
            state.invalidate();
            state.user = user;
            state.status = SessionStatus::Expired;
            (profile, state.generation)
        };
        let _gate = self.credential_gate.lock().await;
        if self.check_generation(expired_generation).await.is_ok() {
            let vault = self.vault.clone();
            tokio::task::spawn_blocking(move || vault.clear(&profile.id))
                .await
                .map_err(|_| credential_error())?
                .map_err(|_| credential_error())?;
        }
        Ok(())
    }
    async fn refresh_once(&self, generation: u64, revision: u64) -> Result<(), SdsError> {
        // Only async synchronization guards span I/O; state guards are always short-lived.
        let _refresh = self.refresh_gate.lock().await;
        let (profile, refresh, user) = {
            let state = self.state.lock().await;
            if state.generation != generation {
                return Err(stale_context());
            }
            if state.revision != revision {
                return Ok(());
            }
            (
                state.profile.clone().ok_or_else(no_connection)?,
                state
                    .refresh
                    .clone()
                    .ok_or_else(|| http_error(401, false))?,
                state.user.clone().ok_or_else(|| http_error(401, false))?,
            )
        };
        let response = self
            .send_scoped(
                HttpRequest::post(
                    join_api(&profile.base_url, "api/v1/auth/refresh")?,
                    serde_json::json!({"refreshToken":refresh}),
                ),
                generation,
                None,
            )
            .await?;
        if response.status == 429 || response.status >= 500 {
            self.mark_offline(generation).await;
            return Err(http_error(response.status, false));
        }
        if response.status != 200 {
            self.expire(generation).await?;
            return Err(http_error(401, false));
        }
        let data: RefreshData = match parse_envelope(&response) {
            Ok(data) => data,
            Err(error) => {
                self.expire(generation).await?;
                return Err(error);
            }
        };
        let access = data
            .access_token
            .or(data.token)
            .filter(|token| !token.is_empty());
        if access.is_none() || data.refresh_token.is_empty() {
            self.expire(generation).await?;
            return Err(invalid_response());
        }
        let remembered = serde_json::to_string(&RememberedSession {
            user,
            refresh: data.refresh_token.clone(),
        })
        .map_err(|_| invalid_response())?;
        let credential_gate = self.credential_gate.lock().await;
        self.check_generation(generation).await?;
        let vault = self.vault.clone();
        let id = profile.id;
        let saved = tokio::task::spawn_blocking(move || vault.write(&id, &remembered))
            .await
            .map_err(|_| credential_error())?;
        if saved.is_err() {
            drop(credential_gate);
            self.expire(generation).await?;
            return Err(credential_error());
        }
        let mut state = self.state.lock().await;
        if state.generation != generation {
            return Err(stale_context());
        }
        state.access = access;
        state.refresh = Some(data.refresh_token);
        state.status = SessionStatus::Authenticated;
        state.revision += 1;
        Ok(())
    }
    pub async fn connection(&self) -> Option<StoreProfile> {
        self.state.lock().await.profile.clone()
    }
    pub async fn set_connection(&self, base_url: &str) -> Result<StoreProfile, SdsError> {
        let profile = StoreProfile::new(base_url, self.environment)?;
        let _change = self.connection_gate.lock().await;
        let previous = {
            let mut state = self.state.lock().await;
            if state
                .profile
                .as_ref()
                .is_some_and(|current| current.id == profile.id)
            {
                return Ok(profile);
            }
            let old = (
                state.profile.clone(),
                state.refresh.clone(),
                state.access.clone(),
            );
            state.invalidate();
            state.profile = Some(profile.clone());
            old
        };
        if let (Some(old), refresh, access) = previous {
            {
                let _gate = self.credential_gate.lock().await;
                let vault = self.vault.clone();
                let id = old.id;
                tokio::task::spawn_blocking(move || vault.clear(&id))
                    .await
                    .map_err(|_| credential_error())?
                    .map_err(|_| credential_error())?;
            }
            if let Some(refresh) = refresh {
                let mut request = HttpRequest::post(
                    join_api(&old.base_url, "api/v1/auth/logout")?,
                    serde_json::json!({"refreshToken":refresh}),
                );
                request.bearer = access;
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    self.transport.send(request),
                )
                .await;
            }
        }
        Ok(profile)
    }
    async fn check_generation(&self, generation: u64) -> Result<(), SdsError> {
        if self.state.lock().await.generation == generation {
            Ok(())
        } else {
            Err(stale_context())
        }
    }
}

fn parse_envelope<T: serde::de::DeserializeOwned>(response: &HttpResponse) -> Result<T, SdsError> {
    let envelope: Envelope<T> =
        serde_json::from_slice(&response.body).map_err(|_| invalid_response())?;
    if envelope.success {
        Ok(envelope.data)
    } else {
        Err(invalid_response())
    }
}
fn invalid_response() -> SdsError {
    SdsError::new(
        "INVALID_RESPONSE",
        "Skills Store returned an unsupported response",
        false,
    )
}
fn credential_error() -> SdsError {
    SdsError::new(
        "CREDENTIAL_UNAVAILABLE",
        "Windows credential storage is unavailable",
        false,
    )
}
fn no_connection() -> SdsError {
    SdsError::new(
        "NO_CONNECTION",
        "Configure your Store connection first",
        false,
    )
}
fn stale_context() -> SdsError {
    SdsError::new(
        "REVIEW_REQUIRED",
        "Connection or account changed; review this action again",
        false,
    )
}
pub(crate) fn http_error(status: u16, login: bool) -> SdsError {
    match status {
        401 if login => SdsError::new("AUTH_FAILED", "Email or password was not accepted", false),
        401 => SdsError::new("SESSION_EXPIRED", "Your Store session has expired", false),
        403 if login => SdsError::new(
            "ACCOUNT_UNAVAILABLE",
            "This Store account is unavailable",
            false,
        ),
        403 => SdsError::new(
            "ACCESS_DENIED",
            "You do not have access to this content",
            false,
        ),
        404 => SdsError::new("NOT_FOUND", "Store content is unavailable", false),
        409 => SdsError::new(
            "RELEASE_CHANGED",
            "The release changed; review it again",
            false,
        ),
        410 => SdsError::new("RELEASE_WITHDRAWN", "This release was withdrawn", false),
        422 => SdsError::new(
            "CATALOG_MODE_UNAVAILABLE",
            "This catalog mode is unavailable",
            false,
        ),
        429 => SdsError::new(
            "RATE_LIMITED",
            "Wait before retrying the Store request",
            true,
        ),
        500..=599 => SdsError::new("STORE_UNAVAILABLE", "Skills Store is unavailable", true),
        _ => invalid_response(),
    }
}
