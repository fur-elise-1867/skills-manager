use super::{error::SdsError, profile::Environment, store_client::canonical_base_url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    SignedOut,
    Authenticated,
    Expired,
    Offline,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub status: SessionStatus,
    pub user: Option<UserView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreProfile {
    pub id: String,
    pub base_url: String,
    pub environment: Environment,
}
impl StoreProfile {
    pub fn new(base_url: &str, environment: Environment) -> Result<Self, SdsError> {
        let base_url = canonical_base_url(base_url, environment)?;
        let id = hex::encode(Sha256::digest(base_url.as_bytes()));
        Ok(Self {
            id,
            base_url,
            environment,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    pub profile_id: String,
    pub account_id: String,
    pub generation: u64,
}
