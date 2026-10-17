use super::{
    contracts::{
        CatalogQuery, Page, SessionView, SkillDetail, SkillSummary, StoreProfile, Taxonomy,
    },
    credentials::WindowsCredentialVault,
    error::SdsError,
    profile::Environment,
    session::SessionService,
    store_client::{ReqwestTransport, StoreClient},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

pub struct SdsServices {
    pub session: Arc<SessionService>,
    pub store: StoreClient,
    data_root: PathBuf,
    settings_gate: Mutex<()>,
    connection_error: Mutex<Option<SdsError>>,
}
impl SdsServices {
    pub fn new(data_root: &Path, environment: Environment) -> Result<Self, SdsError> {
        let (profile, error) = match read_connection(data_root, environment) {
            Ok(profile) => (profile, None),
            Err(error) => (None, Some(error)),
        };
        let session = Arc::new(SessionService::new(
            environment,
            profile,
            Arc::new(ReqwestTransport::new()?),
            Arc::new(WindowsCredentialVault::new(data_root)),
        ));
        Ok(Self {
            store: StoreClient::new(session.clone()),
            session,
            data_root: data_root.into(),
            settings_gate: Mutex::new(()),
            connection_error: Mutex::new(error),
        })
    }
    pub async fn set_connection(&self, base_url: &str) -> Result<StoreProfile, SdsError> {
        let _gate = self.settings_gate.lock().await;
        let profile = self.session.set_connection(base_url).await?;
        let root = self.data_root.clone();
        let saved = profile.clone();
        tokio::task::spawn_blocking(move || write_connection(&root, &saved))
            .await
            .map_err(|_| save_error())??;
        *self.connection_error.lock().await = None;
        Ok(profile)
    }
    pub async fn connection(&self) -> Result<Option<StoreProfile>, SdsError> {
        if let Some(error) = self.connection_error.lock().await.clone() {
            return Err(error);
        }
        Ok(self.session.connection().await)
    }
}

fn read_connection(
    root: &Path,
    environment: Environment,
) -> Result<Option<StoreProfile>, SdsError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Saved {
        base_url: String,
    }
    let path = root.join("connection.json");
    match std::fs::metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(metadata) if metadata.len() <= 65_536 => (),
        _ => return Err(SdsError::invalid("Saved Store connection is unreadable")),
    }
    let saved: Saved = serde_json::from_slice(
        &std::fs::read(path)
            .map_err(|_| SdsError::invalid("Saved Store connection is unreadable"))?,
    )
    .map_err(|_| SdsError::invalid("Saved Store connection is invalid"))?;
    Ok(Some(StoreProfile::new(&saved.base_url, environment)?))
}
fn write_connection(root: &Path, profile: &StoreProfile) -> Result<(), SdsError> {
    use std::io::Write;
    std::fs::create_dir_all(root).map_err(|_| save_error())?;
    let mut file = tempfile::Builder::new()
        .prefix(".connection-")
        .tempfile_in(root)
        .map_err(|_| save_error())?;
    file.write_all(&serde_json::to_vec(profile).map_err(|_| save_error())?)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| save_error())?;
    file.persist(root.join("connection.json"))
        .map_err(|_| save_error())?;
    Ok(())
}
fn save_error() -> SdsError {
    SdsError::new(
        "SETTINGS_SAVE_FAILED",
        "Store connection could not be saved",
        false,
    )
}

#[tauri::command]
pub async fn sds_get_connection(
    state: tauri::State<'_, SdsServices>,
) -> Result<Option<StoreProfile>, SdsError> {
    state.connection().await
}
#[tauri::command]
pub async fn sds_set_connection(
    state: tauri::State<'_, SdsServices>,
    base_url: String,
) -> Result<StoreProfile, SdsError> {
    state.set_connection(&base_url).await
}
#[tauri::command]
pub async fn sds_session_status(
    state: tauri::State<'_, SdsServices>,
) -> Result<SessionView, SdsError> {
    state.session.status().await
}
#[tauri::command]
pub async fn sds_login(
    state: tauri::State<'_, SdsServices>,
    email: String,
    password: String,
) -> Result<SessionView, SdsError> {
    state.session.login(&email, &password).await
}
#[tauri::command]
pub async fn sds_logout(state: tauri::State<'_, SdsServices>) -> Result<(), SdsError> {
    state.session.logout().await
}

#[tauri::command]
pub async fn sds_catalog_taxonomy(
    state: tauri::State<'_, SdsServices>,
) -> Result<Taxonomy, SdsError> {
    state.store.taxonomy().await
}
#[tauri::command]
pub async fn sds_catalog_search(
    state: tauri::State<'_, SdsServices>,
    query: CatalogQuery,
) -> Result<Page<SkillSummary>, SdsError> {
    state.store.search(query).await
}
#[tauri::command]
pub async fn sds_skill_detail(
    state: tauri::State<'_, SdsServices>,
    skill_id: String,
) -> Result<SkillDetail, SdsError> {
    state.store.detail(&skill_id).await
}
