use super::{
    error::SdsError,
    packages::{Manifest, ManifestEntry, PackageLimits},
    targets::{is_reparse, validate_directory_path},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::Read,
    path::Path,
};
use unicode_normalization::UnicodeNormalization;

impl super::contracts::ReviewedRoot {
    pub fn capture(root: &Path) -> Result<Self, SdsError> {
        validate_directory_path(root)?;
        let root_existed = match std::fs::symlink_metadata(root) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(conflict()),
        };
        let mut anchor = if root_existed {
            root
        } else {
            root.parent().ok_or_else(conflict)?
        };
        while matches!(std::fs::symlink_metadata(anchor),Err(error) if error.kind()==std::io::ErrorKind::NotFound)
        {
            anchor = anchor.parent().ok_or_else(conflict)?;
        }
        let pins = DirectoryPins::acquire(anchor)?;
        Ok(Self {
            anchor: anchor.into(),
            identity: super::journal::identity(pins.leaf())?,
            root_existed,
        })
    }
    pub(crate) fn validate_shape(&self, root: &Path) -> Result<(), SdsError> {
        super::targets::validate_directory_shape(&self.anchor)?;
        if (self.root_existed && self.anchor != root)
            || (!self.root_existed
                && !root
                    .ancestors()
                    .skip(1)
                    .any(|ancestor| ancestor == self.anchor))
            || self.identity.file_id.len() != 32
            || !self.identity.file_id.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(conflict());
        }
        Ok(())
    }
    pub(crate) fn pin(&self, root: &Path) -> Result<DirectoryPins, SdsError> {
        self.validate_shape(root)?;
        validate_directory_path(root)?;
        let pins = DirectoryPins::acquire(&self.anchor)?;
        if super::journal::identity(pins.leaf())? != self.identity {
            return Err(conflict());
        }
        let exists = match std::fs::symlink_metadata(root) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(conflict()),
        };
        if exists != self.root_existed {
            return Err(conflict());
        }
        Ok(pins)
    }
}

/// Existing directory handles stay open without delete sharing on Windows. This pins
/// the path's ancestors while the caller reads, stages, or commits below them.
pub(crate) struct DirectoryPins {
    _handles: Vec<File>,
}
impl DirectoryPins {
    pub(crate) fn acquire(path: &Path) -> Result<Self, SdsError> {
        Self::open(path, false)
    }
    pub(crate) fn create(path: &Path) -> Result<Self, SdsError> {
        Self::open(path, true)
    }
    pub(crate) fn leaf(&self) -> &File {
        self._handles
            .last()
            .expect("An absolute path has a root handle")
    }
    fn open(path: &Path, create: bool) -> Result<Self, SdsError> {
        validate_directory_path(path)?;
        let mut ancestors: Vec<_> = path.ancestors().collect();
        ancestors.reverse();
        let mut handles = Vec::new();
        for ancestor in ancestors {
            if create
                && matches!(std::fs::symlink_metadata(ancestor),Err(error) if error.kind()==std::io::ErrorKind::NotFound)
            {
                match std::fs::create_dir(ancestor) {
                    Ok(()) => (),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(_) => return Err(conflict()),
                }
            }
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.custom_flags(0x02000000 | 0x00200000).share_mode(3);
            }
            let file = options.open(ancestor).map_err(|_| conflict())?;
            let meta = file.metadata().map_err(|_| conflict())?;
            if is_reparse(&meta) || !meta.is_dir() {
                return Err(conflict());
            }
            handles.push(file);
        }
        Ok(Self { _handles: handles })
    }
}

pub fn fingerprint_tree(root: &Path) -> Result<(Manifest, String), SdsError> {
    let _pins = DirectoryPins::acquire(root)?;
    let mut manifest = Manifest {
        entries: Vec::new(),
    };
    let mut expanded = 0;
    let mut names = BTreeSet::new();
    walk(root, root, 0, &mut manifest, &mut expanded, &mut names)?;
    manifest.entries.sort_by(|a, b| a.path.cmp(&b.path));
    let fingerprint = manifest_fingerprint(&manifest)?;
    Ok((manifest, fingerprint))
}
pub(crate) fn manifest_fingerprint(manifest: &Manifest) -> Result<String, SdsError> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(manifest).map_err(|_| conflict())?,
    )))
}
fn walk(
    root: &Path,
    directory: &Path,
    depth: usize,
    manifest: &mut Manifest,
    expanded: &mut u64,
    names: &mut BTreeSet<String>,
) -> Result<(), SdsError> {
    let limits = PackageLimits::default();
    if depth > limits.depth {
        return Err(conflict());
    }
    let _pins = DirectoryPins::acquire(directory)?;
    for entry in std::fs::read_dir(directory).map_err(|_| conflict())? {
        let path = entry.map_err(|_| conflict())?.path();
        if manifest.entries.len() >= limits.entries * limits.depth {
            return Err(conflict());
        }
        let metadata = std::fs::symlink_metadata(&path).map_err(|_| conflict())?;
        if is_reparse(&metadata) {
            return Err(conflict());
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| conflict())?
            .to_str()
            .ok_or_else(conflict)?
            .replace('\\', "/");
        validate_relative(&relative)?;
        let key = caseless::default_case_fold_str(&relative.nfc().collect::<String>());
        if !names.insert(key) {
            return Err(conflict());
        }
        if metadata.is_dir() {
            manifest.entries.push(ManifestEntry {
                path: relative,
                size_bytes: 0,
                sha256: None,
            });
            walk(root, &path, depth + 1, manifest, expanded, names)?;
        } else if metadata.is_file() {
            let mut file = open_content_file(&path)?;
            let mut hash = Sha256::new();
            let mut size = 0u64;
            let mut buffer = [0u8; 65_536];
            loop {
                let n = file.read(&mut buffer).map_err(|_| conflict())?;
                if n == 0 {
                    break;
                }
                size = size.checked_add(n as u64).ok_or_else(conflict)?;
                *expanded = expanded.checked_add(n as u64).ok_or_else(conflict)?;
                if *expanded > limits.expanded_bytes {
                    return Err(conflict());
                }
                hash.update(&buffer[..n]);
            }
            manifest.entries.push(ManifestEntry {
                path: relative,
                size_bytes: size,
                sha256: Some(hex::encode(hash.finalize())),
            });
        } else {
            return Err(conflict());
        }
    }
    Ok(())
}
pub(crate) fn open_content_file(path: &Path) -> Result<File, SdsError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000).share_mode(1);
    }
    let file = options.open(path).map_err(|_| conflict())?;
    let metadata = file.metadata().map_err(|_| conflict())?;
    if is_reparse(&metadata) || !metadata.is_file() {
        return Err(conflict());
    }
    Ok(file)
}
pub(crate) fn validate_relative(relative: &str) -> Result<(), SdsError> {
    super::packages::paths::relative_path(relative, false, PackageLimits::default().depth)
        .map(|_| ())
        .map_err(|_| conflict())
}
fn conflict() -> SdsError {
    SdsError::new("TARGET_CONFLICT", "The skill contents cannot be verified safely. Keep the existing files and review this destination.", false)
}
