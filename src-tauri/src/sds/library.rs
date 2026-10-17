pub use super::contracts::{CacheStatus, DeploymentStatus};
use super::{
    contracts::{LibraryDeployment, LibraryItem},
    deployment::{fingerprint_tree, open_content_file, DirectoryPins},
    error::SdsError,
    journal::{identity, Deployment},
    repository::{LibraryEntry, Repository},
    targets::validate_directory_path,
};
use sha2::{Digest, Sha256};
use std::io::Read;

pub(crate) fn with_deployment_path<T>(
    repository: &Repository,
    id: &str,
    action: impl FnOnce(&std::path::Path) -> Result<T, SdsError>,
) -> Result<T, SdsError> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(location_error());
    }
    let deployment = repository
        .deployments()?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(location_error)?;
    validate_directory_path(&deployment.target.path)?;
    let root = DirectoryPins::acquire(deployment.target.path.parent().ok_or_else(location_error)?)?;
    let target = DirectoryPins::acquire(&deployment.target.path)?;
    if identity(root.leaf())? != deployment.root_identity
        || identity(target.leaf())? != deployment.identity
    {
        return Err(location_error());
    }
    action(&deployment.target.path)
}
fn location_error() -> SdsError {
    SdsError::new("LOCATION_UNAVAILABLE", "The recorded destination cannot be revealed safely. Refresh Library and check its location.", false)
}

pub(crate) fn load(repository: &Repository) -> Result<Vec<LibraryItem>, SdsError> {
    let entries = repository.list_library()?;
    let deployments = repository.deployments()?;
    entries
        .into_iter()
        .map(|entry| {
            let cache_status = cache_status(repository, &entry);
            let local = deployments
                .iter()
                .filter(|deployment| deployment.release_key == entry.id)
                .map(inspect_deployment)
                .collect();
            Ok(LibraryItem {
                id: entry.id,
                profile_id: entry.profile_id,
                release: entry.release,
                cached_at: entry.cached_at,
                cache_status,
                deployments: local,
            })
        })
        .collect()
}
fn cache_status(repository: &Repository, entry: &LibraryEntry) -> CacheStatus {
    let root = repository.root().join("releases").join(&entry.id);
    if validate_directory_path(&root).is_err() {
        return CacheStatus::Unverifiable;
    }
    match std::fs::symlink_metadata(&root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return CacheStatus::Missing,
        Err(_) => return CacheStatus::Unverifiable,
        _ => (),
    }
    match repository.cached_release(
        &entry.profile_id,
        &entry.release.skill_id,
        &entry.release.id,
    ) {
        Ok(Some(_)) => CacheStatus::Verified,
        Ok(None) => CacheStatus::Missing,
        Err(error) if error.code == "CACHE_CORRUPT" => CacheStatus::Corrupt,
        Err(_) => CacheStatus::Unverifiable,
    }
}
pub(crate) fn inspect_deployment(deployment: &Deployment) -> LibraryDeployment {
    let status = (|| {
        validate_directory_path(&deployment.target.path)
            .map_err(|_| DeploymentStatus::Unverifiable)?;
        match std::fs::symlink_metadata(&deployment.target.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(DeploymentStatus::Missing)
            }
            Err(_) => return Err(DeploymentStatus::Unverifiable),
            _ => (),
        }
        let root = DirectoryPins::acquire(
            deployment
                .target
                .path
                .parent()
                .ok_or(DeploymentStatus::Unverifiable)?,
        )
        .map_err(|_| DeploymentStatus::Unverifiable)?;
        let target = DirectoryPins::acquire(&deployment.target.path)
            .map_err(|_| DeploymentStatus::Unverifiable)?;
        if identity(root.leaf()).map_err(|_| DeploymentStatus::Unverifiable)?
            != deployment.root_identity
            || identity(target.leaf()).map_err(|_| DeploymentStatus::Unverifiable)?
                != deployment.identity
        {
            return Err(DeploymentStatus::Unverifiable);
        }
        let (_, fingerprint) = fingerprint_tree(&deployment.target.path)
            .map_err(|_| DeploymentStatus::Unverifiable)?;
        Ok(if fingerprint == deployment.fingerprint {
            DeploymentStatus::Installed
        } else {
            DeploymentStatus::Modified
        })
    })()
    .unwrap_or_else(|status| status);
    LibraryDeployment{expected_fingerprint: if matches!(status,DeploymentStatus::Installed|DeploymentStatus::Modified){fingerprint_tree(&deployment.target.path).ok().map(|(_,f)|f)}else{None},id:deployment.id.clone(),target:deployment.target.clone(),status,mcp:deployment.mcp.clone(),installed_at:deployment.installed_at.clone(),reason:match status{
        DeploymentStatus::Installed=>None,
        DeploymentStatus::Modified=>Some("Local contents have changed. Keep these files; force removal is unavailable in the pilot.".into()),
        DeploymentStatus::Missing=>Some("The installed folder is missing.".into()),
        DeploymentStatus::Unverifiable=>Some("This destination cannot be verified safely. Keep its current files and review its location.".into()),
    }}
}
pub(crate) fn preview(repository: &Repository, id: &str) -> Result<String, SdsError> {
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(SdsError::new(
            "INVALID_REQUEST",
            "Choose a Library release to preview.",
            false,
        ));
    }
    let entry = repository
        .list_library()?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| {
            SdsError::new(
                "LIBRARY_ENTRY_NOT_FOUND",
                "This Library release is no longer available.",
                false,
            )
        })?;
    let cached=repository.cached_release(&entry.profile_id,&entry.release.skill_id,&entry.release.id)?.ok_or_else(||SdsError::new("CACHE_CORRUPT","The release cache is unavailable. Keep existing installations and download a verified copy.",false))?;
    let skill = cached
        .artifact
        .manifest
        .entries
        .iter()
        .find(|entry| entry.path.eq_ignore_ascii_case("SKILL.md") && entry.sha256.is_some())
        .ok_or_else(preview_error)?;
    const LIMIT: u64 = 1_048_576;
    if skill.size_bytes > LIMIT {
        return Err(SdsError::new(
            "PREVIEW_LIMIT",
            "This skill document is too large to preview (maximum 1 MiB).",
            false,
        ));
    }
    let _pins = DirectoryPins::acquire(&cached.artifact.root)?;
    let file = open_content_file(&cached.artifact.root.join(&skill.path))?;
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| preview_error())?;
    if bytes.len() as u64 != skill.size_bytes
        || Some(hex::encode(Sha256::digest(&bytes))) != skill.sha256
    {
        return Err(preview_error());
    }
    String::from_utf8(bytes).map_err(|_| preview_error())
}
fn preview_error() -> SdsError {
    SdsError::new("PREVIEW_UNAVAILABLE","The skill document could not be verified as readable text. Keep the cached files and check the release.",false)
}
