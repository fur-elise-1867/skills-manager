pub mod commands;
pub mod contracts;
pub mod credentials;
pub mod error;
#[cfg(debug_assertions)]
pub mod fixtures;
pub mod profile;
pub mod runtime;
pub mod session;
pub mod store_client;

#[tauri::command]
pub fn runtime_profile() -> Result<profile::ProfileInfo, error::SdsError> {
    if cfg!(feature = "sds") {
        Ok(profile::SdsProfile::resolve()?.info())
    } else {
        Ok(profile::ProfileInfo {
            kind: "upstream".into(),
            environment: profile::Environment::Internal,
            fixture_mode: false,
        })
    }
}

#[cfg(all(test, feature = "sds"))]
mod tests;
