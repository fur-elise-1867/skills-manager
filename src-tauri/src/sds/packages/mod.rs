pub(crate) mod paths;
mod seven_zip;
mod zip;

use super::{
    contracts::{PackageFormat, Release},
    error::SdsError,
    targets::{is_reparse, validate_directory_path},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct PackageLimits {
    pub archive_bytes: u64,
    pub expanded_bytes: u64,
    pub entries: usize,
    pub depth: usize,
    pub decoder_memory_bytes: u64,
    pub header_bytes: u64,
}
impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            archive_bytes: 104_857_600,
            expanded_bytes: 524_288_000,
            entries: 10_000,
            depth: 32,
            decoder_memory_bytes: 268_435_456,
            header_bytes: 8_388_608,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub entries: Vec<ManifestEntry>,
}
#[derive(Debug, Clone)]
pub struct VerifiedArtifact {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub fingerprint: String,
}

pub fn prepare_package(
    archive: &Path,
    release: &Release,
    staging: &Path,
    limits: &PackageLimits,
) -> Result<VerifiedArtifact, SdsError> {
    prepare_package_cancellable(archive, release, staging, limits, &AtomicBool::new(false))
}
pub fn prepare_package_cancellable(
    archive: &Path,
    release: &Release,
    staging: &Path,
    limits: &PackageLimits,
    cancel: &AtomicBool,
) -> Result<VerifiedArtifact, SdsError> {
    check_cancel(cancel)?;
    if limits.archive_bytes == 0
        || limits.expanded_bytes == 0
        || limits.entries == 0
        || limits.depth == 0
    {
        return Err(limit_error());
    }
    paths::relative_path(&release.install_name, false, 1)?;
    let extension = match release.package.format {
        PackageFormat::Zip => ".zip",
        PackageFormat::SevenZip => ".7z",
    };
    if !release
        .package
        .file_name
        .to_ascii_lowercase()
        .ends_with(extension)
    {
        return Err(invalid());
    }
    let parent = staging.parent().ok_or_else(invalid)?;
    validate_directory_path(parent)?;
    validate_directory_path(staging)?;
    if staging.try_exists().map_err(|_| invalid())? {
        return Err(invalid());
    }
    let scratch = tempfile::Builder::new()
        .prefix(".sds-package-")
        .tempdir_in(parent)
        .map_err(|_| invalid())?;
    let mut stable = tempfile::tempfile_in(scratch.path()).map_err(|_| invalid())?;
    validate_directory_path(archive.parent().ok_or_else(invalid)?)?;
    let metadata = std::fs::symlink_metadata(archive).map_err(|_| invalid())?;
    if is_reparse(&metadata) || !metadata.is_file() {
        return Err(invalid());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000).share_mode(1);
    }
    let mut source = options.open(archive).map_err(|_| invalid())?;
    let metadata = source.metadata().map_err(|_| invalid())?;
    if is_reparse(&metadata) || !metadata.is_file() {
        return Err(invalid());
    }
    if metadata.len() > limits.archive_bytes || release.package.size_bytes > limits.archive_bytes {
        return Err(limit_error());
    }
    let mut digest = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        check_cancel(cancel)?;
        let count = source.read(&mut buffer).map_err(|_| invalid())?;
        if count == 0 {
            break;
        }
        size = size.checked_add(count as u64).ok_or_else(limit_error)?;
        if size > limits.archive_bytes {
            return Err(limit_error());
        }
        digest.update(&buffer[..count]);
        stable.write_all(&buffer[..count]).map_err(|_| invalid())?;
    }
    if size != release.package.size_bytes
        || hex::encode(digest.finalize()) != release.package.sha256.to_ascii_lowercase()
    {
        return Err(SdsError::new(
            "PACKAGE_INTEGRITY",
            "The package size or SHA-256 does not match its release. Download it again.",
            false,
        ));
    }
    stable.seek(SeekFrom::Start(0)).map_err(|_| invalid())?;
    let payload = scratch.path().join("payload");
    std::fs::create_dir(&payload).map_err(|_| invalid())?;
    let mut sink = Extraction::new(&payload, limits, cancel);
    match release.package.format {
        PackageFormat::Zip => zip::extract(stable, &mut sink)?,
        PackageFormat::SevenZip => seven_zip::extract(stable, &mut sink)?,
    }
    let root = sink.skill_root()?;
    let manifest = sink.manifest(&root);
    let fingerprint = hex::encode(Sha256::digest(
        serde_json::to_vec(&manifest).map_err(|_| invalid())?,
    ));
    check_cancel(cancel)?;
    // Publish only a completely verified tree; the scratch directory owns all failure cleanup.
    std::fs::rename(payload.join(&root), staging).map_err(|_| invalid())?;
    Ok(VerifiedArtifact {
        root: staging.into(),
        manifest,
        fingerprint,
    })
}

