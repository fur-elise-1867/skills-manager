use super::error::SdsError;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Environment {
    Development,
    Internal,
}

impl Environment {
    pub fn parse(value: &str) -> Result<Self, SdsError> {
        match value {
            "development" => Ok(Self::Development),
            "internal" => Ok(Self::Internal),
            _ => Err(SdsError::invalid("Unsupported SDS environment")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub kind: String,
    pub environment: Environment,
    pub fixture_mode: bool,
}

pub struct SdsProfile {
    data_root: PathBuf,
    environment: Environment,
    fixture_mode: bool,
}

impl SdsProfile {
    pub fn new(
        data_root: PathBuf,
        environment: Environment,
        fixture_mode: bool,
        debug: bool,
    ) -> Result<Self, SdsError> {
        if fixture_mode && (!debug || environment != Environment::Development) {
            return Err(SdsError::invalid(
                "Fixtures require an explicit development debug build",
            ));
        }
        Ok(Self {
            data_root: if fixture_mode {
                data_root.join("development-fixtures")
            } else {
                data_root
            },
            environment,
            fixture_mode,
        })
    }

    pub fn resolve() -> Result<Self, SdsError> {
        let environment = Environment::parse(option_env!("SDS_ENVIRONMENT").unwrap_or("internal"))?;
        let fixtures = option_env!("SDS_FIXTURES").unwrap_or("0") == "1";
        let local = dirs::data_local_dir()
            .ok_or_else(|| SdsError::invalid("Local application data directory is unavailable"))?;
        Self::new(
            local.join("SDS").join("SkillsManager"),
            environment,
            fixtures,
            cfg!(debug_assertions),
        )
    }

    pub fn data_root(&self) -> &Path {
        &self.data_root
    }
    pub fn info(&self) -> ProfileInfo {
        ProfileInfo {
            kind: "sds".into(),
            environment: self.environment,
            fixture_mode: self.fixture_mode,
        }
    }
}

/// Dispatch happens before upstream storage, plugins and background tasks exist.
pub fn dispatch_runtime(sds: bool, run_sds: impl FnOnce(), run_upstream: impl FnOnce()) {
    if sds {
        run_sds()
    } else {
        run_upstream()
    }
}

pub fn matches_identifier(sds: bool, identifier: &str) -> bool {
    sds == (identifier == "com.sds.skillsmanager.desktop")
}
