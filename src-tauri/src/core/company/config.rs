use super::types::{CompanyError, CompanyResult};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct CompanyConfig {
    pub store_url: String,
    pub profile_root: PathBuf,
    pub data_root: PathBuf,
    pub mock: bool,
    pub allowed_hosts: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub store_url: String,
    pub profile_root: String,
    pub data_root: String,
    pub mock: bool,
    pub configured: bool,
    pub recovery_warnings: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreConfig {
    base_url: String,
    #[serde(default)]
    allowed_hosts: Vec<String>,
}

impl CompanyConfig {
    pub fn from_environment() -> CompanyResult<Self> {
        Self::from_environment_with_resource(None)
    }
    pub fn from_environment_with_resource(resource: Option<&Path>) -> CompanyResult<Self> {
        let default_profile = dirs::home_dir().ok_or_else(|| {
            CompanyError::new(
                "CONFIGURATION_ERROR",
                "Không xác định được profile Windows.",
            )
        })?;
        let default_data = dirs::data_local_dir()
            .ok_or_else(|| {
                CompanyError::new(
                    "CONFIGURATION_ERROR",
                    "Không xác định được thư mục dữ liệu.",
                )
            })?
            .join("CompanySkillsManager");
        let overrides = [
            "COMPANY_STORE_MODE",
            "COMPANY_STORE_BASE_URL",
            "COMPANY_PROFILE_ROOT",
            "COMPANY_DATA_ROOT",
        ];
        if !cfg!(debug_assertions) && overrides.iter().any(|name| env::var(name).is_ok()) {
            return Err(CompanyError::new(
                "CONFIGURATION_ERROR",
                "Bản phát hành không nhận cấu hình debug/mock.",
            ));
        }
        let data_root = env::var_os("COMPANY_DATA_ROOT")
            .map(PathBuf::from)
            .unwrap_or(default_data);
        let profile_root = env::var_os("COMPANY_PROFILE_ROOT")
            .map(PathBuf::from)
            .unwrap_or(default_profile);
        let file = data_root.join("store.json");
        let (store_url, allowed_hosts) = match env::var("COMPANY_STORE_BASE_URL") {
            Ok(url) => (url, vec![]),
            Err(_) if file.exists() || resource.is_some_and(|p| p.is_file()) => {
                let file = if file.exists() {
                    file.as_path()
                } else {
                    resource.unwrap()
                };
                let stored =
                    serde_json::from_slice::<StoreConfig>(&fs::read(file).map_err(|_| {
                        CompanyError::new("CONFIGURATION_ERROR", "Không đọc được store.json.")
                    })?)
                    .map_err(|_| {
                        CompanyError::new("CONFIGURATION_ERROR", "store.json không hợp lệ.")
                    })?;
                (stored.base_url, stored.allowed_hosts)
            }
            Err(_) => (String::new(), vec![]),
        };
        let config = Self {
            store_url: store_url.trim_end_matches('/').to_string(),
            profile_root,
            data_root,
            mock: env::var("COMPANY_STORE_MODE").as_deref() == Ok("mock"),
            allowed_hosts,
        };
        config.validate()?;
        Ok(config)
    }
    pub fn for_test(url: &str, profile_root: PathBuf, data_root: PathBuf) -> CompanyResult<Self> {
        let config = Self {
            store_url: url.trim_end_matches('/').to_string(),
            profile_root,
            data_root,
            mock: true,
            allowed_hosts: vec![],
        };
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> CompanyResult<()> {
        if !self.profile_root.is_absolute() || !self.data_root.is_absolute() {
            return Err(CompanyError::new(
                "CONFIGURATION_ERROR",
                "Đường dẫn profile và dữ liệu phải là đường dẫn tuyệt đối.",
            ));
        }
        if self.mock && !cfg!(debug_assertions) {
            return Err(CompanyError::new(
                "CONFIGURATION_ERROR",
                "Bản phát hành không nhận mock.",
            ));
        }
        validate_store_endpoint(
            &self.store_url,
            &self.allowed_hosts,
            cfg!(debug_assertions) && self.mock,
        )
    }
    pub fn environment(&self) -> Environment {
        Environment {
            store_url: self.store_url.clone(),
            profile_root: self.profile_root.display().to_string(),
            data_root: self.data_root.display().to_string(),
            mock: self.mock,
            configured: !self.store_url.is_empty(),
            recovery_warnings: Vec::new(),
        }
    }
}

pub fn validate_store_endpoint(
    base_url: &str,
    allowed_hosts: &[String],
    allow_debug_mock: bool,
) -> CompanyResult<()> {
    if base_url.is_empty() {
        return Ok(());
    }
    let url = reqwest::Url::parse(base_url)
        .map_err(|_| CompanyError::new("CONFIGURATION_ERROR", "Địa chỉ Store không hợp lệ."))?;
    let host = url.host_str().unwrap_or_default();
    let loopback = matches!(host, "127.0.0.1" | "localhost" | "::1" | "[::1]");
    let mock = allow_debug_mock && loopback;
    if (url.scheme() != "https" && !(mock && url.scheme() == "http"))
        || (!mock
            && (loopback
                || !allowed_hosts
                    .iter()
                    .any(|allowed| allowed.trim().eq_ignore_ascii_case(host))))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || host.ends_with(".example")
        || host.ends_with(".invalid")
        || matches!(host, "example.com" | "example.org" | "example.net")
    {
        return Err(CompanyError::new(
                "CONFIGURATION_ERROR",
                "Store phải dùng HTTPS và host được IT cho phép. HTTP loopback chỉ dùng cho mock debug.",
            ));
    }
    Ok(())
}
