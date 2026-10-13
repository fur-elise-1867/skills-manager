use super::*;
use crate::core::company::aipro_paths::{overlaps, validate_ancestors};
use std::{collections::BTreeMap, path::PathBuf};

fn changed() -> CompanyError {
    CompanyError::new(
        "CONTENT_CHANGED",
        "Nội dung đã thay đổi sau khi xem. Giữ bản đang dùng và chuẩn bị cập nhật lại.",
    )
}
fn files(root: &Path) -> CompanyResult<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !package_store::excluded(&e.file_name().to_string_lossy()))
    {
        let entry = entry.map_err(|_| write_error())?;
        package_store::reject_links(entry.path())?;
        if entry.path() == root {
            continue;
        }
        let path = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| write_error())?
            .to_string_lossy()
            .replace('\\', "/");
        if entry.file_type().is_file() {
            result.insert(path, package_store::hash_file(entry.path())?);
        } else if entry.file_type().is_dir()
            && fs::read_dir(entry.path())
                .map_err(|_| write_error())?
                .next()
                .is_none()
        {
            result.insert(path, "directory".into());
        }
    }
    Ok(result)
}

impl CompanyService {
    pub fn check_updates(&self, ids: &[String]) -> CompanyResult<Vec<Installation>> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        let mut results = Vec::new();
        for id in ids {
            let mut item = self.refresh_installation(id)?;
            item.checked_at = Some(chrono::Utc::now().to_rfc3339());
            match self.detail(&item.skill_id) {
                Ok(skill) => {
                    item.store_state = if skill.package.sha256 == item.package_hash {
                        StoreState::Current
                    } else {
                        StoreState::NewPackage
                    };
                    item.store_error = None;
                }
                Err(e) if matches!(e.code.as_str(), "UNAUTHENTICATED" | "SESSION_EXPIRED") => {
                    return Err(e)
                }
                Err(e) => {
                    item.store_state = if e.code == "SKILL_NOT_FOUND" {
                        StoreState::Unavailable
                    } else {
                        StoreState::Unknown
                    };
                    item.store_error = Some(e.message);
                }
            }
            self.repository.save_installation(&item)?;
            results.push(item);
        }
        Ok(results)
    }
    pub fn prepare_update(&self, id: &str) -> CompanyResult<OperationView> {
        let owner = self.owner()?;
        let item = self.refresh_installation(id)?;
        if !matches!(
            item.content_state,
            ContentState::Original | ContentState::Customized
        ) {
            return Err(CompanyError::new(
                "CONTENT_UNREADABLE",
                "Chưa kiểm tra được nội dung bản cài. Kiểm tra file/quyền rồi thử lại.",
            ));
        }
        let target = Path::new(&item.target);
        if self
            .operations
            .lock()
            .map_err(|_| operation_error())?
            .values()
            .any(|op| {
                op.view.state == OperationState::RecoveryRequired
                    && overlaps(Path::new(&op.view.target), target)
            })
        {
            return Err(CompanyError::new(
                "RECOVERY_REQUIRED",
                "Đích còn dữ liệu cần phục hồi. Không cập nhật đè.",
            ));
        }
        let old_full_hash = content_hash(target, false)?;
        let skill = self.detail(&item.skill_id)?;
        if skill.slug != item.slug {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Slug Store đã thay đổi. Giữ bản cài để kiểm tra nguồn.",
            ));
        }
        if skill.package.sha256 == item.package_hash {
            return Err(CompanyError::new(
                "NO_UPDATE",
                "Bản cài đang dùng gói hiện tại trên Store.",
            ));
        }
        let description = skill.description.as_ref().ok_or_else(|| {
            CompanyError::new("INVALID_RESPONSE", "Store chưa có description đầy đủ.")
        })?;
        let view = OperationView {
            id: uuid::Uuid::new_v4().to_string(),
            kind: OperationKind::Update,
            state: OperationState::AwaitingDecision,
            target: item.target.clone(),
            mcp: check_global_mcp(&parse_requirements(description), &self.config.profile_root),
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
            preview_ready: false,
            customized: item.content_state == ContentState::Customized,
            changes: Vec::new(),
        };
        self.operations
            .lock()
            .map_err(|_| operation_error())?
            .insert(
                view.id.clone(),
                Operation {
                    view: view.clone(),
                    project_id: item.project_id.clone(),
                    old: Some(item),
                    old_full_hash: Some(old_full_hash),
                    preview: None,
                    skill,
                    owner,
                    cancelled: Arc::new(AtomicBool::new(false)),
                    bypass: false,
                },
            );
        Ok(view)
    }
    pub(super) fn confirm_preview(
        self: &Arc<Self>,
        id: &str,
        bypass: bool,
    ) -> CompanyResult<OperationView> {
        let owner = self.owner()?;
        let (skill, old, hash, cancelled, view) = {
            let mut operations = self.operations.lock().map_err(|_| operation_error())?;
            let op = operations.get_mut(id).ok_or_else(operation_error)?;
            if op.owner != owner || op.cancelled.load(Ordering::Relaxed) {
                return Err(CompanyError::new(
                    "SESSION_EXPIRED",
                    "Quyết định đã hết hiệu lực. Chuẩn bị cập nhật lại.",
                ));
            }
            if op.view.state != OperationState::AwaitingDecision || op.view.preview_ready {
                return Ok(op.view.clone());
            }
            if matches!(op.view.mcp.state, McpState::Missing | McpState::Unknown) && !bypass {
                return Err(CompanyError::new(
                    "MCP_WARNING",
                    "Chọn dừng hoặc bỏ qua cảnh báo MCP.",
                ));
            }
            op.view.state = OperationState::Downloading;
            op.bypass = bypass;
            (
                op.skill.clone(),
                op.old.clone().ok_or_else(operation_error)?,
                op.old_full_hash.clone().ok_or_else(operation_error)?,
                op.cancelled.clone(),
                op.view.clone(),
            )
        };
        let app = self.clone();
        let id = id.to_string();
        std::thread::spawn(move || {
            if let Err(error) =
                app.prepare_preview_files(&id, &skill, &old, &hash, &owner, &cancelled)
            {
                app.update_failure(&id, error);
            }
        });
        Ok(view)
    }
    fn prepare_preview_files(
        &self,
        id: &str,
        skill: &Skill,
        old: &Installation,
        old_hash: &str,
        owner: &str,
        cancelled: &AtomicBool,
    ) -> CompanyResult<()> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        Self::check_cancel(cancelled)?;
        if self.owner()? != owner {
            return Err(CompanyError::new(
                "SESSION_EXPIRED",
                "Chuẩn bị cập nhật lại sau đăng nhập.",
            ));
        }
        let operations_root = self.config.data_root.join("operations");
        validate_ancestors(&operations_root)?;
        fs::create_dir_all(&operations_root).map_err(|_| write_error())?;
        let temporary = tempfile::tempdir_in(&operations_root).map_err(|_| write_error())?;
        let archive = temporary.path().join("download");
        self.download(id, skill, &archive, cancelled)?;
        self.update(id, OperationState::Validating, None, None);
        let package = package_store::prepare_archive(
            &archive,
            &skill.package,
            &skill.slug,
            &self.config.data_root.join("packages"),
        )?;
        self.repository
            .save_package(&skill.package, &package.content_hash, &package.full_hash)?;
        let current = self.guarded_installation(&old.id)?;
        if current.package_hash != old.package_hash
            || content_hash(Path::new(&current.target), false)? != old_hash
        {
            return Err(changed());
        }
        let preview = self.config.data_root.join("previews").join(id);
        validate_ancestors(&preview)?;
        if preview.exists() {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Vùng preview đã có nội dung.",
            ));
        }
        copy_tree(Path::new(&current.target), &preview.join("current"))?;
        copy_tree(&package.original_root, &preview.join("incoming"))?;
        if content_hash(&preview.join("current"), false)? != old_hash
            || content_hash(&preview.join("incoming"), false)? != package.full_hash
            || content_hash(Path::new(&current.target), false)? != old_hash
        {
            return Err(changed());
        }
        let before = files(&preview.join("current"))?;
        let after = files(&preview.join("incoming"))?;
        let mut changes = Vec::new();
        for (path, hash) in &before {
            if after.get(path) != Some(hash) {
                changes.push(FileChange {
                    path: path.clone(),
                    kind: if after.contains_key(path) {
                        "modified"
                    } else {
                        "deleted"
                    }
                    .into(),
                });
            }
        }
        for path in after.keys() {
            if !before.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    kind: "added".into(),
                });
            }
        }
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        Self::check_cancel(cancelled)?;
        if self.owner()? != owner {
            return Err(CompanyError::new(
                "SESSION_EXPIRED",
                "Phiên đã thay đổi. Chuẩn bị cập nhật lại.",
            ));
        }
        let mut operations = self.operations.lock().map_err(|_| operation_error())?;
        let op = operations.get_mut(id).ok_or_else(operation_error)?;
        op.preview = Some(package);
        op.view.preview_ready = true;
        op.view.changes = changes;
        op.view.state = OperationState::AwaitingDecision;
        Ok(())
    }
    pub fn preview_open_path(
        &self,
        id: &str,
        side: &str,
        relative: &str,
    ) -> CompanyResult<PathBuf> {
        uuid::Uuid::parse_str(id).map_err(|_| operation_error())?;
        let owner = self.owner()?;
        let operations = self.operations.lock().map_err(|_| operation_error())?;
        let op = operations.get(id).ok_or_else(operation_error)?;
        if !op.view.preview_ready || op.owner != owner || op.cancelled.load(Ordering::Relaxed) {
            return Err(operation_error());
        }
        if !matches!(side, "current" | "incoming") {
            return Err(operation_error());
        }
        let root = self.config.data_root.join("previews").join(id).join(side);
        let path = if relative.is_empty() {
            root
        } else {
            root.join(package_store::checked_entry(relative)?)
        };
        validate_ancestors(&path)?;
        if !path.exists() {
            return Err(CompanyError::new(
                "CONTENT_MISSING",
                "File không có ở phía đã chọn. Có thể mở thư mục để xem.",
            ));
        }
        Ok(path)
    }
    pub fn apply_update(
        self: &Arc<Self>,
        id: &str,
        replace_customized: bool,
    ) -> CompanyResult<OperationView> {
        let owner = self.owner()?;
        let (old, package, hash, cancelled, skill, bypass, view) = {
            let mut operations = self.operations.lock().map_err(|_| operation_error())?;
            let op = operations.get_mut(id).ok_or_else(operation_error)?;
            if op.owner != owner || op.cancelled.load(Ordering::Relaxed) {
                return Err(CompanyError::new(
                    "SESSION_EXPIRED",
                    "Quyết định đã hết hiệu lực. Chuẩn bị cập nhật lại.",
                ));
            }
            if op.view.kind != OperationKind::Update || !op.view.preview_ready {
                return Err(operation_error());
            }
            if op.view.state != OperationState::AwaitingDecision {
                return Ok(op.view.clone());
            }
            if op.view.customized && !replace_customized {
                return Err(CompanyError::new(
                    "CUSTOMIZED_CONFIRMATION_REQUIRED",
                    "Xác nhận thay bản đã tùy chỉnh bằng gói Store sau khi xem thay đổi.",
                ));
            }
            op.view.state = OperationState::Staging;
            (
                op.old.clone().ok_or_else(operation_error)?,
                op.preview.clone().ok_or_else(operation_error)?,
                op.old_full_hash.clone().ok_or_else(operation_error)?,
                op.cancelled.clone(),
                op.skill.clone(),
                op.bypass,
                op.view.clone(),
            )
        };
        let app = self.clone();
        let id = id.to_string();
        std::thread::spawn(move || {
            match app.replace_installation(
                &id, &old, &package, &hash, &owner, &cancelled, &skill, bypass,
            ) {
                Ok(()) => app.update(
                    &id,
                    OperationState::Succeeded,
                    Some(skill.package.size_bytes),
                    None,
                ),
                Err(error) => app.update_failure(&id, error),
            }
        });
        Ok(view)
    }
    pub(super) fn update_failure(&self, id: &str, error: CompanyError) {
        self.auth_result::<()>(Err(error.clone())).ok();
        let state = match error.code.as_str() {
            "CANCELLED" => OperationState::Cancelled,
            "RECOVERY_REQUIRED" => OperationState::RecoveryRequired,
            _ => OperationState::Failed,
        };
        self.update(id, state, None, Some(error));
    }
    pub fn backups(&self, installation_id: &str) -> CompanyResult<Vec<Backup>> {
        self.guarded_installation(installation_id)?;
        self.repository.backups(installation_id)
    }
}
