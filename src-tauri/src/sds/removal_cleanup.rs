use super::{
    deployment::{fingerprint_tree, validate_relative, DirectoryPins},
    error::SdsError,
    journal::{identity, review, CommitIntent},
    packages::{Manifest, ManifestEntry},
    targets::{is_reparse, validate_directory_path},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::Path,
};

/// Delete only verified manifest entries through their own no-follow handles.
/// Unknown/changed files are kept; an interrupted cleanup can resume from a subset.
pub(crate) fn cleanup(
    intent: &CommitIntent,
    expected: &Manifest,
    mut after_entry: impl FnMut() -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    cleanup_tree(
        &intent.stage_path(),
        &intent.stage_identity,
        expected,
        &mut after_entry,
    )
}
pub(crate) fn cleanup_tree(
    root: &Path,
    expected_identity: &super::contracts::FileIdentity,
    expected: &Manifest,
    mut after_entry: impl FnMut() -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    validate_directory_path(root.parent().ok_or_else(review)?)?;
    match std::fs::symlink_metadata(&root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(review()),
        _ => (),
    }
    let _parent = DirectoryPins::acquire(root.parent().ok_or_else(review)?)?;
    let root_pins = DirectoryPins::acquire(&root)?;
    if identity(root_pins.leaf())? != *expected_identity {
        return Err(review());
    }
    // Deny additions or edits present before cleanup; never enumerate untrusted
    // names as deletion candidates. Traversal itself has the package bounds.
    let (actual, _) = fingerprint_tree(&root)?;
    if actual
        .entries
        .iter()
        .any(|entry| !expected.entries.contains(entry))
    {
        return Err(review());
    }
    for entry in expected
        .entries
        .iter()
        .filter(|entry| entry.sha256.is_some())
    {
        validate_relative(&entry.path)?;
        let path = root.join(&entry.path);
        if absent(&path)? {
            continue;
        }
        let _pins = DirectoryPins::acquire(path.parent().ok_or_else(review)?)?;
        let mut handle = DeleteHandle::open(&path, false)?;
        handle.verify(entry)?;
        handle.delete()?;
        drop(handle);
        after_entry()?;
    }
    let mut directories: Vec<_> = expected
        .entries
        .iter()
        .filter(|entry| entry.sha256.is_none())
        .collect();
    directories.sort_by_key(|entry| std::cmp::Reverse(entry.path.matches('/').count()));
    for entry in directories {
        validate_relative(&entry.path)?;
        let path = root.join(&entry.path);
        if absent(&path)? {
            continue;
        }
        let _pins = DirectoryPins::acquire(path.parent().ok_or_else(review)?)?;
        let handle = DeleteHandle::open(&path, true)?;
        if std::fs::read_dir(&path)
            .map_err(|_| review())?
            .next()
            .is_some()
        {
            return Err(review());
        }
        handle.delete()?;
        drop(handle);
        after_entry()?;
    }
    if std::fs::read_dir(&root)
        .map_err(|_| review())?
        .next()
        .is_some()
    {
        return Err(review());
    }
    drop(root_pins);
    let original = DeleteHandle::open(&root, true)?;
    if identity(&original.file)? != *expected_identity {
        return Err(review());
    }
    if std::fs::read_dir(&root)
        .map_err(|_| review())?
        .next()
        .is_some()
    {
        return Err(review());
    }
    original.delete()?;
    Ok(())
}
fn absent(path: &Path) -> Result<bool, SdsError> {
    validate_directory_path(path.parent().ok_or_else(review)?)?;
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(_) => Err(review()),
    }
}
struct DeleteHandle {
    file: File,
}
impl DeleteHandle {
    fn open(path: &Path, directory: bool) -> Result<Self, SdsError> {
        validate_directory_path(path.parent().ok_or_else(review)?)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options
                .access_mode(0x80000000 | 0x00010000)
                .share_mode(if directory { 3 } else { 1 })
                .custom_flags(0x00200000 | if directory { 0x02000000 } else { 0 });
        }
        let file = options.open(path).map_err(|_| review())?;
        let metadata = file.metadata().map_err(|_| review())?;
        if is_reparse(&metadata)
            || (directory && !metadata.is_dir())
            || (!directory && !metadata.is_file())
        {
            return Err(review());
        }
        Ok(Self { file })
    }
    fn verify(&mut self, entry: &ManifestEntry) -> Result<(), SdsError> {
        if self.file.metadata().map_err(|_| review())?.len() != entry.size_bytes {
            return Err(review());
        }
        let mut hash = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let n = self.file.read(&mut buffer).map_err(|_| review())?;
            if n == 0 {
                break;
            }
            size = size.checked_add(n as u64).ok_or_else(review)?;
            if size > entry.size_bytes {
                return Err(review());
            }
            hash.update(&buffer[..n]);
        }
        if size != entry.size_bytes || Some(hex::encode(hash.finalize())) != entry.sha256 {
            return Err(review());
        }
        Ok(())
    }
    fn delete(&self) -> Result<(), SdsError> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::{
                FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
            };
            let mut info = FILE_DISPOSITION_INFO { DeleteFile: true };
            // SAFETY: live DELETE-access handle and exact SDK structure size.
            if unsafe {
                SetFileInformationByHandle(
                    self.file.as_raw_handle(),
                    FileDispositionInfo,
                    (&mut info as *mut FILE_DISPOSITION_INFO).cast(),
                    std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
                )
            } == 0
            {
                return Err(review());
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(SdsError::new(
                "UNSUPPORTED_PLATFORM",
                "SDS removal requires Windows.",
                false,
            ))
        }
    }
}
