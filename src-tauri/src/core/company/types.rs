use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}
pub type CompanyResult<T> = Result<T, CompanyError>;
impl CompanyError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            request_id: None,
            retry_after_seconds: None,
        }
    }
}
impl std::fmt::Display for CompanyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for CompanyError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum McpState {
    NotRequired,
    Undeclared,
    Present,
    Missing,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpReport {
    pub state: McpState,
    pub required: Vec<String>,
    pub present: Vec<String>,
    pub missing: Vec<String>,
    pub reason: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(alias = "display_name")]
    pub display_name: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub access_token: String,
    pub expires_at: String,
    pub user: User,
    #[serde(default)]
    pub nonce: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub user: Option<User>,
    pub expires_at: Option<String>,
    pub persistent: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMeta {
    pub format: String,
    #[serde(alias = "size_bytes")]
    pub size_bytes: u64,
    pub sha256: String,
    pub etag: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(default, alias = "description_excerpt")]
    pub description_excerpt: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(alias = "updated_at")]
    pub updated_at: String,
    pub package: PackageMeta,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub page: u32,
    #[serde(alias = "page_size")]
    pub page_size: u32,
    pub total: u64,
    #[serde(alias = "total_pages")]
    pub total_pages: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPage {
    pub items: Vec<Skill>,
    pub pagination: Pagination,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<CatalogCache>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCache {
    pub fetched_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    AwaitingDecision,
    Downloading,
    Validating,
    Staging,
    Committing,
    Succeeded,
    Cancelled,
    Failed,
    RecoveryRequired,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    pub id: String,
    pub state: OperationState,
    pub target: String,
    pub mcp: McpReport,
    #[serde(default)]
    pub mcp_path: String,
    #[serde(default)]
    pub skill: Option<Skill>,
    pub progress_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<CompanyError>,
    #[serde(default)]
    pub kind: OperationKind,
    #[serde(default)]
    pub preview_ready: bool,
    #[serde(default)]
    pub customized: bool,
    #[serde(default)]
    pub changes: Vec<FileChange>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    #[default]
    Install,
    Update,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub kind: FileChangeKind,
    #[serde(default)]
    pub current: Option<FilePreview>,
    #[serde(default)]
    pub incoming: Option<FilePreview>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Added,
    Modified,
    Deleted,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub kind: PreviewKind,
    pub size_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    Text,
    Binary,
    Directory,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub skill_id: String,
    pub slug: String,
    pub name: String,
    pub target: String,
    pub package_hash: String,
    pub content_hash: String,
    pub description_snapshot: String,
    pub installed_at: String,
    pub mcp_bypassed: bool,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub content_state: ContentState,
    #[serde(default)]
    pub store_state: StoreState,
    #[serde(default)]
    pub checked_at: Option<String>,
    #[serde(default)]
    pub store_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StoreState {
    Current,
    NewPackage,
    #[default]
    Unknown,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContentState {
    Original,
    Customized,
    #[default]
    Unknown,
    Missing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub installation_id: String,
    pub created_at: String,
    pub reason: String,
    pub path: String,
    pub full_hash: String,
    pub installation: Installation,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReplacePhase {
    Prepared,
    OldMoved,
    NewMoved,
    Committed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceJournal {
    pub view: OperationView,
    pub old: Installation,
    pub new: Installation,
    pub old_full_hash: String,
    pub new_full_hash: String,
    pub backup_id: String,
    pub phase: ReplacePhase,
    pub stage_owned: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallJournal {
    pub view: OperationView,
    pub installation: Installation,
    pub full_hash: String,
    #[serde(default)]
    pub stage_owned: bool,
}
