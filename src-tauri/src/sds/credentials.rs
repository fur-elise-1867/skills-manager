use super::error::SdsError;

pub trait CredentialVault: Send + Sync {
    fn read(&self, profile_id: &str) -> Result<Option<String>, SdsError>;
    fn write(&self, profile_id: &str, value: &str) -> Result<(), SdsError>;
    fn clear(&self, profile_id: &str) -> Result<(), SdsError>;
}

pub struct WindowsCredentialVault {
    marker_root: std::path::PathBuf,
    serial: std::sync::Mutex<()>,
}
impl WindowsCredentialVault {
    pub fn new(data_root: &std::path::Path) -> Self {
        Self {
            marker_root: data_root.join("credential-state"),
            serial: std::sync::Mutex::new(()),
        }
    }
    fn marker(&self, id: &str) -> Result<std::path::PathBuf, SdsError> {
        if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(unavailable());
        }
        Ok(self.marker_root.join(format!("{id}.signed-out")))
    }
    fn entry(&self, id: &str) -> Result<keyring::Entry, SdsError> {
        self.marker(id)?;
        #[cfg(windows)]
        {
            let credential = keyring::windows::WinCredential::new_with_target(
                None,
                "com.sds.skillsmanager.desktop.store",
                id,
            )
            .map_err(|_| unavailable())?;
            Ok(keyring::Entry::new_with_credential(Box::new(credential)))
        }
        #[cfg(not(windows))]
        {
            Err(unavailable())
        }
    }
}
impl CredentialVault for WindowsCredentialVault {
    fn read(&self, id: &str) -> Result<Option<String>, SdsError> {
        let _serial = self.serial.lock().map_err(|_| unavailable())?;
        if self.marker(id)?.try_exists().map_err(|_| unavailable())? {
            return Ok(None);
        }
        match self.entry(id)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(unavailable()),
        }
    }
    fn write(&self, id: &str, value: &str) -> Result<(), SdsError> {
        let _serial = self.serial.lock().map_err(|_| unavailable())?;
        let marker = self.marker(id)?;
        self.entry(id)?
            .set_password(value)
            .map_err(|_| unavailable())?;
        match std::fs::remove_file(marker) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(unavailable()),
        }
    }
    fn clear(&self, id: &str) -> Result<(), SdsError> {
        use std::io::Write;
        let _serial = self.serial.lock().map_err(|_| unavailable())?;
        let marker = self.marker(id)?;
        std::fs::create_dir_all(&self.marker_root).map_err(|_| unavailable())?;
        let mut file = std::fs::File::create(marker).map_err(|_| unavailable())?;
        file.write_all(b"signed-out")
            .and_then(|_| file.sync_all())
            .map_err(|_| unavailable())?;
        match self.entry(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(unavailable()),
        }
    }
}

fn unavailable() -> SdsError {
    SdsError::new(
        "CREDENTIAL_UNAVAILABLE",
        "Windows credential storage is unavailable",
        false,
    )
}
