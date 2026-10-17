//! Only compiled into debug builds; the launcher opts in and owns the HTTP server.
use super::{credentials::CredentialVault, error::SdsError};
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub struct FixtureVault(Mutex<HashMap<String, String>>);
impl CredentialVault for FixtureVault {
    fn read(&self, id: &str) -> Result<Option<String>, SdsError> {
        Ok(self.0.lock().map_err(|_| unavailable())?.get(id).cloned())
    }
    fn write(&self, id: &str, value: &str) -> Result<(), SdsError> {
        self.0
            .lock()
            .map_err(|_| unavailable())?
            .insert(id.into(), value.into());
        Ok(())
    }
    fn clear(&self, id: &str) -> Result<(), SdsError> {
        self.0.lock().map_err(|_| unavailable())?.remove(id);
        Ok(())
    }
}
fn unavailable() -> SdsError {
    SdsError::new(
        "CREDENTIAL_UNAVAILABLE",
        "Development fixture session is unavailable",
        false,
    )
}

pub fn loopback_fixture_url(base: &str) -> Result<(), SdsError> {
    let url = reqwest::Url::parse(base).map_err(|_| invalid())?;
    let host = url.host_str().ok_or_else(invalid)?;
    if url.scheme() != "http"
        || !host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    {
        return Err(invalid());
    }
    Ok(())
}
fn invalid() -> SdsError {
    SdsError::invalid("Development fixtures require a dedicated loopback HTTP Store")
}
