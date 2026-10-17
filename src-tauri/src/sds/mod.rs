pub mod commands;
pub mod contracts;
pub mod credentials;
#[cfg(feature = "sds")]
pub mod deployment;
pub mod error;
#[cfg(debug_assertions)]
pub mod fixtures;
#[cfg(feature = "sds")]
pub mod journal;
#[cfg(feature = "sds")]
pub mod library;
pub mod mcp;
#[cfg(feature = "sds")]
pub mod operations;
#[cfg(feature = "sds")]
pub mod packages;
pub mod profile;
#[cfg(feature = "sds")]
pub mod removal;
#[cfg(feature = "sds")]
mod removal_cleanup;
#[cfg(feature = "sds")]
pub mod repository;
pub mod runtime;
pub mod session;
pub mod store_client;
pub mod targets;

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
#[cfg(feature = "sds")]
pub mod updates_metadata;

#[cfg(feature = "sds")]
pub mod workspaces;

#[cfg(feature = "sds")]
pub mod snapshots;

#[cfg(feature = "sds")]
pub mod lifecycle;

#[cfg(feature = "sds")]
pub mod replacement;

#[cfg(feature = "sds")]
pub mod updates;

#[cfg(feature = "sds")]
pub mod offline;

#[cfg(feature = "sds")]
pub mod cache_cleanup;
