use super::{
    contracts::{McpReport, OperationResult, OperationState, PreparedOperation, Target},
    deployment::{fingerprint_tree, DirectoryPins},
    error::SdsError,
    packages::{Manifest, VerifiedArtifact},
    repository::{cache_key, copy_manifest, Repository},
    targets::{is_reparse, validate_directory_path},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

pub use super::contracts::FileIdentity;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitIntent {
    pub operation_id: String,
    pub target_root: PathBuf,
    pub root_identity: FileIdentity,
    pub stage_identity: FileIdentity,
    pub fingerprint: String,
    pub release_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: String,
    pub release_key: String,
    pub target: Target,
    pub identity: FileIdentity,
    pub root_identity: FileIdentity,
    pub manifest: Manifest,
    pub fingerprint: String,
    pub mcp: McpReport,
    pub installed_at: String,
}

impl CommitIntent {
    pub fn stage_path(&self) -> PathBuf {
        staging_parent(&self.target_root).join(&self.operation_id)
    }
    pub fn stage_install(
        repository: &Repository,
        plan: &PreparedOperation,
        artifact: &VerifiedArtifact,
    ) -> Result<Self, SdsError> {
        super::repository::validate_operation(plan)?;
        if plan.kind != "install" || !plan.can_execute {
            return Err(review());
        }
        let root = plan.target.path.parent().ok_or_else(review)?;
        root.parent().ok_or_else(review)?;
        let stage_parent = staging_parent(root);
        let _reviewed = plan.reviewed_root.pin(root).map_err(|_| review())?;
        let _pins = DirectoryPins::create(root)?;
        let root_identity = identity(_pins.leaf())?;
        let _lock = target_lock(repository, &root_identity, &plan.release.install_name)?;
        if std::fs::symlink_metadata(&plan.target.path).is_ok() {
            return Err(review());
        }
        if tracked_destination(repository, plan)? {
            return Err(review());
        }
        let cached = repository
            .cached_release(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?
            .ok_or_else(review)?;
        if serde_json::to_value(&cached.release).map_err(|_| review())?
            != serde_json::to_value(&plan.release).map_err(|_| review())?
            || cached.artifact.fingerprint != artifact.fingerprint
            || cached.artifact.manifest != artifact.manifest
        {
            return Err(review());
        }
        let _stage_parent = DirectoryPins::create(&stage_parent)?;
        let stage = stage_parent.join(&plan.id);
        std::fs::create_dir(&stage).map_err(|_| review())?;
        // Stages are deliberately retained on any error so later recovery cannot discard unknown files.
        copy_manifest(&artifact.root, &stage, &artifact.manifest)?;
        if fingerprint_tree(&stage)?.1 != artifact.fingerprint {
            return Err(review());
        }
        let staged = DirectoryPins::acquire(&stage)?;
        let stage_identity = identity(staged.leaf())?;
        if stage_identity.volume != root_identity.volume {
            return Err(review());
        }
        let intent = Self {
            operation_id: plan.id.clone(),
            target_root: root.into(),
            root_identity,
            stage_identity,
            fingerprint: artifact.fingerprint.clone(),
            release_key: cached.key,
        };
        repository.save_intent(plan, &intent)?;
        Ok(intent)
    }
    pub fn validate(&self, plan: &PreparedOperation) -> Result<(), SdsError> {
        if self.operation_id != plan.id
            || uuid::Uuid::parse_str(&self.operation_id).is_err()
            || plan.kind != "install"
            || plan.target.path.parent() != Some(self.target_root.as_path())
            || self.target_root.parent().is_none()
            || plan.target.path.file_name().and_then(|name| name.to_str())
                != Some(&plan.release.install_name)
            || self.release_key
                != cache_key(
                    &plan.context.profile_id,
                    &plan.release.skill_id,
                    &plan.release.id,
                )?
            || self.fingerprint.len() != 64
            || !self.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(review());
        }
        validate_directory_path(&self.target_root)?;
        validate_directory_path(&plan.target.path)?;
        validate_directory_path(&self.stage_path())?;
        Ok(())
    }
}

fn staging_parent(root: &Path) -> PathBuf {
    let mut parent = root.to_path_buf();
    parent.pop();
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let reserved = ".sds-skills-manager-staging";
    parent.join(if caseless::default_case_fold_str(name) == reserved {
        ".sds-skills-manager-operation-staging"
    } else {
        reserved
    })
}

pub fn commit_install(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
) -> Result<(), SdsError> {
    commit_install_inner(repository, plan, intent, |_| Ok(()))
}
#[derive(Clone, Copy)]
pub(crate) enum CommitBoundary {
    BeforeRename,
    AfterRename,
    AfterDatabase,
}
#[cfg(test)]
pub(crate) fn commit_install_test_hook(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
    hook: impl FnMut(CommitBoundary) -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    commit_install_inner(repository, plan, intent, hook)
}
fn commit_install_inner(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
    mut hook: impl FnMut(CommitBoundary) -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    let ready = PreparedInstallCommit::prepare(repository, plan, intent)?;
    hook(CommitBoundary::BeforeRename)?;
    ready.publish_inner(repository, &mut hook)
}

pub(crate) struct PreparedInstallCommit {
    plan: PreparedOperation,
    intent: CommitIntent,
    cached: super::repository::CachedRelease,
    rename: RenameGuard,
    _root_pins: DirectoryPins,
    _stage_pins: DirectoryPins,
    _lock: File,
}
impl PreparedInstallCommit {
    /// Content hashing happens before entering the session publication guard.
    pub(crate) fn prepare(
        repository: &Repository,
        plan: &PreparedOperation,
        intent: &CommitIntent,
    ) -> Result<Self, SdsError> {
        intent.validate(plan)?;
        let pins = DirectoryPins::acquire(&intent.target_root)?;
        if identity(pins.leaf())? != intent.root_identity {
            return Err(review());
        }
        let lock = target_lock(
            repository,
            &intent.root_identity,
            &plan.release.install_name,
        )?;
        if tracked_destination(repository, plan)? {
            return Err(review());
        }
        let stage_pins = DirectoryPins::acquire(intent.stage_path().parent().ok_or_else(review)?)?;
        match std::fs::symlink_metadata(&plan.target.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            _ => return Err(review()),
        }
        let cached = repository
            .cached_release(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?
            .ok_or_else(review)?;
        if cached.key != intent.release_key
            || cached.artifact.fingerprint != intent.fingerprint
            || serde_json::to_value(&cached.release).map_err(|_| review())?
                != serde_json::to_value(&plan.release).map_err(|_| review())?
        {
            return Err(review());
        }
        if fingerprint_tree(&intent.stage_path())?.1 != intent.fingerprint {
            return Err(review());
        }
        let rename = RenameGuard::open(&intent.stage_path())?;
        if identity(&rename.file)? != intent.stage_identity {
            return Err(review());
        }
        Ok(Self {
            plan: plan.clone(),
            intent: intent.clone(),
            cached,
            rename,
            _root_pins: pins,
            _stage_pins: stage_pins,
            _lock: lock,
        })
    }
    /// Run on a blocking worker while holding the frozen session context. Windows
    /// cannot rename a directory while its children have open handles, so hash
    /// immediately before/after the rename instead of retaining child handles.
    pub(crate) fn publish(self, repository: &Repository) -> Result<(), SdsError> {
        self.publish_inner(repository, &mut |_| Ok(()))
    }
    fn publish_inner(
        self,
        repository: &Repository,
        hook: &mut impl FnMut(CommitBoundary) -> Result<(), SdsError>,
    ) -> Result<(), SdsError> {
        drop(self.rename);
        if fingerprint_tree(&self.intent.stage_path())?.1 != self.intent.fingerprint {
            return Err(review());
        }
        let rename = RenameGuard::open(&self.intent.stage_path())?;
        if identity(&rename.file)? != self.intent.stage_identity {
            return Err(review());
        }
        if tracked_destination(repository, &self.plan)? {
            return Err(review());
        }
        rename.rename_to(&self.plan.target.path)?;
        hook(CommitBoundary::AfterRename)?;
        drop(rename);
        let _target = DirectoryPins::acquire(&self.plan.target.path)?;
        if fingerprint_tree(&self.plan.target.path)?.1 != self.intent.fingerprint {
            return Err(recovery_error());
        }
        repository.finish_deployment_verified(&self.plan, &self.intent, &self.cached)?;
        hook(CommitBoundary::AfterDatabase)?;
        Ok(())
    }
}

pub fn recover_pending(
    repository: &Repository,
    approved_roots: &[PathBuf],
) -> Result<Vec<OperationResult>, SdsError> {
    let mut results = Vec::new();
    for (plan, intent) in repository.pending_intents()? {
        let outcome = (|| {
            intent.validate(&plan)?;
            let pins = DirectoryPins::acquire(&intent.target_root)?;
            if identity(pins.leaf())? != intent.root_identity {
                return Err(recovery_error());
            }
            // Compare live directory identities, not Unicode-folded spellings. Windows
            // can hold distinct directories whose names share a Unicode case-fold key.
            let _approved = approved_roots
                .iter()
                .find_map(|root| {
                    let pins = DirectoryPins::acquire(root).ok()?;
                    (identity(pins.leaf()).ok()? == intent.root_identity).then_some(pins)
                })
                .ok_or_else(recovery_error)?;
            let _lock = target_lock(
                repository,
                &intent.root_identity,
                &plan.release.install_name,
            )?;
            let target_exists = std::fs::symlink_metadata(&plan.target.path).is_ok();
            let stage_exists = std::fs::symlink_metadata(intent.stage_path()).is_ok();
            if target_exists && !stage_exists {
                let target = DirectoryPins::acquire(&plan.target.path)?;
                if identity(target.leaf())? != intent.stage_identity
                    || fingerprint_tree(&plan.target.path)?.1 != intent.fingerprint
                {
                    return Err(recovery_error());
                }
                repository.finish_deployment(&plan, &intent)?;
                return repository.operation_status(&plan.id);
            }
            // No rename happened. Preserve the owned stage as recovery evidence; it is not installed.
            if !target_exists && stage_exists {
                let stage = DirectoryPins::acquire(&intent.stage_path())?;
                if identity(stage.leaf())? != intent.stage_identity
                    || fingerprint_tree(&intent.stage_path())?.1 != intent.fingerprint
                {
                    return Err(recovery_error());
                }
                let failed=OperationResult{operation_id:plan.id.clone(),state:OperationState::Failed,stage:"recovered-before-commit".into(),error:Some(SdsError::new("INSTALL_INTERRUPTED","Installation stopped before committing. Its staging evidence was kept; prepare and review the installation again.",true))};
                repository.fail_intent(&failed)?;
                return Ok(failed);
            }
            Err(recovery_error())
        })();
        let result = match outcome {
            Ok(result) => result,
            Err(error) => {
                let result = OperationResult {
                    operation_id: plan.id.clone(),
                    state: OperationState::RecoveryRequired,
                    stage: "recovery-required".into(),
                    error: Some(error),
                };
                repository.record_result(&result)?;
                result
            }
        };
        results.push(result);
    }
    Ok(results)
}
pub(crate) fn target_key(root: &FileIdentity, name: &str) -> Result<String, SdsError> {
    super::deployment::validate_relative(name)?;
    if name.contains('/') {
        return Err(review());
    }
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(&(root, caseless::default_case_fold_str(name))).map_err(|_| review())?,
    )))
}
pub(crate) fn managed_install(
    repository: &Repository,
    plan: &PreparedOperation,
) -> Result<Option<Deployment>, SdsError> {
    let root = plan.target.path.parent().ok_or_else(review)?;
    let pins = DirectoryPins::acquire(root)?;
    let root_id = identity(pins.leaf())?;
    let _lock = target_lock(repository, &root_id, &plan.release.install_name)?;
    inspect_managed(repository, plan, &root_id)
}
pub(crate) fn tracked_destination(
    repository: &Repository,
    plan: &PreparedOperation,
) -> Result<bool, SdsError> {
    let root = plan.target.path.parent().ok_or_else(review)?;
    let root_id = match std::fs::symlink_metadata(root) {
        Ok(_) => Some(identity(DirectoryPins::acquire(root)?.leaf())?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(review()),
    };
    for deployment in repository.deployments()? {
        if deployment.target.path == plan.target.path {
            return Ok(true);
        }
        if let Some(root) = &root_id {
            let name = deployment
                .target
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(review)?;
            if target_key(&deployment.root_identity, name)?
                == target_key(root, &plan.release.install_name)?
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn inspect_managed(
    repository: &Repository,
    plan: &PreparedOperation,
    root: &FileIdentity,
) -> Result<Option<Deployment>, SdsError> {
    let key = target_key(root, &plan.release.install_name)?;
    let release_key = cache_key(
        &plan.context.profile_id,
        &plan.release.skill_id,
        &plan.release.id,
    )?;
    for deployment in repository.deployments()? {
        let name = deployment
            .target
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(review)?;
        if target_key(&deployment.root_identity, name)? != key {
            continue;
        }
        if deployment.release_key != release_key {
            return Ok(None);
        }
        let target = DirectoryPins::acquire(&plan.target.path)?;
        if identity(target.leaf())? != deployment.identity
            || fingerprint_tree(&plan.target.path)?.1 != deployment.fingerprint
        {
            return Ok(None);
        }
        return Ok(Some(deployment));
    }
    Ok(None)
}
pub(crate) fn confirm_existing(
    repository: &Repository,
    plan: &PreparedOperation,
) -> Result<(), SdsError> {
    let root = plan.target.path.parent().ok_or_else(review)?;
    let pins = plan.reviewed_root.pin(root).map_err(|_| review())?;
    let root_id = identity(pins.leaf())?;
    let _lock = target_lock(repository, &root_id, &plan.release.install_name)?;
    let deployment = inspect_managed(repository, plan, &root_id)?.ok_or_else(review)?;
    if plan.expected_fingerprint.as_deref() != Some(&deployment.fingerprint) {
        return Err(review());
    }
    repository.complete_existing(plan, &deployment)
}
fn target_lock(repository: &Repository, root: &FileIdentity, name: &str) -> Result<File, SdsError> {
    let directory = repository.root().join("locks");
    let _pins = DirectoryPins::create(&directory)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000).share_mode(3);
    }
    let file = options
        .open(directory.join(format!("{}.lock", target_key(root, name)?)))
        .map_err(|_| review())?;
    let meta = file.metadata().map_err(|_| review())?;
    if is_reparse(&meta) || !meta.is_file() {
        return Err(review());
    }
    fs2::FileExt::try_lock_exclusive(&file).map_err(|_| {
        SdsError::new(
            "OPERATION_BUSY",
            "Another process is changing this destination. Try again.",
            true,
        )
    })?;
    Ok(file)
}
pub(crate) fn identity(file: &File) -> Result<FileIdentity, SdsError> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
        };
        let mut info = FILE_ID_INFO::default();
        // SAFETY: the SDK writes exactly FILE_ID_INFO into this valid buffer for a live handle.
        let ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        };
        if ok == 0 {
            return Err(review());
        }
        Ok(FileIdentity {
            volume: info.VolumeSerialNumber,
            file_id: hex::encode(info.FileId.Identifier),
        })
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata().map_err(|_| review())?;
        Ok(FileIdentity {
            volume: meta.dev(),
            file_id: format!("{:032x}", meta.ino()),
        })
    }
}
struct RenameGuard {
    file: File,
}
impl RenameGuard {
    fn open(path: &Path) -> Result<Self, SdsError> {
        validate_directory_path(path)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // DELETE + FILE_READ_ATTRIBUTES; deny other delete handles while renaming by this handle.
            options
                .access_mode(0x00010000 | 0x80)
                .share_mode(3)
                .custom_flags(0x02000000 | 0x00200000 | 0x80000000);
        }
        let file = options.open(path).map_err(|_| review())?;
        let meta = file.metadata().map_err(|_| review())?;
        if !meta.is_dir() || is_reparse(&meta) {
            return Err(review());
        }
        Ok(Self { file })
    }
    fn rename_to(&self, target: &Path) -> Result<(), SdsError> {
        #[cfg(windows)]
        {
            use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
            use windows_sys::Win32::Storage::FileSystem::{
                FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO,
            };
            let name: Vec<u16> = target.as_os_str().encode_wide().collect();
            let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
            let length = offset + name.len() * 2;
            // u64 storage provides FILE_RENAME_INFO alignment; zero means no replacement and no root handle.
            let mut storage = vec![0u64; (length + 7) / 8];
            let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
            // SAFETY: aligned buffer contains the SDK fixed header followed by the exact UTF-16 bytes.
            unsafe {
                (*info).FileNameLength = (name.len() * 2) as u32;
                std::ptr::copy_nonoverlapping(
                    name.as_ptr(),
                    (info.cast::<u8>().add(offset)).cast::<u16>(),
                    name.len(),
                );
                if SetFileInformationByHandle(
                    self.file.as_raw_handle(),
                    FileRenameInfo,
                    info.cast(),
                    length as u32,
                ) == 0
                {
                    return Err(review());
                }
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = target;
            Err(SdsError::new(
                "UNSUPPORTED_PLATFORM",
                "SDS deployment currently requires Windows",
                false,
            ))
        }
    }
}
pub(crate) fn review() -> SdsError {
    SdsError::new(
        "REVIEW_REQUIRED",
        "The destination or operation changed. Prepare and review the action again.",
        false,
    )
}
fn recovery_error() -> SdsError {
    SdsError::new("RECOVERY_REQUIRED","The interrupted operation cannot be reconciled automatically. Its files were preserved for review.",false)
}