pub(super) struct Extraction<'a> {
    root: &'a Path,
    limits: &'a PackageLimits,
    cancel: &'a AtomicBool,
    names: paths::Names,
    entries: Vec<ManifestEntry>,
    count: usize,
    expanded: u64,
}
impl<'a> Extraction<'a> {
    fn new(root: &'a Path, limits: &'a PackageLimits, cancel: &'a AtomicBool) -> Self {
        Self {
            root,
            limits,
            cancel,
            names: paths::Names::default(),
            entries: vec![],
            count: 0,
            expanded: 0,
        }
    }
    pub(super) fn add(
        &mut self,
        name: &str,
        directory: bool,
        declared: u64,
        reader: &mut dyn Read,
    ) -> Result<(), SdsError> {
        check_cancel(self.cancel)?;
        self.count += 1;
        if self.count > self.limits.entries || declared > self.limits.expanded_bytes {
            return Err(limit_error());
        }
        let name = paths::relative_path(name, directory, self.limits.depth)?;
        self.names.insert(&name, directory)?;
        let path = self.root.join(&name);
        if directory {
            if declared != 0 {
                return Err(invalid());
            }
            std::fs::create_dir_all(&path).map_err(|_| invalid())?;
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| invalid())?;
        }
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| invalid())?;
        let mut digest = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 65_536];
        loop {
            check_cancel(self.cancel)?;
            let max_read = (self.limits.expanded_bytes - self.expanded)
                .saturating_add(1)
                .min(buffer.len() as u64) as usize;
            let count = reader
                .read(&mut buffer[..max_read])
                .map_err(|_| invalid())?;
            if count == 0 {
                break;
            }
            self.expanded += count as u64;
            size += count as u64;
            if self.expanded > self.limits.expanded_bytes {
                return Err(limit_error());
            }
            digest.update(&buffer[..count]);
            output.write_all(&buffer[..count]).map_err(|_| invalid())?;
        }
        if size != declared {
            return Err(invalid());
        }
        output.sync_all().map_err(|_| invalid())?;
        self.entries.push(ManifestEntry {
            path: name,
            size_bytes: size,
            sha256: Some(hex::encode(digest.finalize())),
        });
        Ok(())
    }
    fn skill_root(&self) -> Result<String, SdsError> {
        let roots: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.path.rsplit('/').next() == Some("SKILL.md"))
            .collect();
        if roots.len() != 1 {
            return Err(SdsError::new("PACKAGE_ROOT", "Upload a package with exactly one SKILL.md at its root, or inside one enclosing folder.", false));
        }
        let name = &roots[0].path;
        let root = name.strip_suffix("SKILL.md").unwrap().trim_end_matches('/');
        if root.contains('/')
            || self.names.paths().any(|path| {
                !root.is_empty() && path != root && !path.starts_with(&format!("{root}/"))
            })
        {
            return Err(SdsError::new(
                "PACKAGE_ROOT",
                "The package must contain one skill root without unrelated files.",
                false,
            ));
        }
        Ok(root.into())
    }
    fn manifest(&self, root: &str) -> Manifest {
        let prefix = if root.is_empty() {
            String::new()
        } else {
            format!("{root}/")
        };
        let mut entries = self.entries.clone();
        entries.extend(
            self.names
                .directories()
                .filter(|name| *name != root)
                .map(|path| ManifestEntry {
                    path: path.into(),
                    size_bytes: 0,
                    sha256: None,
                }),
        );
        for entry in &mut entries {
            entry.path = entry
                .path
                .strip_prefix(&prefix)
                .unwrap_or(&entry.path)
                .into();
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Manifest { entries }
    }
}
pub(super) fn invalid() -> SdsError {
    SdsError::new("PACKAGE_INVALID", "The archive is invalid or contains unsupported entries. Upload a standard ZIP or 7z package without encryption or split volumes.", false)
}
pub(super) fn limit_error() -> SdsError {
    SdsError::new("PACKAGE_LIMIT", "The package exceeds an extraction limit. Use an archive up to 100 MiB, expanded content up to 500 MiB, at most 10,000 entries and 32 path levels.", false)
}
pub(super) fn check_cancel(cancel: &AtomicBool) -> Result<(), SdsError> {
    if cancel.load(Ordering::Acquire) {
        Err(SdsError::new(
            "CANCELLED",
            "Package preparation was cancelled",
            false,
        ))
    } else {
        Ok(())
    }
}
