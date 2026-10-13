use super::*;
use crate::core::company::aipro_paths::{path_key, validate_ancestors};
use std::path::PathBuf;

fn needs_recovery() -> CompanyError {
    CompanyError::new(
        "RECOVERY_REQUIRED",
        "Dữ liệu cập nhật đã được giữ. Mở lại app để phục hồi; không ghi đè đích này.",
    )
}
fn changed() -> CompanyError {
    CompanyError::new(
        "CONTENT_CHANGED",
        "Nội dung đã thay đổi sau preview. Giữ bản hiện tại và xem lại trước khi cập nhật.",
    )
}
fn matches(path: &Path, hash: &str) -> bool {
    path.is_dir()
        && validate_ancestors(path).is_ok()
        && content_hash(path, false).ok().as_deref() == Some(hash)
}

impl CompanyService {
    fn snapshot_backup(&self, old: &Installation, expected: &str) -> CompanyResult<Backup> {
        let root = self.config.data_root.join("backups").join(&old.id);
        uuid::Uuid::parse_str(&old.id).map_err(|_| operation_error())?;
        validate_ancestors(&root)?;
        fs::create_dir_all(&root).map_err(|_| {
            CompanyError::new(
                "BACKUP_FAILED",
                "Không tạo được thư mục backup. Giữ bản đang dùng.",
            )
        })?;
        let id = uuid::Uuid::new_v4().to_string();
        let temporary = tempfile::tempdir_in(&root).map_err(|_| write_error())?;
        copy_tree(Path::new(&old.target), &temporary.path().join("content"))?;
        if !matches(&temporary.path().join("content"), expected)
            || !matches(Path::new(&old.target), expected)
        {
            return Err(changed());
        }
        let destination = root.join(&id);
        let backup = Backup {
            id,
            installation_id: old.id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            reason: "update".into(),
            path: destination.join("content").display().to_string(),
            full_hash: expected.into(),
            installation: old.clone(),
        };
        let metadata = temporary.path().join("metadata.json");
        fs::write(
            &metadata,
            serde_json::to_vec(&backup).map_err(|_| write_error())?,
        )
        .map_err(|_| write_error())?;
        File::options()
            .write(true)
            .open(&metadata)
            .map_err(|_| write_error())?
            .sync_all()
            .map_err(|_| write_error())?;
        fs::rename(temporary.path(), &destination).map_err(|_| write_error())?;
        self.repository.save_backup(&backup)?;
        Ok(backup)
    }
    pub(super) fn replacement_paths(
        &self,
        journal: &ReplaceJournal,
    ) -> CompanyResult<(PathBuf, PathBuf, PathBuf)> {
        if journal.old.id != journal.new.id
            || journal.old.project_id != journal.new.project_id
            || journal.old.slug != journal.new.slug
        {
            return Err(needs_recovery());
        }
        let target =
            self.installation_target(journal.old.project_id.as_deref(), &journal.old.slug)?;
        if path_key(&target) != path_key(Path::new(&journal.old.target))
            || path_key(&target) != path_key(Path::new(&journal.new.target))
        {
            return Err(needs_recovery());
        }
        let stage = stage_path(&target, &journal.view.id)?;
        let rollback = target
            .parent()
            .ok_or_else(write_error)?
            .join(format!(".company-rollback-{}", journal.view.id));
        validate_ancestors(&rollback)?;
        Ok((target, stage, rollback))
    }
    pub(super) fn replace_installation(
        &self,
        id: &str,
        old: &Installation,
        package: &package_store::VerifiedPackage,
        old_hash: &str,
        owner: &str,
        cancelled: &AtomicBool,
        skill: &Skill,
        bypass: bool,
    ) -> CompanyResult<()> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        Self::check_cancel(cancelled)?;
        if self.owner()? != owner {
            return Err(CompanyError::new(
                "SESSION_EXPIRED",
                "Phiên đã thay đổi; chuẩn bị cập nhật lại.",
            ));
        }
        let current = self.guarded_installation(&old.id)?;
        if current.package_hash != old.package_hash || !matches(Path::new(&old.target), old_hash) {
            return Err(changed());
        }
        if !matches(&package.original_root, &package.full_hash) {
            return Err(CompanyError::new(
                "PACKAGE_INTEGRITY_FAILED",
                "Gói gốc đã đổi sau preview. Không cập nhật.",
            ));
        }
        let backup = self.snapshot_backup(old, old_hash)?;
        let mut new = old.clone();
        new.package_hash = package.archive_hash.clone();
        new.content_hash = package.content_hash.clone();
        new.name = skill.name.clone();
        new.description_snapshot = skill.description.clone().unwrap_or_default();
        new.installed_at = chrono::Utc::now().to_rfc3339();
        new.mcp_bypassed = bypass;
        new.content_state = ContentState::Original;
        new.store_state = StoreState::Current;
        new.checked_at = Some(chrono::Utc::now().to_rfc3339());
        new.store_error = None;
        let mut journal = ReplaceJournal {
            view: self.status(id)?,
            old: old.clone(),
            new,
            old_full_hash: old_hash.into(),
            new_full_hash: package.full_hash.clone(),
            backup_id: backup.id,
            phase: ReplacePhase::Prepared,
            stage_owned: false,
        };
        let (target, stage, rollback) = self.replacement_paths(&journal)?;
        if fs::symlink_metadata(&stage).is_ok() || fs::symlink_metadata(&rollback).is_ok() {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Vùng staging/rollback đã có nội dung. Không ghi đè.",
            ));
        }
        self.repository.record_replacement(&journal)?;
        let result = (|| {
            fs::create_dir(&stage).map_err(|_| write_error())?;
            journal.stage_owned = true;
            self.repository.record_replacement(&journal)?;
            copy_tree(&package.original_root, &stage)?;
            if !matches(&stage, &journal.new_full_hash) {
                return Err(write_error());
            }
            Self::check_cancel(cancelled)?;
            if self.owner()? != owner {
                return Err(CompanyError::new(
                    "SESSION_EXPIRED",
                    "Phiên đã thay đổi; chuẩn bị cập nhật lại.",
                ));
            }
            if !matches(&target, &journal.old_full_hash) {
                return Err(changed());
            }
            self.enter_commit(id)?;
            fs::rename(&target, &rollback).map_err(|_| write_error())?;
            journal.phase = ReplacePhase::OldMoved;
            self.repository.record_replacement(&journal)?;
            if !matches(&rollback, &journal.old_full_hash) {
                return Err(changed());
            }
            if fs::symlink_metadata(&target).is_ok() {
                return Err(needs_recovery());
            }
            if !matches(&stage, &journal.new_full_hash) {
                return Err(needs_recovery());
            }
            fs::rename(&stage, &target).map_err(|_| write_error())?;
            if !matches(&target, &journal.new_full_hash) {
                return Err(needs_recovery());
            }
            journal.phase = ReplacePhase::NewMoved;
            self.repository.record_replacement(&journal)?;
            journal.phase = ReplacePhase::Committed;
            self.repository.commit_replacement(&journal)?;
            Ok(())
        })();
        if let Err(error) = result {
            let durable = self
                .repository
                .pending_replacements()?
                .into_iter()
                .find(|j| j.view.id == id)
                .ok_or_else(needs_recovery)?;
            if !self.recover_replacement(&durable).unwrap_or(false) {
                return Err(needs_recovery());
            }
            return Err(error);
        }
        if !self.recover_replacement(&journal).unwrap_or(false) {
            return Err(needs_recovery());
        }
        Ok(())
    }
    pub(super) fn recover_replacement(&self, journal: &ReplaceJournal) -> CompanyResult<bool> {
        let (target, stage, rollback) = self.replacement_paths(journal)?;
        let backup = self
            .repository
            .backups(&journal.old.id)?
            .into_iter()
            .find(|b| b.id == journal.backup_id)
            .ok_or_else(needs_recovery)?;
        let expected_backup = self
            .config
            .data_root
            .join("backups")
            .join(&journal.old.id)
            .join(&journal.backup_id)
            .join("content");
        if path_key(Path::new(&backup.path)) != path_key(&expected_backup)
            || backup.full_hash != journal.old_full_hash
            || !matches(&expected_backup, &journal.old_full_hash)
        {
            return Ok(false);
        }
        let stage_exists = fs::symlink_metadata(&stage).is_ok();
        let rollback_exists = fs::symlink_metadata(&rollback).is_ok();
        let target_exists = fs::symlink_metadata(&target).is_ok();
        if stage_exists && (!journal.stage_owned || !matches(&stage, &journal.new_full_hash)) {
            return Ok(false);
        }
        if rollback_exists && !matches(&rollback, &journal.old_full_hash) {
            return Ok(false);
        }
        if journal.phase == ReplacePhase::Committed {
            let current = self.guarded_installation(&journal.new.id)?;
            if current.package_hash != journal.new.package_hash
                || current.content_hash != journal.new.content_hash
            {
                return Ok(false);
            }
            if stage_exists {
                fs::remove_dir_all(&stage).map_err(|_| write_error())?;
            }
            if rollback_exists {
                fs::remove_dir_all(&rollback).map_err(|_| write_error())?;
            }
            self.repository.finish_replacement(&journal.view.id)?;
            return Ok(true);
        }
        if rollback_exists {
            if target_exists {
                if !journal.stage_owned || stage_exists || !matches(&target, &journal.new_full_hash)
                {
                    return Ok(false);
                }
                fs::rename(&target, &stage).map_err(|_| write_error())?;
            }
            fs::rename(&rollback, &target).map_err(|_| write_error())?;
        } else if !matches(&target, &journal.old_full_hash) {
            return Ok(false);
        }
        if stage.exists() && journal.stage_owned && matches(&stage, &journal.new_full_hash) {
            fs::remove_dir_all(&stage).map_err(|_| write_error())?;
        }
        self.repository.finish_replacement(&journal.view.id)?;
        Ok(true)
    }
    pub(super) fn recover_replacements(&self) -> CompanyResult<()> {
        for journal in self.repository.pending_replacements()? {
            if self.recover_replacement(&journal).unwrap_or(false) {
                continue;
            }
            let mut view = journal.view.clone();
            view.state = OperationState::RecoveryRequired;
            view.error = Some(CompanyError::new(
                "RECOVERY_REQUIRED",
                format!(
                    "Đã giữ dữ liệu cập nhật cần kiểm tra: {} ; .company-rollback-{} ; backup {}",
                    view.target, view.id, journal.backup_id
                ),
            ));
            let skill = view.skill.clone().unwrap_or(Skill {
                id: journal.new.skill_id.clone(),
                slug: journal.new.slug.clone(),
                name: journal.new.name.clone(),
                description_excerpt: None,
                description: Some(journal.new.description_snapshot.clone()),
                updated_at: journal.new.installed_at.clone(),
                package: PackageMeta {
                    format: "zip".into(),
                    size_bytes: 0,
                    sha256: journal.new.package_hash.clone(),
                    etag: String::new(),
                },
            });
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
                        project_id: journal.old.project_id.clone(),
                        old: Some(journal.old),
                        old_full_hash: Some(journal.old_full_hash),
                        preview: None,
                        bypass: false,
                    },
                );
        }
        Ok(())
    }
}
