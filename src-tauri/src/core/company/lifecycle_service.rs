use super::*;
use crate::core::company::aipro_paths::{overlaps, path_key, validate_ancestors};
use std::path::PathBuf;

fn conflict() -> CompanyError {
    CompanyError::new(
        "TARGET_CONFLICT",
        "Đích đã có bản cài hoặc nội dung khác. Không ghi đè.",
    )
}
fn changed() -> CompanyError {
    CompanyError::new(
        "CONTENT_CHANGED",
        "Nội dung đã thay đổi sau khi xem. Chuẩn bị thao tác lại.",
    )
}
fn recovery_error() -> CompanyError {
    CompanyError::new(
        "RECOVERY_REQUIRED",
        "Đã giữ dữ liệu khôi phục/gỡ cần kiểm tra. Mở lại app để phục hồi.",
    )
}
fn absent(path: &Path) -> bool {
    fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
}
fn matches(path: &Path, hash: &str) -> bool {
    path.is_dir()
        && validate_ancestors(path).is_ok()
        && content_hash(path, false).ok().as_deref() == Some(hash)
}
fn snapshot_skill(item: &Installation) -> Skill {
    Skill {
        id: item.skill_id.clone().unwrap_or_default(),
        slug: item.slug.clone(),
        name: item.name.clone(),
        description_excerpt: None,
        description: Some(item.description_snapshot.clone()),
        updated_at: item.installed_at.clone(),
        package: PackageMeta {
            format: "zip".into(),
            size_bytes: 0,
            sha256: item.package_hash.clone(),
            etag: String::new(),
        },
    }
}

