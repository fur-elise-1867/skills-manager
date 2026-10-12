use super::{
    aipro_paths::{global_target, stage_path},
    config::CompanyConfig,
    installation_repository::CompanyRepository,
    mcp_check::{check_global_mcp, parse_requirements},
    package_store::{self, content_hash, copy_tree, MAX_ARCHIVE},
    session::{CredentialStore, SessionService},
    store_client::StoreClient,
    types::*,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

struct Operation {
    view: OperationView,
    skill: Skill,
    owner: String,
    cancelled: Arc<AtomicBool>,
}
pub struct CompanyService {
    pub config: CompanyConfig,
    session: SessionService,
    client: StoreClient,
    repository: CompanyRepository,
    operations: Mutex<HashMap<String, Operation>>,
    worker: Mutex<()>,
}
fn operation_error() -> CompanyError {
    CompanyError::new(
        "OPERATION_NOT_FOUND",
        "Không tìm thấy thao tác cài đặt. Bắt đầu cài lại để lấy thông tin mới.",
    )
}
fn write_error() -> CompanyError {
    CompanyError::new(
        "WRITE_FAILED",
        "Không ghi được bản cài. Kiểm tra quyền, dung lượng và file đang khóa.",
    )
}
impl CompanyService {
    pub fn new(
        config: CompanyConfig,
        credentials: Arc<dyn CredentialStore>,
    ) -> CompanyResult<Self> {
        let session = SessionService::new(&config, credentials)?;
        let repository = CompanyRepository::open(&config.data_root.join("manager.db"))?;
        let app = Self {
            client: StoreClient::new(&config)?,
            config,
            session,
            repository,
            operations: Mutex::new(HashMap::new()),
            worker: Mutex::new(()),
        };
        app.recover()?;
        Ok(app)
    }
    pub fn current_session(&self) -> SessionView {
        self.session.current()
    }
    pub fn login(&self, username: &str, password: &str) -> CompanyResult<SessionView> {
        self.repository.clear_catalog(&self.config.store_url)?;
        self.session.login(username, password)
    }
    pub fn logout(&self) -> CompanyResult<()> {
        self.cancel_pending();
        let cache_result = self.repository.clear_catalog(&self.config.store_url);
        let logout_result = self.session.logout();
        cache_result.and(logout_result)
    }
    pub fn installations(&self) -> CompanyResult<Vec<Installation>> {
        self.repository.installations()
    }
    pub fn search(&self, query: &str, page: u32) -> CompanyResult<SkillPage> {
        let token = self.session.access_token()?;
        let user = self
            .session
            .current()
            .user
            .ok_or_else(|| CompanyError::new("SESSION_EXPIRED", "Vui lòng đăng nhập lại."))?;
        let result = self.auth_result(self.client.search(&token, query, page));
        if self.session.access_token().ok().as_deref() != Some(token.as_str()) {
            return Err(CompanyError::new(
                "SESSION_EXPIRED",
                "Phiên đã thay đổi. Vui lòng thử lại.",
            ));
        }
        match result {
            Ok(page_result) => {
                self.repository.cache_catalog(
                    &self.config.store_url,
                    &user.id,
                    query,
                    page,
                    &page_result,
                )?;
                Ok(page_result)
            }
            Err(error) if matches!(error.code.as_str(), "NETWORK_ERROR" | "STORE_UNAVAILABLE") => {
                self.repository
                    .cached_catalog(&self.config.store_url, &user.id, query, page)?
                    .ok_or(error)
            }
            Err(error) => Err(error),
        }
    }
    pub fn detail(&self, id: &str) -> CompanyResult<Skill> {
        self.auth_result(self.client.detail(&self.session.access_token()?, id))
    }
    fn auth_result<T>(&self, result: CompanyResult<T>) -> CompanyResult<T> {
        if result
            .as_ref()
            .err()
            .map(|e| matches!(e.code.as_str(), "UNAUTHENTICATED" | "SESSION_EXPIRED"))
            .unwrap_or(false)
        {
            self.cancel_pending();
            self.session.clear();
            self.repository.clear_catalog(&self.config.store_url)?;
        }
        result
    }
    fn cancel_pending(&self) {
        if let Ok(operations) = self.operations.lock() {
            for operation in operations.values() {
                if !matches!(
                    operation.view.state,
                    OperationState::Committing | OperationState::Succeeded
                ) {
                    operation.cancelled.store(true, Ordering::Relaxed);
                }
            }
        }
    }
    fn owner(&self) -> CompanyResult<String> {
        Ok(hex::encode(Sha256::digest(
            self.session.access_token()?.as_bytes(),
        )))
    }
    pub fn prepare_install(&self, id: &str) -> CompanyResult<OperationView> {
        let owner = self.owner()?;
        let skill = self.detail(id)?;
        let description = skill.description.as_ref().ok_or_else(|| {
            CompanyError::new(
                "INVALID_RESPONSE",
                "Detail của Store thiếu description đầy đủ.",
            )
        })?;
        let target = global_target(&self.config.profile_root, &skill.slug)?;
        let target_key = target.to_string_lossy().replace('\\', "/");
        if self
            .operations
            .lock()
            .map_err(|_| operation_error())?
            .values()
            .any(|operation| {
                operation.view.state == OperationState::RecoveryRequired
                    && operation
                        .view
                        .target
                        .replace('\\', "/")
                        .eq_ignore_ascii_case(&target_key)
            })
        {
            return Err(CompanyError::new("RECOVERY_REQUIRED", "Đích này có thao tác chưa phục hồi xong. Kiểm tra dữ liệu đã giữ trước khi cài lại."));
        }
        if target.exists()
            || self.installations()?.iter().any(|item| {
                item.target
                    .replace('\\', "/")
                    .eq_ignore_ascii_case(&target.to_string_lossy().replace('\\', "/"))
            })
        {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Đích đã có nội dung. Xem bản đã cài hoặc xử lý đích này trước khi cài lại.",
            ));
        }
        if skill.package.size_bytes > MAX_ARCHIVE
            || !matches!(skill.package.format.as_str(), "zip" | "7z")
            || !skill.package.etag.starts_with('"')
            || !skill.package.etag.ends_with('"')
        {
            return Err(CompanyError::new(
                "INVALID_PACKAGE",
                "Metadata gói không hợp lệ hoặc vượt giới hạn.",
            ));
        }
        let mcp = check_global_mcp(&parse_requirements(description), &self.config.profile_root);
        let view = OperationView {
            id: uuid::Uuid::new_v4().to_string(),
            state: OperationState::AwaitingDecision,
            target: target.display().to_string(),
            mcp,
            mcp_path: self
                .config
                .profile_root
                .join(".aipro/mcp.json")
                .display()
                .to_string(),
            skill: Some(skill.clone()),
            progress_bytes: 0,
            total_bytes: skill.package.size_bytes,
            error: None,
        };
        self.operations
            .lock()
            .map_err(|_| operation_error())?
            .insert(
                view.id.clone(),
                Operation {
                    view: view.clone(),
                    skill,
                    owner,
                    cancelled: Arc::new(AtomicBool::new(false)),
                },
            );
        Ok(view)
    }
    pub fn status(&self, id: &str) -> CompanyResult<OperationView> {
        self.operations
            .lock()
            .map_err(|_| operation_error())?
            .get(id)
            .map(|op| op.view.clone())
            .ok_or_else(operation_error)
    }
    pub fn cancel(&self, id: &str) -> CompanyResult<OperationView> {
        let mut operations = self.operations.lock().map_err(|_| operation_error())?;
        let operation = operations.get_mut(id).ok_or_else(operation_error)?;
        if !matches!(
            operation.view.state,
            OperationState::Committing | OperationState::Succeeded
        ) {
            operation.cancelled.store(true, Ordering::Relaxed);
            if operation.view.state == OperationState::AwaitingDecision {
                operation.view.state = OperationState::Cancelled;
            }
        }
        Ok(operation.view.clone())
    }
    pub fn confirm(self: &Arc<Self>, id: &str, bypass_mcp: bool) -> CompanyResult<OperationView> {
        let owner = self.owner()?;
        let (skill, cancelled, view) = {
            let mut operations = self.operations.lock().map_err(|_| operation_error())?;
            let operation = operations.get_mut(id).ok_or_else(operation_error)?;
            if operation.cancelled.load(Ordering::Relaxed) {
                return Err(CompanyError::new(
                    "CANCELLED",
                    "Thao tác đã hủy. Bắt đầu cài lại để kiểm tra thông tin mới.",
                ));
            }
            if operation.owner != owner {
                return Err(CompanyError::new(
                    "SESSION_EXPIRED",
                    "Quyết định cài đã hết hiệu lực. Kiểm tra lại skill và MCP.",
                ));
            }
            if operation.view.state != OperationState::AwaitingDecision {
                return Ok(operation.view.clone());
            }
            if matches!(
                operation.view.mcp.state,
                McpState::Missing | McpState::Unknown
            ) && !bypass_mcp
            {
                return Err(CompanyError::new(
                    "MCP_WARNING",
                    "Cần chọn dừng hoặc bỏ qua cảnh báo MCP.",
                ));
            }
            operation.view.state = OperationState::Downloading;
            (
                operation.skill.clone(),
                operation.cancelled.clone(),
                operation.view.clone(),
            )
        };
        let app = Arc::clone(self);
        let id = id.to_string();
        std::thread::spawn(move || {
            let result = app.install(&id, &skill, &owner, &cancelled, bypass_mcp);
            if let Err(error) = result {
                app.auth_result::<()>(Err(error.clone())).ok();
                let state = if error.code == "CANCELLED" {
                    OperationState::Cancelled
                } else if error.code == "RECOVERY_REQUIRED" {
                    OperationState::RecoveryRequired
                } else {
                    OperationState::Failed
                };
                app.update(&id, state, None, Some(error));
            } else {
                app.update(
                    &id,
                    OperationState::Succeeded,
                    Some(skill.package.size_bytes),
                    None,
                );
            }
        });
        Ok(view)
    }
    fn update(
        &self,
        id: &str,
        state: OperationState,
        bytes: Option<u64>,
        error: Option<CompanyError>,
    ) {
        if let Ok(mut operations) = self.operations.lock() {
            if let Some(operation) = operations.get_mut(id) {
                operation.view.state = state;
                operation.view.error = error;
                if let Some(bytes) = bytes {
                    operation.view.progress_bytes = bytes;
                }
            }
        }
    }
    fn check_cancel(cancelled: &AtomicBool) -> CompanyResult<()> {
        if cancelled.load(Ordering::Relaxed) {
            Err(CompanyError::new("CANCELLED", "Đã hủy cài đặt."))
        } else {
            Ok(())
        }
    }
    fn install(
        &self,
        id: &str,
        skill: &Skill,
        owner: &str,
        cancelled: &AtomicBool,
        bypass: bool,
    ) -> CompanyResult<()> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        Self::check_cancel(cancelled)?;
        if self.owner()? != owner {
            return Err(CompanyError::new(
                "SESSION_EXPIRED",
                "Cần đăng nhập và chuẩn bị cài lại.",
            ));
        }
        let operations_root = self.config.data_root.join("operations");
        fs::create_dir_all(&operations_root).map_err(|_| write_error())?;
        package_store::reject_links(&operations_root)?;
        let temporary = tempfile::tempdir_in(&operations_root).map_err(|_| write_error())?;
        let archive = temporary.path().join("download");
        self.download(id, skill, &archive, cancelled)?;
        Self::check_cancel(cancelled)?;
        self.update(id, OperationState::Validating, None, None);
        let package = package_store::prepare_archive(
            &archive,
            &skill.package,
            &skill.slug,
            &self.config.data_root.join("packages"),
        )?;
        self.repository
            .save_package(&skill.package, &package.content_hash, &package.full_hash)?;
        Self::check_cancel(cancelled)?;
        let target = global_target(&self.config.profile_root, &skill.slug)?;
        if target.exists() {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Đích đã có nội dung kể từ lúc kiểm tra. Không ghi đè.",
            ));
        }
        let stage = stage_path(&target, id)?;
        fs::create_dir_all(target.parent().ok_or_else(write_error)?).map_err(|_| write_error())?;
        global_target(&self.config.profile_root, &skill.slug)?;
        let view = self.status(id)?;
        let installation = Installation {
            id: id.into(),
            skill_id: skill.id.clone(),
            slug: skill.slug.clone(),
            name: skill.name.clone(),
            target: target.display().to_string(),
            package_hash: package.archive_hash,
            content_hash: package.content_hash,
            description_snapshot: skill.description.clone().unwrap_or_default(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            mcp_bypassed: bypass,
        };
        let mut journal = InstallJournal {
            view,
            installation,
            full_hash: package.full_hash.clone(),
            stage_owned: false,
        };
        self.repository.prepare(&journal)?;
        self.update(id, OperationState::Staging, None, None);
        let mut owned_stage = false;
        let before_rename = (|| {
            if stage.exists() {
                return Err(CompanyError::new(
                    "TARGET_CONFLICT",
                    "Thư mục staging đã tồn tại.",
                ));
            }
            fs::create_dir(&stage).map_err(|_| write_error())?;
            owned_stage = true;
            journal.stage_owned = true;
            self.repository.stage_created(&journal)?;
            copy_tree(&package.original_root, &stage)?;
            if content_hash(&stage, false)? != package.full_hash {
                return Err(CompanyError::new(
                    "PACKAGE_INTEGRITY_FAILED",
                    "Bản sao không khớp gói nguồn.",
                ));
            }
            Self::check_cancel(cancelled)?;
            if self.owner()? != owner {
                return Err(CompanyError::new(
                    "SESSION_EXPIRED",
                    "Phiên đã thay đổi; chuẩn bị cài lại.",
                ));
            }
            global_target(&self.config.profile_root, &skill.slug)?;
            if target.exists() {
                return Err(CompanyError::new(
                    "TARGET_CONFLICT",
                    "Đích đã có nội dung. Không ghi đè.",
                ));
            }
            self.update(id, OperationState::Committing, None, None);
            fs::rename(&stage, &target).map_err(|_| write_error())
        })();
        if let Err(error) = before_rename {
            if owned_stage && stage.exists() && package_store::reject_links(&stage).is_ok() {
                let _ = fs::remove_dir_all(&stage);
            }
            let _ = self.repository.finish(id, "cancelled");
            return Err(error);
        }
        self.repository.commit(&journal).map_err(|_| {
            CompanyError::new(
                "RECOVERY_REQUIRED",
                "File đã được giữ tại đích; mở lại app để hoàn tất khôi phục dữ liệu quản lý.",
            )
        })
    }
    fn download(
        &self,
        id: &str,
        skill: &Skill,
        archive: &Path,
        cancelled: &AtomicBool,
    ) -> CompanyResult<()> {
        let mut response = self.client.package_response(
            &self.session.access_token()?,
            &skill.id,
            &skill.package.etag,
        )?;
        let mut output = File::create(archive).map_err(|_| write_error())?;
        let mut bytes = 0u64;
        let mut buffer = [0u8; 65_536];
        loop {
            Self::check_cancel(cancelled)?;
            let count = response
                .read(&mut buffer)
                .map_err(|_| StoreClient::network_error())?;
            if count == 0 {
                break;
            }
            bytes += count as u64;
            if bytes > MAX_ARCHIVE || bytes > skill.package.size_bytes {
                return Err(CompanyError::new(
                    "PACKAGE_INTEGRITY_FAILED",
                    "Gói tải vượt kích thước đã khai báo.",
                ));
            }
            output
                .write_all(&buffer[..count])
                .map_err(|_| write_error())?;
            self.update(id, OperationState::Downloading, Some(bytes), None);
        }
        output.sync_all().map_err(|_| write_error())?;
        if bytes != skill.package.size_bytes {
            return Err(CompanyError::new(
                "PACKAGE_INTEGRITY_FAILED",
                "Gói tải chưa đủ bytes.",
            ));
        }
        Ok(())
    }
    fn recover_install(&self, journal: &InstallJournal) -> CompanyResult<bool> {
        let target = global_target(&self.config.profile_root, &journal.installation.slug)?;
        let stage = stage_path(&target, &journal.view.id)?;
        let matches_target = target
            .to_string_lossy()
            .replace('\\', "/")
            .eq_ignore_ascii_case(&journal.installation.target.replace('\\', "/"));
        if journal.stage_owned
            && matches_target
            && target.exists()
            && !stage.exists()
            && content_hash(&target, false).ok().as_deref() == Some(journal.full_hash.as_str())
        {
            self.repository.commit(journal)?;
            return Ok(true);
        } else if matches_target
            && !target.exists()
            && (!stage.exists()
                || (journal.stage_owned
                    && content_hash(&stage, false).ok().as_deref()
                        == Some(journal.full_hash.as_str())))
        {
            if stage.exists() {
                package_store::reject_links(&stage)?;
                fs::remove_dir_all(&stage).map_err(|_| write_error())?;
            }
            self.repository.finish(&journal.view.id, "cancelled")?;
            return Ok(true);
        }
        Ok(false)
    }
    fn recover(&self) -> CompanyResult<()> {
        for journal in self.repository.pending()? {
            if !self.recover_install(&journal).unwrap_or(false) {
                self.repository
                    .finish(&journal.view.id, "recovery_required")?;
                let mut view = journal.view.clone();
                view.state = OperationState::RecoveryRequired;
                view.error = Some(CompanyError::new(
                    "RECOVERY_REQUIRED",
                    format!(
                        "Nội dung thao tác cần kiểm tra. Đã giữ dữ liệu: {} ; staging: {}",
                        view.target,
                        format!(".company-stage-{}", view.id)
                    ),
                ));
                let skill = Skill {
                    id: journal.installation.skill_id,
                    slug: journal.installation.slug,
                    name: journal.installation.name,
                    description_excerpt: None,
                    description: Some(journal.installation.description_snapshot),
                    updated_at: journal.installation.installed_at,
                    package: PackageMeta {
                        format: "zip".into(),
                        size_bytes: 0,
                        sha256: journal.installation.package_hash,
                        etag: String::new(),
                    },
                };
                self.operations
                    .lock()
                    .map_err(|_| operation_error())?
                    .insert(
                        view.id.clone(),
                        Operation {
                            view,
                            skill,
                            owner: String::new(),
                            cancelled: Arc::new(AtomicBool::new(true)),
                        },
                    );
            }
        }
        Ok(())
    }
    pub fn recovery_warnings(&self) -> Vec<String> {
        self.operations
            .lock()
            .map(|operations| {
                operations
                    .values()
                    .filter(|op| op.view.state == OperationState::RecoveryRequired)
                    .filter_map(|op| op.view.error.as_ref().map(|error| error.message.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
