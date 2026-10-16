pub mod error;
pub mod profile;
pub mod runtime;

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
