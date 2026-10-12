use super::{config::CompanyConfig, store_client::StoreClient, types::*};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub trait CredentialStore: Send + Sync {
    fn read(&self) -> CompanyResult<Option<String>>;
    fn write(&self, value: &str) -> CompanyResult<()>;
    fn delete(&self) -> CompanyResult<()>;
}
#[derive(Default)]
pub struct MemoryCredentials(Mutex<Option<String>>);
impl CredentialStore for MemoryCredentials {
    fn read(&self) -> CompanyResult<Option<String>> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn write(&self, value: &str) -> CompanyResult<()> {
        *self.0.lock().unwrap() = Some(value.into());
        Ok(())
    }
    fn delete(&self) -> CompanyResult<()> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
pub struct WindowsCredentials {
    account: String,
}
impl WindowsCredentials {
    pub fn new(config: &CompanyConfig) -> Self {
        use sha2::{Digest, Sha256};
        Self {
            account: hex::encode(Sha256::digest(config.store_url.as_bytes())),
        }
    }
    fn entry(&self) -> CompanyResult<keyring::Entry> {
        keyring::Entry::new("CompanySkillsManager.Store", &self.account).map_err(|_| {
            CompanyError::new(
                "CREDENTIAL_ERROR",
                "Không truy cập được Windows Credential Manager.",
            )
        })
    }
}
impl CredentialStore for WindowsCredentials {
    fn read(&self) -> CompanyResult<Option<String>> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CompanyError::new(
                "CREDENTIAL_ERROR",
                "Không đọc được phiên đã lưu.",
            )),
        }
    }
    fn write(&self, value: &str) -> CompanyResult<()> {
        self.entry()?
            .set_password(value)
            .map_err(|_| CompanyError::new("CREDENTIAL_ERROR", "Không lưu được phiên đăng nhập."))
    }
    fn delete(&self) -> CompanyResult<()> {
        match self.entry()?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(CompanyError::new(
                "CREDENTIAL_ERROR",
                "Không xóa được credential đã lưu.",
            )),
        }
    }
}
struct State {
    session: Option<StoredSession>,
    persistent: bool,
}
pub struct SessionService {
    client: StoreClient,
    credentials: Arc<dyn CredentialStore>,
    marker: PathBuf,
    state: Mutex<State>,
}
impl SessionService {
    pub fn new(
        config: &CompanyConfig,
        credentials: Arc<dyn CredentialStore>,
    ) -> CompanyResult<Self> {
        fs::create_dir_all(&config.data_root).map_err(|_| {
            CompanyError::new("WRITE_FAILED", "Không tạo được thư mục dữ liệu app.")
        })?;
        use sha2::{Digest, Sha256};
        let marker = config.data_root.join(format!(
            "session-{}.marker",
            hex::encode(Sha256::digest(config.store_url.as_bytes()))
        ));
        let stored = credentials
            .read()
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_str::<StoredSession>(&value).ok());
        let nonce = fs::read_to_string(&marker).unwrap_or_default();
        let session =
            stored.filter(|value| value.nonce == nonce && !nonce.is_empty() && Self::valid(value));
        let persistent = session.is_some();
        Ok(Self {
            client: StoreClient::new(config)?,
            credentials,
            marker,
            state: Mutex::new(State {
                session,
                persistent,
            }),
        })
    }
    fn valid(session: &StoredSession) -> bool {
        chrono::DateTime::parse_from_rfc3339(&session.expires_at)
            .map(|expires| expires > chrono::Utc::now())
            .unwrap_or(false)
            && !session.access_token.is_empty()
    }
    pub fn current(&self) -> SessionView {
        let state = self.state.lock().unwrap();
        match state
            .session
            .as_ref()
            .filter(|session| Self::valid(session))
        {
            Some(session) => SessionView {
                user: Some(session.user.clone()),
                expires_at: Some(session.expires_at.clone()),
                persistent: state.persistent,
            },
            None => SessionView {
                user: None,
                expires_at: None,
                persistent: false,
            },
        }
    }
    pub fn access_token(&self) -> CompanyResult<String> {
        self.state
            .lock()
            .unwrap()
            .session
            .as_ref()
            .filter(|session| Self::valid(session))
            .map(|session| session.access_token.clone())
            .ok_or_else(|| {
                CompanyError::new(
                    "SESSION_EXPIRED",
                    "Phiên đăng nhập đã hết hạn. Vui lòng đăng nhập lại.",
                )
            })
    }
    pub fn login(&self, username: &str, password: &str) -> CompanyResult<SessionView> {
        let mut session = self.client.login(username, password)?;
        if !Self::valid(&session) {
            return Err(CompanyError::new(
                "INVALID_RESPONSE",
                "Store trả phiên không hợp lệ.",
            ));
        }
        session.nonce = uuid::Uuid::new_v4().to_string();
        let marker_ok = fs::write(&self.marker, &session.nonce).is_ok();
        let persistent =
            marker_ok
                && self
                    .credentials
                    .write(&serde_json::to_string(&session).map_err(|_| {
                        CompanyError::new("CREDENTIAL_ERROR", "Không lưu được phiên.")
                    })?)
                    .is_ok();
        if !persistent {
            let _ = self.credentials.delete();
        }
        *self.state.lock().unwrap() = State {
            session: Some(session),
            persistent,
        };
        Ok(self.current())
    }
    pub fn clear(&self) {
        *self.state.lock().unwrap() = State {
            session: None,
            persistent: false,
        };
        let _ = fs::write(&self.marker, "");
        let _ = self.credentials.delete();
    }
    pub fn logout(&self) -> CompanyResult<()> {
        let token = self.access_token().ok();
        self.clear();
        if let Some(token) = token {
            self.client.logout(&token).map_err(|_| {
                CompanyError::new(
                    "LOGOUT_UNCONFIRMED",
                    "Đã đăng xuất tại máy; chưa xác nhận thu hồi phiên phía Store.",
                )
            })?;
        }
        Ok(())
    }
}