impl CompanyService {
    pub(super) fn verify_backup(&self, backup: &Backup) -> CompanyResult<PathBuf> {
        let invalid = || {
            CompanyError::new(
                "BACKUP_INVALID",
                "Bản lưu thiếu, đã thay đổi hoặc không hợp lệ. Giữ bản đang dùng.",
            )
        };
        uuid::Uuid::parse_str(&backup.id).map_err(|_| invalid())?;
        uuid::Uuid::parse_str(&backup.installation_id).map_err(|_| invalid())?;
        if backup.installation_id != backup.installation.id {
            return Err(invalid());
        }
        let content = self
            .config
            .data_root
            .join("backups")
            .join(&backup.installation_id)
            .join(&backup.id)
            .join("content");
        if path_key(&content) != path_key(Path::new(&backup.path))
            || !matches(&content, &backup.full_hash)
        {
            return Err(invalid());
        }
        let metadata: Backup = serde_json::from_slice(
            &fs::read(content.parent().unwrap().join("metadata.json")).map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
        if serde_json::to_value(metadata).ok() != serde_json::to_value(backup).ok() {
            return Err(invalid());
        }
        Ok(content)
    }
    pub fn prepare_restore(&self, backup_id: &str) -> CompanyResult<OperationView> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        let backup = self
            .repository
            .all_backups()?
            .into_iter()
            .find(|b| b.id == backup_id)
            .ok_or_else(|| CompanyError::new("BACKUP_NOT_FOUND", "Không tìm thấy bản lưu."))?;
        self.verify_backup(&backup)?;
        let old = self.guarded_installation(&backup.installation_id)?;
        if old.target != backup.installation.target
            || old.slug != backup.installation.slug
            || old.project_id != backup.installation.project_id
        {
            return Err(conflict());
        }
        let mut new = backup.installation.clone();
        new.state = InstallationState::Active;
        new.store_state = if new.origin == Origin::Local {
            StoreState::NotApplicable
        } else {
            StoreState::Unknown
        };
        new.checked_at = None;
        new.store_error = None;
        self.prepare_lifecycle(old, new, Some(backup), OperationKind::Restore)
    }
    pub fn prepare_remove(&self, installation_id: &str) -> CompanyResult<OperationView> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        let old = self.guarded_installation(installation_id)?;
        if old.state != InstallationState::Active {
            return Err(CompanyError::new("ALREADY_REMOVED", "Bản cài đã gỡ."));
        }
        let mut new = old.clone();
        new.state = InstallationState::Removed;
        new.content_state = ContentState::Missing;
        self.prepare_lifecycle(old, new, None, OperationKind::Remove)
    }
    fn check_lifecycle_target(&self, old: &Installation) -> CompanyResult<PathBuf> {
        let target = self.installation_target(old.project_id.as_deref(), &old.slug)?;
        if path_key(&target) != path_key(Path::new(&old.target)) {
            return Err(conflict());
        }
        if self.repository.installations()?.iter().any(|i| {
            i.state == InstallationState::Active
                && i.id != old.id
                && overlaps(Path::new(&i.target), &target)
        }) {
            return Err(conflict());
        }
        if old.state == InstallationState::Removed && !absent(&target) {
            return Err(conflict());
        }
        Ok(target)
    }
    fn prepare_lifecycle(
        &self,
        old: Installation,
        new: Installation,
        source: Option<Backup>,
        kind: OperationKind,
    ) -> CompanyResult<OperationView> {
        let target = self.check_lifecycle_target(&old)?;
        if self
            .operations
            .lock()
            .map_err(|_| operation_error())?
            .values()
            .any(|op| {
                !matches!(
                    op.view.state,
                    OperationState::Succeeded | OperationState::Failed | OperationState::Cancelled
                ) && overlaps(Path::new(&op.view.target), &target)
            })
        {
            return Err(CompanyError::new(
                "TARGET_BUSY",
                "Đích có thao tác chưa hoàn tất. Đóng hoặc hủy thao tác đó trước.",
            ));
        }
        let old_hash = if absent(&target) {
            None
        } else {
            Some(content_hash(&target, false)?)
        };
        let view = OperationView {
            id: uuid::Uuid::new_v4().to_string(),
            state: OperationState::AwaitingDecision,
            target: old.target.clone(),
            mcp: McpReport {
                state: McpState::Undeclared,
                required: vec![],
                present: vec![],
                missing: vec![],
                reason: Some("Snapshot cục bộ; không kiểm tra lại MCP/Store.".into()),
                checked_at: String::new(),
            },
            mcp_path: String::new(),
            skill: None,
            progress_bytes: 0,
            total_bytes: 0,
            error: None,
            kind,
            preview_ready: true,
            customized: self.refreshed(old.clone()).content_state == ContentState::Customized,
            changes: vec![],
        };
        let journal = LifecycleJournal {
            view: view.clone(),
            old: old.clone(),
            new,
            old_full_hash: old_hash,
            new_full_hash: source.as_ref().map(|b| b.full_hash.clone()),
            current_backup: None,
            source_backup: source,
            phase: ReplacePhase::Prepared,
            stage_owned: false,
        };
        self.operations
            .lock()
            .map_err(|_| operation_error())?
            .insert(
                view.id.clone(),
                Operation {
                    view: view.clone(),
                    skill: snapshot_skill(&old),
                    owner: String::new(),
                    cancelled: Arc::new(AtomicBool::new(false)),
                    project_id: old.project_id.clone(),
                    old: Some(old),
                    old_full_hash: journal.old_full_hash.clone(),
                    preview: None,
                    bypass: false,
                },
            );
        self.lifecycle_jobs
            .lock()
            .map_err(|_| operation_error())?
            .insert(view.id.clone(), journal);
        Ok(view)
    }
    pub fn apply_lifecycle(self: &Arc<Self>, id: &str) -> CompanyResult<OperationView> {
        let journal = self
            .lifecycle_jobs
            .lock()
            .map_err(|_| operation_error())?
            .get(id)
            .cloned()
            .ok_or_else(operation_error)?;
        let (cancelled, view) = {
            let mut ops = self.operations.lock().map_err(|_| operation_error())?;
            let op = ops.get_mut(id).ok_or_else(operation_error)?;
            Self::check_cancel(&op.cancelled)?;
            if op.view.state != OperationState::AwaitingDecision {
                return Ok(op.view.clone());
            }
            op.view.state = OperationState::Staging;
            (op.cancelled.clone(), op.view.clone())
        };
        let app = self.clone();
        let id = id.to_string();
        std::thread::spawn(move || match app.run_lifecycle(journal, &cancelled) {
            Ok(()) => app.update(&id, OperationState::Succeeded, None, None),
            Err(error) => {
                let state = match error.code.as_str() {
                    "CANCELLED" => OperationState::Cancelled,
                    "RECOVERY_REQUIRED" => OperationState::RecoveryRequired,
                    _ => OperationState::Failed,
                };
                app.update(&id, state, None, Some(error));
            }
        });
        Ok(view)
    }
    fn lifecycle_paths(&self, j: &LifecycleJournal) -> CompanyResult<(PathBuf, PathBuf, PathBuf)> {
        if j.old.id != j.new.id
            || j.old.target != j.new.target
            || j.old.project_id != j.new.project_id
            || j.old.slug != j.new.slug
        {
            return Err(recovery_error());
        }
        let target = self.installation_target(j.old.project_id.as_deref(), &j.old.slug)?;
        if path_key(&target) != path_key(Path::new(&j.old.target)) {
            return Err(recovery_error());
        }
        let stage = stage_path(&target, &j.view.id)?;
        let rollback = target
            .parent()
            .ok_or_else(write_error)?
            .join(format!(".company-rollback-{}", j.view.id));
        validate_ancestors(&rollback)?;
        Ok((target, stage, rollback))
    }
    fn run_lifecycle(&self, mut j: LifecycleJournal, cancelled: &AtomicBool) -> CompanyResult<()> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        Self::check_cancel(cancelled)?;
        let current = self.guarded_installation(&j.old.id)?;
        if current.package_hash != j.old.package_hash
            || current.state != j.old.state
            || current.content_hash != j.old.content_hash
        {
            return Err(changed());
        }
        let target = self.check_lifecycle_target(&j.old)?;
        let expected_old = j.old_full_hash.clone();
        let old_matches = |path: &Path| match &expected_old {
            Some(hash) => matches(path, hash),
            None => absent(path),
        };
        if !old_matches(&target) {
            return Err(changed());
        }
        let source = j
            .source_backup
            .as_ref()
            .map(|b| self.verify_backup(b))
            .transpose()?;
        if let Some(hash) = &j.old_full_hash {
            j.current_backup = Some(self.snapshot_backup(
                &current,
                hash,
                if j.view.kind == OperationKind::Remove {
                    "remove"
                } else {
                    "restore"
                },
            )?);
        }
        let (target, stage, rollback) = self.lifecycle_paths(&j)?;
        if !absent(&stage) || !absent(&rollback) {
            return Err(conflict());
        }
        validate_ancestors(target.parent().ok_or_else(write_error)?)?;
        fs::create_dir_all(target.parent().ok_or_else(write_error)?).map_err(|_| write_error())?;
        self.repository.record_lifecycle(&j)?;
        let mut moved = false;
        let result = (|| {
            if let Some(source) = &source {
                fs::create_dir(&stage).map_err(|_| write_error())?;
                j.stage_owned = true;
                self.repository.record_lifecycle(&j)?;
                copy_tree(source, &stage)?;
                if !matches(
                    &stage,
                    j.new_full_hash.as_deref().ok_or_else(recovery_error)?,
                ) {
                    return Err(write_error());
                }
                self.verify_backup(j.source_backup.as_ref().unwrap())?;
            }
            Self::check_cancel(cancelled)?;
            self.check_lifecycle_target(&j.old)?;
            if !old_matches(&target) {
                return Err(changed());
            }
            self.enter_commit(&j.view.id)?;
            if j.old_full_hash.is_some() {
                fs::rename(&target, &rollback).map_err(|_| write_error())?;
                moved = true;
                j.phase = ReplacePhase::OldMoved;
                self.repository.record_lifecycle(&j)?;
                if !matches(&rollback, j.old_full_hash.as_deref().unwrap()) {
                    if absent(&target) && rollback.is_dir() && validate_ancestors(&rollback).is_ok()
                    {
                        fs::rename(&rollback, &target).map_err(|_| recovery_error())?;
                        moved = false;
                    }
                    return Err(changed());
                }
            }
            if !absent(&target) {
                return Err(conflict());
            }
            if j.new_full_hash.is_some() {
                if !matches(&stage, j.new_full_hash.as_deref().unwrap()) {
                    return Err(recovery_error());
                }
                fs::rename(&stage, &target).map_err(|_| write_error())?;
                moved = true;
                if !matches(&target, j.new_full_hash.as_deref().unwrap()) {
                    return Err(recovery_error());
                }
            }
            j.phase = ReplacePhase::NewMoved;
            self.repository.record_lifecycle(&j)?;
            j.phase = ReplacePhase::Committed;
            self.repository.commit_lifecycle(&j)?;
            Ok(())
        })();
        if let Err(error) = result {
            if !moved {
                if j.stage_owned && !absent(&stage) {
                    if !j
                        .new_full_hash
                        .as_ref()
                        .is_some_and(|hash| matches(&stage, hash))
                    {
                        return Err(recovery_error());
                    }
                    cleanup::remove_verified_tree(
                        &stage,
                        j.new_full_hash.as_deref().ok_or_else(recovery_error)?,
                    )
                    .map_err(|_| recovery_error())?;
                }
                self.repository.finish_lifecycle(&j.view.id)?;
                return Err(error);
            }
            let durable = self
                .repository
                .pending_lifecycle()?
                .into_iter()
                .find(|p| p.view.id == j.view.id)
                .ok_or_else(recovery_error)?;
            if !self.recover_lifecycle_one(&durable).unwrap_or(false) {
                return Err(recovery_error());
            }
            return Err(error);
        }
        if !self.recover_lifecycle_one(&j).unwrap_or(false) {
            return Err(recovery_error());
        }
        Ok(())
    }
    pub(super) fn recover_lifecycle_one(&self, j: &LifecycleJournal) -> CompanyResult<bool> {
        let (target, stage, rollback) = self.lifecycle_paths(j)?;
        if self.repository.installations()?.iter().any(|item| {
            item.id != j.old.id
                && item.state == InstallationState::Active
                && overlaps(Path::new(&item.target), &target)
        }) {
            return Ok(false);
        }
        if let Some(backup) = &j.current_backup {
            if self.verify_backup(backup).is_err()
                || Some(backup.full_hash.as_str()) != j.old_full_hash.as_deref()
            {
                return Ok(false);
            }
        } else if j.old_full_hash.is_some() {
            return Ok(false);
        }
        if !absent(&stage)
            && (!j.stage_owned || !j.new_full_hash.as_ref().is_some_and(|h| matches(&stage, h)))
        {
            return Ok(false);
        }
        if !absent(&rollback)
            && !j
                .old_full_hash
                .as_ref()
                .is_some_and(|h| matches(&rollback, h))
        {
            return Ok(false);
        }
        let current = self.guarded_installation(&j.old.id)?;
        if j.phase == ReplacePhase::Committed {
            if serde_json::to_value(&current).ok() != serde_json::to_value(&j.new).ok() {
                return Ok(false);
            }
            if !absent(&stage) {
                cleanup::remove_verified_tree(
                    &stage,
                    j.new_full_hash.as_deref().ok_or_else(recovery_error)?,
                )?;
            }
            if !absent(&rollback) {
                cleanup::remove_verified_tree(
                    &rollback,
                    j.old_full_hash.as_deref().ok_or_else(recovery_error)?,
                )?;
            }
        } else {
            if serde_json::to_value(&current).ok() != serde_json::to_value(&j.old).ok() {
                return Ok(false);
            }
            if !absent(&rollback) || j.old_full_hash.is_none() {
                if !absent(&target) {
                    if !absent(&stage)
                        || !j.stage_owned
                        || !j
                            .new_full_hash
                            .as_ref()
                            .is_some_and(|h| matches(&target, h))
                    {
                        return Ok(false);
                    }
                    fs::rename(&target, &stage).map_err(|_| write_error())?;
                }
                if !absent(&rollback) {
                    fs::rename(&rollback, &target).map_err(|_| write_error())?;
                }
            } else if !j
                .old_full_hash
                .as_ref()
                .is_some_and(|h| matches(&target, h))
            {
                return Ok(false);
            }
            if !absent(&stage) {
                cleanup::remove_verified_tree(
                    &stage,
                    j.new_full_hash.as_deref().ok_or_else(recovery_error)?,
                )?;
            }
        }
        self.repository.finish_lifecycle(&j.view.id)?;
        Ok(true)
    }
    pub(super) fn recover_lifecycle(&self) -> CompanyResult<()> {
        for journal in self.repository.pending_lifecycle()? {
            if self.recover_lifecycle_one(&journal).unwrap_or(false) {
                continue;
            }
            let mut view = journal.view.clone();
            view.state = OperationState::RecoveryRequired;
            view.error = Some(CompanyError::new(
                "RECOVERY_REQUIRED",
                format!(
                    "Đã giữ dữ liệu khôi phục/gỡ tại {} và .company-rollback-{}.",
                    view.target, view.id
                ),
            ));
            self.operations
                .lock()
                .map_err(|_| operation_error())?
                .insert(
                    view.id.clone(),
                    Operation {
                        view,
                        skill: snapshot_skill(&journal.old),
                        owner: String::new(),
                        cancelled: Arc::new(AtomicBool::new(true)),
                        project_id: journal.old.project_id.clone(),
                        old: Some(journal.old),
                        old_full_hash: journal.old_full_hash,
                        preview: None,
                        bypass: false,
                    },
                );
        }
        Ok(())
    }
}
