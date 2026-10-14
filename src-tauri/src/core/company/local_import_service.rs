use super::*;
use crate::core::company::aipro_paths::{overlaps, path_key, validate_ancestors};
use std::path::PathBuf;

#[derive(Clone)]
pub(super) struct LocalSource {
    path: PathBuf,
    meta: PackageMeta,
}

impl CompanyService {
    pub fn prepare_local_import(
        &self,
        archive: &Path,
        slug: &str,
        project_id: Option<&str>,
    ) -> CompanyResult<OperationView> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        package_store::validate_slug(slug)?;
        validate_ancestors(archive)?;
        if !archive.is_absolute() || !archive.is_file() {
            return Err(CompanyError::new(
                "INVALID_PACKAGE",
                "Chọn file ZIP hoặc 7z có sẵn trên máy.",
            ));
        }
        let format = archive
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(format.as_str(), "zip" | "7z") {
            return Err(CompanyError::new(
                "INVALID_PACKAGE",
                "Chỉ hỗ trợ ZIP hoặc 7z.",
            ));
        }
        let target = self.installation_target(project_id, slug)?;
        if fs::symlink_metadata(&target).is_ok()
            || self.repository.installations()?.iter().any(|i| {
                i.state == InstallationState::Active && overlaps(Path::new(&i.target), &target)
            })
        {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Đích đã có bản cài hoặc nội dung được quản lý. Không ghi đè.",
            ));
        }
        if self
            .operations
            .lock()
            .map_err(|_| operation_error())?
            .values()
            .any(|op| {
                op.view.state == OperationState::RecoveryRequired
                    && path_key(Path::new(&op.view.target)) == path_key(&target)
            })
        {
            return Err(CompanyError::new(
                "RECOVERY_REQUIRED",
                "Đích có dữ liệu chưa phục hồi. Không nhập đè.",
            ));
        }
        let mut input = File::open(archive).map_err(|_| write_error())?;
        let mut buffer = [0u8; 65_536];
        let mut hash = Sha256::new();
        let mut size = 0u64;
        loop {
            let count = input.read(&mut buffer).map_err(|_| write_error())?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > MAX_ARCHIVE {
                return Err(CompanyError::new(
                    "INVALID_PACKAGE",
                    "File vượt giới hạn 100 MiB.",
                ));
            }
            hash.update(&buffer[..count]);
        }
        if size == 0 {
            return Err(CompanyError::new("INVALID_PACKAGE", "File gói rỗng."));
        }
        let meta = PackageMeta {
            format,
            size_bytes: size,
            sha256: hex::encode(hash.finalize()),
            etag: String::new(),
        };
        // OperationView uses the shared name/package shape. An empty operation
        // skill id never becomes a Store association on a local installation.
        let skill = Skill {
            id: String::new(),
            slug: slug.into(),
            name: slug.into(),
            description_excerpt: None,
            description: None,
            updated_at: String::new(),
            package: meta.clone(),
        };
        let view = OperationView { id: uuid::Uuid::new_v4().to_string(), state: OperationState::AwaitingDecision,
            target: target.display().to_string(), mcp: McpReport { state: McpState::Unknown, required: vec![], present: vec![], missing: vec![],
                reason: Some("Gói local chưa có dữ liệu yêu cầu MCP từ Skill Store. Không suy ra yêu cầu từ SKILL.md.".into()), checked_at: String::new() },
            mcp_path: String::new(), skill: Some(skill.clone()), progress_bytes: 0, total_bytes: size, error: None,
            kind: OperationKind::LocalImport, preview_ready: false, customized: false, changes: vec![] };
        self.local_sources
            .lock()
            .map_err(|_| operation_error())?
            .insert(
                view.id.clone(),
                LocalSource {
                    path: archive.into(),
                    meta,
                },
            );
        self.operations
            .lock()
            .map_err(|_| operation_error())?
            .insert(
                view.id.clone(),
                Operation {
                    view: view.clone(),
                    skill,
                    owner: String::new(),
                    cancelled: Arc::new(AtomicBool::new(false)),
                    project_id: project_id.map(str::to_string),
                    old: None,
                    old_full_hash: None,
                    preview: None,
                    bypass: false,
                },
            );
        Ok(view)
    }
    pub(super) fn copy_local_archive(
        &self,
        id: &str,
        output: &Path,
        cancelled: &AtomicBool,
    ) -> CompanyResult<()> {
        let source = self
            .local_sources
            .lock()
            .map_err(|_| operation_error())?
            .get(id)
            .cloned()
            .ok_or_else(operation_error)?;
        validate_ancestors(&source.path)?;
        let mut input = File::open(&source.path).map_err(|_| write_error())?;
        let mut destination = File::create(output).map_err(|_| write_error())?;
        let mut buffer = [0u8; 65_536];
        let mut size = 0u64;
        loop {
            Self::check_cancel(cancelled)?;
            let count = input.read(&mut buffer).map_err(|_| write_error())?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > MAX_ARCHIVE || size > source.meta.size_bytes {
                return Err(CompanyError::new(
                    "PACKAGE_INTEGRITY_FAILED",
                    "File local đã thay đổi sau khi chọn.",
                ));
            }
            destination
                .write_all(&buffer[..count])
                .map_err(|_| write_error())?;
            self.update(id, OperationState::Downloading, Some(size), None);
        }
        destination.sync_all().map_err(|_| write_error())?;
        if size != source.meta.size_bytes {
            return Err(CompanyError::new(
                "PACKAGE_INTEGRITY_FAILED",
                "File local đã thay đổi sau khi chọn.",
            ));
        }
        Ok(())
    }
}
