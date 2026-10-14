use super::*;
use crate::core::company::aipro_paths::{path_key, validate_ancestors};
use std::{collections::BTreeSet, path::PathBuf};

fn terminal(state: &OperationState) -> bool {
    matches!(
        state,
        OperationState::Succeeded | OperationState::Cancelled | OperationState::Failed
    )
}
fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'a'..=b'f'))
}
fn bytes(path: &Path) -> CompanyResult<u64> {
    validate_ancestors(path)?;
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err(write_error()),
        Ok(_) => {}
    }
    let mut size = 0u64;
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = entry.map_err(|_| write_error())?;
        package_store::reject_links(entry.path())?;
        let meta = entry.metadata().map_err(|_| write_error())?;
        if meta.is_file() {
            size = size.checked_add(meta.len()).ok_or_else(write_error)?;
        }
    }
    Ok(size)
}
impl CompanyService {
    // Called under worker: no package/backup writer or cleanup can run while
    // references are gathered and used. Completed operations retain audit only.
    fn storage_references(&self) -> CompanyResult<(BTreeSet<String>, BTreeSet<String>)> {
        let mut packages = BTreeSet::new();
        let mut backups = BTreeSet::new();
        for item in self.repository.installations()? {
            if item.state == InstallationState::Active {
                packages.insert(item.package_hash);
            }
        }
        for backup in self.repository.all_backups()? {
            packages.insert(backup.installation.package_hash);
        }
        for journal in self.repository.pending()? {
            packages.insert(journal.installation.package_hash);
        }
        for journal in self.repository.pending_replacements()? {
            packages.insert(journal.old.package_hash);
            packages.insert(journal.new.package_hash);
            backups.insert(journal.backup_id);
        }
        for journal in self.repository.pending_lifecycle()? {
            packages.insert(journal.old.package_hash);
            packages.insert(journal.new.package_hash);
            for backup in [journal.current_backup, journal.source_backup]
                .into_iter()
                .flatten()
            {
                backups.insert(backup.id);
            }
        }
        let ops = self.operations.lock().map_err(|_| operation_error())?;
        for op in ops.values().filter(|op| !terminal(&op.view.state)) {
            packages.insert(op.skill.package.sha256.clone());
            if let Some(old) = &op.old {
                packages.insert(old.package_hash.clone());
            }
            if let Some(preview) = &op.preview {
                packages.insert(preview.archive_hash.clone());
            }
        }
        for journal in self
            .lifecycle_jobs
            .lock()
            .map_err(|_| operation_error())?
            .values()
        {
            if ops
                .get(&journal.view.id)
                .is_some_and(|op| !terminal(&op.view.state))
            {
                for backup in [&journal.current_backup, &journal.source_backup]
                    .into_iter()
                    .flatten()
                {
                    backups.insert(backup.id.clone());
                }
            }
        }
        Ok((packages, backups))
    }
    fn package_path(&self, hash: &str) -> CompanyResult<PathBuf> {
        if !valid_hash(hash) {
            return Err(CompanyError::new(
                "INVALID_REQUEST",
                "Package hash không hợp lệ.",
            ));
        }
        let path = self.config.data_root.join("packages").join(hash);
        validate_ancestors(&path)?;
        Ok(path)
    }
    fn backup_root(&self, backup: &Backup) -> CompanyResult<PathBuf> {
        uuid::Uuid::parse_str(&backup.id).map_err(|_| operation_error())?;
        uuid::Uuid::parse_str(&backup.installation_id).map_err(|_| operation_error())?;
        let root = self
            .config
            .data_root
            .join("backups")
            .join(&backup.installation_id)
            .join(&backup.id);
        validate_ancestors(&root)?;
        if path_key(&root.join("content")) != path_key(Path::new(&backup.path)) {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Bản lưu không khớp vùng dữ liệu ứng dụng.",
            ));
        }
        Ok(root)
    }
    pub fn storage_status(&self) -> CompanyResult<StorageSummary> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        let (references, in_use) = self.storage_references()?;
        let package_root = self.config.data_root.join("packages");
        let package_bytes = bytes(&package_root)?;
        let mut hashes: BTreeSet<String> = self.repository.package_hashes()?.into_iter().collect();
        if package_root.is_dir() {
            for entry in fs::read_dir(&package_root).map_err(|_| write_error())? {
                let entry = entry.map_err(|_| write_error())?;
                if let Some(hash) = entry.file_name().to_str().filter(|hash| valid_hash(hash)) {
                    hashes.insert(hash.to_string());
                }
            }
        }
        let mut packages = Vec::new();
        let mut unreferenced_package_bytes = 0;
        for hash in hashes {
            let size = bytes(&self.package_path(&hash)?)?;
            let referenced = references.contains(&hash);
            if !referenced {
                unreferenced_package_bytes += size;
            }
            packages.push(StoragePackage {
                hash,
                bytes: size,
                referenced,
            });
        }
        let backup_bytes = bytes(&self.config.data_root.join("backups"))?;
        let mut backups = Vec::new();
        for backup in self.repository.all_backups()? {
            backups.push(StorageBackup {
                bytes: bytes(&self.backup_root(&backup)?)?,
                in_use: in_use.contains(&backup.id),
                backup,
            });
        }
        Ok(StorageSummary {
            package_bytes,
            backup_bytes,
            unreferenced_package_bytes,
            packages,
            backups,
        })
    }
    pub fn delete_backup(&self, id: &str) -> CompanyResult<()> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        if self.storage_references()?.1.contains(id) {
            return Err(CompanyError::new(
                "BACKUP_IN_USE",
                "Bản lưu đang dùng cho thao tác hoặc recovery. Không xóa.",
            ));
        }
        let backup = self
            .repository
            .all_backups()?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| CompanyError::new("BACKUP_NOT_FOUND", "Không tìm thấy bản lưu."))?;
        let root = self.backup_root(&backup)?;
        if root.exists() {
            cleanup::remove_verified_tree(&root, &content_hash(&root, false)?)?;
        }
        self.repository.delete_backup(id)
    }
    pub fn cleanup_packages(&self, hashes: &[String]) -> CompanyResult<Vec<String>> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        let references = self.storage_references()?.0;
        let hashes: BTreeSet<_> = hashes.iter().cloned().collect();
        for hash in &hashes {
            self.package_path(hash)?;
            if references.contains(hash) {
                return Err(CompanyError::new(
                    "PACKAGE_IN_USE",
                    "Gói đang được bản cài, backup hoặc thao tác tham chiếu. Không xóa.",
                ));
            }
        }
        let mut deleted = Vec::new();
        for hash in hashes {
            let root = self.package_path(&hash)?;
            if root.exists() {
                cleanup::remove_verified_tree(&root, &content_hash(&root, false)?)?;
            }
            self.repository.delete_package(&hash)?;
            deleted.push(hash);
        }
        Ok(deleted)
    }
}
