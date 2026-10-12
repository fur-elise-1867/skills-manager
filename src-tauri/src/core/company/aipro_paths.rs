use super::{
    package_store::{reject_links, validate_slug},
    types::*,
};
use std::path::{Path, PathBuf};

pub fn global_target(profile: &Path, slug: &str) -> CompanyResult<PathBuf> {
    validate_slug(slug)?;
    let target = profile.join(".aipro").join("skills").join(slug);
    for ancestor in target.ancestors() {
        if std::fs::symlink_metadata(ancestor).is_ok() {
            reject_links(ancestor)?;
        }
    }
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
