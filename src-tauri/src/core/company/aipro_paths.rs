use super::{
    package_store::{reject_links, validate_slug},
    types::*,
};
use std::path::{Path, PathBuf};

pub fn path_key(path: &Path) -> String {
    let key = path
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase();
    if let Some(unc) = key.strip_prefix("//?/unc/") {
        format!("//{unc}")
    } else {
        key.strip_prefix("//?/").unwrap_or(&key).to_string()
    }
}
pub fn overlaps(a: &Path, b: &Path) -> bool {
    let a = path_key(a);
    let b = path_key(b);
    a == b || a.starts_with(&(b.clone() + "/")) || b.starts_with(&(a + "/"))
}
pub fn validate_ancestors(path: &Path) -> CompanyResult<()> {
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(_) => reject_links(ancestor)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(CompanyError::new(
                    "PATH_ERROR",
                    "Không kiểm tra được đường dẫn. Kiểm tra quyền thư mục.",
                ))
            }
        }
    }
    Ok(())
}

pub fn global_target(profile: &Path, slug: &str) -> CompanyResult<PathBuf> {
    validate_slug(slug)?;
    let target = profile.join(".aipro").join("skills").join(slug);
    validate_ancestors(&target)?;
    Ok(target)
}
pub fn stage_path(target: &Path, id: &str) -> CompanyResult<PathBuf> {
    uuid::Uuid::parse_str(id)
        .map_err(|_| CompanyError::new("WRITE_FAILED", "Operation ID không hợp lệ."))?;
    Ok(target
        .parent()
        .ok_or_else(|| CompanyError::new("WRITE_FAILED", "Đích cài không hợp lệ."))?
        .join(format!(".company-stage-{id}")))
}
