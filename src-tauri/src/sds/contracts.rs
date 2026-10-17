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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    pub profile_id: String,
    pub account_id: String,
    pub generation: u64,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatalogMode {
    #[default]
    AllTime,
    Trending,
    Hot,
}
impl CatalogMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllTime => "all_time",
            Self::Trending => "trending",
            Self::Hot => "hot",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogQuery {
    pub q: String,
    pub mode: CatalogMode,
    pub source_id: Option<String>,
    pub page: u32,
    pub page_size: u32,
}
impl Default for CatalogQuery {
    fn default() -> Self {
        Self {
            q: String::new(),
            mode: CatalogMode::AllTime,
            source_id: None,
            page: 1,
            page_size: 20,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total_items: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mode {
    pub id: CatalogMode,
    pub label: String,
    pub available: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taxonomy {
    pub sources: Vec<Source>,
    pub catalog_modes: Vec<Mode>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub metadata_revision: u64,
    pub roles: Vec<String>,
    pub tags: Vec<String>,
    pub category_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub creator: UserView,
    pub compatible_agents: Option<Vec<String>>,
    pub can_download: bool,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseRef {
    pub id: String,
    pub sequence: u64,
    pub version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    #[serde(flatten)]
    pub metadata: SkillMetadata,
    pub latest_release: Option<ReleaseRef>,
}
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackageFormat {
    Zip,
    #[serde(rename = "7z")]
    SevenZip,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub format: PackageFormat,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub download_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub skill_id: String,
    pub id: String,
    pub sequence: u64,
    pub version: String,
    pub install_name: String,
    pub published_at: String,
    pub changelog: String,
    pub compatible_agents: Option<Vec<String>>,
    pub required_mcps: Option<Vec<String>>,
    pub package: PackageInfo,
}
impl Release {
    pub fn reference(&self) -> ReleaseRef {
        ReleaseRef {
            id: self.id.clone(),
            sequence: self.sequence,
            version: self.version.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub summary: SkillSummary,
    pub store_page_url: Option<String>,
    pub latest_release: Option<Release>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub agent_key: String,
    pub workspace_id: String,
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSettings {
    pub override_root: Option<std::path::PathBuf>,
    pub target: Option<Target>,
    pub error: Option<super::error::SdsError>,
    pub locked: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum McpState {
    NotRequired,
    Unknown,
    Configured,
    Missing,
    Unverifiable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpReport {
    pub state: McpState,
    pub missing: Vec<String>,
    pub reason: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub skill_id: String,
    pub release_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileIdentity {
    pub volume: u64,
    pub file_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewedRoot {
    pub anchor: std::path::PathBuf,
    pub identity: FileIdentity,
    pub root_existed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedOperation {
    pub id: String,
    pub kind: String,
    pub release: Release,
    pub target: Target,
    pub reviewed_root: ReviewedRoot,
    pub expected_fingerprint: Option<String>,
    pub mcp: McpReport,
    pub requires_mcp_ack: bool,
    pub can_execute: bool,
    pub conflict: Option<String>,
    pub context: SessionContext,
    pub runtime_epoch: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationState {
    Prepared,
    Downloading,
    Validating,
    Staging,
    Committing,
    Completed,
    Failed,
    Cancelled,
    RecoveryRequired,
}
impl OperationState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub operation_id: String,
    pub state: OperationState,
    pub stage: String,
    pub error: Option<super::error::SdsError>,
}
