use super::*;
use crate::core::company::aipro_paths::{overlaps, path_key, validate_ancestors};
use std::path::PathBuf;

impl CompanyService {
    pub fn projects(&self) -> CompanyResult<Vec<Project>> {
        self.repository.projects()
    }

    pub fn add_project(&self, path: &Path) -> CompanyResult<Project> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        if !path.is_absolute() || !path.is_dir() {
            return Err(CompanyError::new(
                "INVALID_PROJECT",
                "Chọn thư mục dự án có sẵn trên máy.",
            ));
        }
        validate_ancestors(path)?;
        let path = fs::canonicalize(path).map_err(|_| write_error())?;
        if overlaps(&path, &self.config.data_root)
            || self.repository.installations()?.iter().any(|item| {
                let key = path_key(&path);
                let target = path_key(Path::new(&item.target));
                key == target || key.starts_with(&(target + "/"))
            })
        {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Dự án không được nằm trong bản cài hoặc vùng dữ liệu ứng dụng.",
            ));
        }
        if let Some(existing) = self
            .projects()?
            .into_iter()
            .find(|p| path_key(Path::new(&p.path)) == path_key(&path))
        {
            return Ok(existing);
        }
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.display().to_string(),
        };
        self.repository.save_project(&project)?;
        Ok(project)
    }
    pub fn remove_project(&self, id: &str) -> CompanyResult<()> {
        let _worker = self.worker.lock().map_err(|_| write_error())?;
        if self
            .repository
            .installations()?
            .iter()
            .any(|item| item.project_id.as_deref() == Some(id))
        {
            return Err(CompanyError::new(
                "PROJECT_IN_USE",
                "Dự án còn bản cài được quản lý; không bỏ dự án khỏi danh sách.",
            ));
        }
        if self
            .operations
            .lock()
            .map_err(|_| operation_error())?
            .values()
            .any(|op| {
                op.project_id.as_deref() == Some(id)
                    && !matches!(
                        op.view.state,
                        OperationState::Succeeded
                            | OperationState::Failed
                            | OperationState::Cancelled
                    )
            })
        {
            return Err(CompanyError::new(
                "PROJECT_IN_USE",
                "Dự án còn thao tác chưa hoàn tất.",
            ));
        }
        self.repository.remove_project(id)
    }
    pub(super) fn installation_target(
        &self,
        project_id: Option<&str>,
        slug: &str,
    ) -> CompanyResult<PathBuf> {
        let target = self.recorded_target(project_id, slug)?;
        if project_id.is_some() && !self.scope_root(project_id)?.is_dir() {
            return Err(CompanyError::new(
                "PROJECT_MISSING",
                "Thư mục dự án không còn tồn tại. Chọn lại thư mục trước khi cài.",
            ));
        }
        Ok(target)
    }
    fn scope_root(&self, project_id: Option<&str>) -> CompanyResult<PathBuf> {
        match project_id {
            None => Ok(self.config.profile_root.clone()),
            Some(id) => Ok(PathBuf::from(
                self.projects()?
                    .into_iter()
                    .find(|p| p.id == id)
                    .ok_or_else(|| CompanyError::new("PROJECT_NOT_FOUND", "Không tìm thấy dự án."))?
                    .path,
            )),
        }
    }
    fn recorded_target(&self, project_id: Option<&str>, slug: &str) -> CompanyResult<PathBuf> {
        let root = self.scope_root(project_id)?;
        let target = global_target(&root, slug)?;
        if overlaps(&target, &self.config.data_root) {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Đích cài chồng lấn dữ liệu ứng dụng.",
            ));
        }
        Ok(target)
    }
    pub(super) fn guarded_installation(&self, id: &str) -> CompanyResult<Installation> {
        let item = self
            .repository
            .installations()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| {
                CompanyError::new("INSTALLATION_NOT_FOUND", "Không tìm thấy bản cài.")
            })?;
        let expected = self.recorded_target(item.project_id.as_deref(), &item.slug)?;
        if path_key(&expected) != path_key(Path::new(&item.target)) {
            return Err(CompanyError::new(
                "TARGET_CONFLICT",
                "Đường dẫn bản cài không khớp ownership đã ghi nhận.",
            ));
        }
        Ok(item)
    }
    pub(super) fn refreshed(&self, mut item: Installation) -> Installation {
        item.content_state = if self.guarded_installation(&item.id).is_err() {
            ContentState::Unknown
        } else {
            match fs::symlink_metadata(&item.target) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => ContentState::Missing,
                Err(_) => ContentState::Unknown,
                Ok(meta) if !meta.is_dir() => ContentState::Unknown,
                Ok(_) => match content_hash(Path::new(&item.target), true) {
                    Ok(hash) if hash == item.content_hash => ContentState::Original,
                    Ok(_) => ContentState::Customized,
                    Err(_) => ContentState::Unknown,
                },
            }
        };
        item
    }
    pub fn refresh_installation(&self, id: &str) -> CompanyResult<Installation> {
        Ok(self.refreshed(self.guarded_installation(id)?))
    }
    pub fn installation_open_path(&self, id: &str, kind: &str) -> CompanyResult<PathBuf> {
        let item = self.guarded_installation(id)?;
        let target = PathBuf::from(item.target);
        let path = match kind {
            "skill_file" => target.join("SKILL.md"),
            "folder" => target,
            _ => {
                return Err(CompanyError::new(
                    "INVALID_REQUEST",
                    "Chọn mở SKILL.md hoặc thư mục.",
                ))
            }
        };
        validate_ancestors(&path)?;
        if !path.exists() {
            return Err(CompanyError::new(
                "CONTENT_MISSING",
                "Không tìm thấy file hoặc thư mục bản cài.",
            ));
        }
        Ok(path)
    }
}
