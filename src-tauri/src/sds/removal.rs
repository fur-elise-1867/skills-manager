use super::{
    contracts::{
        DeploymentStatus, OperationResult, OperationState, PreparedOperation, ReviewedRoot,
        SessionContext,
    },
    error::SdsError,
    journal::{review, Deployment},
    repository::{cache_key, Repository},
};
use super::{
    deployment::{fingerprint_tree, DirectoryPins},
    journal::{identity, target_lock, CommitIntent, RenameGuard},
};
use std::path::Path;

pub(crate) fn deployment(repository: &Repository, id: &str) -> Result<Deployment, SdsError> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(SdsError::new(
            "INVALID_REQUEST",
            "Choose a recorded deployment to remove.",
            false,
        ));
    }
    repository
        .deployments()?
        .into_iter()
        .find(|deployment| deployment.id == id)
        .ok_or_else(|| {
            SdsError::new(
                "DEPLOYMENT_NOT_FOUND",
                "This recorded deployment is no longer available. Refresh your Library.",
                false,
            )
        })
}
pub(crate) fn prepare(
    repository: &Repository,
    epoch: &str,
    id: &str,
) -> Result<PreparedOperation, SdsError> {
    let deployment = deployment(repository, id)?;
    let release = repository
        .list_library()?
        .into_iter()
        .find(|entry| entry.id == deployment.release_key)
        .ok_or_else(review)?;
    let observation = super::library::inspect_deployment(&deployment);
    let can_execute = matches!(
        observation.status,
        DeploymentStatus::Installed | DeploymentStatus::Missing
    );
    let reviewed_root = ReviewedRoot::capture(deployment.target.path.parent().ok_or_else(review)?)?;
    let plan = PreparedOperation {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "remove".into(),
        deployment_id: Some(deployment.id),
        release: release.release,
        target: deployment.target,
        reviewed_root,
        expected_fingerprint: (observation.status == DeploymentStatus::Installed)
            .then_some(deployment.fingerprint),
        mcp: deployment.mcp,
        requires_mcp_ack: false,
        can_execute,
        conflict: (!can_execute).then_some(observation.reason.unwrap_or_else(|| {
            "This destination cannot be removed safely. Keep its current files.".into()
        })),
        context: SessionContext {
            profile_id: release.profile_id,
            account_id: "local-library".into(),
            generation: 0,
        },
        runtime_epoch: epoch.into(),
    };
    repository.create_operation(
        &plan,
        &OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Prepared,
            stage: "review".into(),
            error: None,
        },
    )?;
    Ok(plan)
}
pub(crate) fn reviewed_deployment(
    repository: &Repository,
    plan: &PreparedOperation,
) -> Result<Deployment, SdsError> {
    if plan.kind != "remove" || !plan.can_execute || plan.requires_mcp_ack {
        return Err(review());
    }
    let deployment = deployment(
        repository,
        plan.deployment_id.as_deref().ok_or_else(review)?,
    )?;
    if deployment.target != plan.target
        || deployment.release_key
            != cache_key(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?
        || plan
            .expected_fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint != &deployment.fingerprint)
    {
        return Err(review());
    }
    let current = repository
        .list_library()?
        .into_iter()
        .find(|entry| entry.id == deployment.release_key)
        .ok_or_else(review)?;
    if serde_json::to_value(&current.release).map_err(|_| review())?
        != serde_json::to_value(&plan.release).map_err(|_| review())?
    {
        return Err(review());
    }
    Ok(deployment)
}

pub(crate) fn remove_missing(
    repository: &Repository,
    plan: &PreparedOperation,
) -> Result<(), SdsError> {
    let deployment = reviewed_deployment(repository, plan)?;
    if plan.expected_fingerprint.is_some() {
        return Err(review());
    }
    let _root = plan
        .reviewed_root
        .pin(plan.target.path.parent().ok_or_else(review)?)
        .map_err(|_| review())?;
    let _lock = target_lock(
        repository,
        &deployment.root_identity,
        &plan.release.install_name,
    )?;
    match std::fs::symlink_metadata(&plan.target.path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            repository.finish_remove(plan, None, &deployment)
        }
        _ => Err(review()),
    }
}
fn check_owned_target(plan: &PreparedOperation, intent: &CommitIntent) -> Result<(), SdsError> {
    let target = DirectoryPins::acquire(&plan.target.path).map_err(|_| review())?;
    if identity(target.leaf())? != intent.stage_identity
        || fingerprint_tree(&plan.target.path)?.1 != intent.fingerprint
    {
        return Err(review());
    }
    Ok(())
}
fn removal_intent(
    plan: &PreparedOperation,
    deployment: &Deployment,
) -> Result<CommitIntent, SdsError> {
    if plan.expected_fingerprint.as_deref() != Some(&deployment.fingerprint) {
        return Err(review());
    }
    let intent = CommitIntent {
        operation_id: plan.id.clone(),
        target_root: plan.target.path.parent().ok_or_else(review)?.into(),
        root_identity: deployment.root_identity.clone(),
        stage_identity: deployment.identity.clone(),
        fingerprint: deployment.fingerprint.clone(),
        release_key: deployment.release_key.clone(),
    };
    intent.validate(plan)?;
    Ok(intent)
}
pub(crate) fn stage_remove(
    repository: &Repository,
    plan: &PreparedOperation,
) -> Result<CommitIntent, SdsError> {
    let deployment = reviewed_deployment(repository, plan)?;
    let intent = removal_intent(plan, &deployment)?;
    let root = plan
        .reviewed_root
        .pin(&intent.target_root)
        .map_err(|_| review())?;
    if identity(root.leaf())? != intent.root_identity {
        return Err(review());
    }
    let _lock = target_lock(
        repository,
        &intent.root_identity,
        &plan.release.install_name,
    )?;
    check_owned_target(plan, &intent)?;
    repository.removal_manifest(plan, &intent)?;
    let parent = intent
        .stage_path()
        .parent()
        .ok_or_else(review)?
        .to_path_buf();
    let quarantine = DirectoryPins::create(&parent)?;
    if identity(quarantine.leaf())?.volume != intent.root_identity.volume {
        return Err(review());
    }
    ensure_absent(&intent.stage_path())?;
    repository.save_intent(plan, &intent)?;
    Ok(intent)
}
#[derive(Clone, Copy)]
pub(crate) enum RemoveBoundary {
    BeforeRename,
    AfterRename,
    AfterDatabase,
    AfterCleanupEntry,
    AfterCleanup,
}
pub(crate) fn commit_remove(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
) -> Result<(), SdsError> {
    commit_inner(repository, plan, intent, &mut |_| Ok(()))
}
#[cfg(test)]
pub(crate) fn commit_remove_test_hook(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
    mut hook: impl FnMut(RemoveBoundary) -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    commit_inner(repository, plan, intent, &mut hook)
}
fn commit_inner(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
    hook: &mut impl FnMut(RemoveBoundary) -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    intent.validate(plan)?;
    let deployment = reviewed_deployment(repository, plan)?;
    if serde_json::to_value(removal_intent(plan, &deployment)?).map_err(|_| review())?
        != serde_json::to_value(intent).map_err(|_| review())?
    {
        return Err(review());
    }
    let root = plan
        .reviewed_root
        .pin(&intent.target_root)
        .map_err(|_| review())?;
    if identity(root.leaf())? != intent.root_identity {
        return Err(review());
    }
    let _lock = target_lock(
        repository,
        &intent.root_identity,
        &plan.release.install_name,
    )?;
    let quarantine = intent.stage_path();
    let _parent = DirectoryPins::acquire(quarantine.parent().ok_or_else(review)?)?;
    ensure_absent(&quarantine)?;
    check_owned_target(plan, intent)?;
    repository.removal_manifest(plan, intent)?;
    let rename = RenameGuard::open(&plan.target.path)?;
    if identity(&rename.file)? != intent.stage_identity {
        return Err(review());
    }
    hook(RemoveBoundary::BeforeRename)?;
    rename.rename_to(&quarantine)?;
    hook(RemoveBoundary::AfterRename)?;
    drop(rename);
    if !quarantine_matches(intent)? {
        if restore_original(plan, intent).is_ok() {
            repository.fail_intent(&OperationResult {
                operation_id: plan.id.clone(),
                state: OperationState::Failed,
                stage: "local-changes-restored".into(),
                error: Some(review()),
            })?;
            return Ok(());
        }
        return Err(recovery_error());
    }
    repository.finish_remove(plan, Some(intent), &deployment)?;
    hook(RemoveBoundary::AfterDatabase)?;
    finish_cleanup(repository, plan, intent, hook)?;
    Ok(())
}
fn ensure_absent(path: &Path) -> Result<(), SdsError> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(review()),
    }
}
fn quarantine_matches(intent: &CommitIntent) -> Result<bool, SdsError> {
    let pins = DirectoryPins::acquire(&intent.stage_path())?;
    Ok(identity(pins.leaf())? == intent.stage_identity
        && fingerprint_tree(&intent.stage_path())?.1 == intent.fingerprint)
}
fn restore_original(plan: &PreparedOperation, intent: &CommitIntent) -> Result<(), SdsError> {
    ensure_absent(&plan.target.path)?;
    let rename = RenameGuard::open(&intent.stage_path())?;
    if identity(&rename.file)? != intent.stage_identity {
        return Err(recovery_error());
    }
    rename.rename_to(&plan.target.path)
}
pub(crate) fn recover(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
) -> Result<OperationResult, SdsError> {
    intent.validate(plan)?;
    let _lock = target_lock(
        repository,
        &intent.root_identity,
        &plan.release.install_name,
    )?;
    if repository.operation_status(&plan.id)?.state == OperationState::Completed {
        return finish_cleanup(repository, plan, intent, &mut |_| Ok(()));
    }
    let deployment = reviewed_deployment(repository, plan)?;
    if serde_json::to_value(removal_intent(plan, &deployment)?).map_err(|_| review())?
        != serde_json::to_value(intent).map_err(|_| review())?
    {
        return Err(recovery_error());
    }
    let root = DirectoryPins::acquire(&intent.target_root)?;
    if identity(root.leaf())? != intent.root_identity {
        return Err(recovery_error());
    }
    let target_exists = exists(&plan.target.path)?;
    let quarantine_exists = exists(&intent.stage_path())?;
    if target_exists && !quarantine_exists {
        check_owned_target(plan, intent)?;
        repository.fail_intent(&OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Failed,
            stage: "recovered-before-remove".into(),
            error: Some(SdsError::new(
                "REMOVE_INTERRUPTED",
                "Removal stopped before changing this destination. Review it again.",
                true,
            )),
        })?;
        return repository.operation_status(&plan.id);
    }
    if !target_exists && quarantine_exists {
        if !quarantine_matches(intent)? {
            restore_original(plan, intent)?;
            repository.fail_intent(&OperationResult {
                operation_id: plan.id.clone(),
                state: OperationState::Failed,
                stage: "local-changes-restored".into(),
                error: Some(review()),
            })?;
            return repository.operation_status(&plan.id);
        }
        repository.finish_remove(plan, Some(intent), &deployment)?;
        return finish_cleanup(repository, plan, intent, &mut |_| Ok(()));
    }
    Err(recovery_error())
}
fn exists(path: &Path) -> Result<bool, SdsError> {
    // Validate ancestors before probing a leaf so even status checks do not follow links.
    super::targets::validate_directory_path(path.parent().ok_or_else(review)?)?;
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(recovery_error()),
    }
}
fn finish_cleanup(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &CommitIntent,
    hook: &mut impl FnMut(RemoveBoundary) -> Result<(), SdsError>,
) -> Result<OperationResult, SdsError> {
    let cleaned = (|| {
        let manifest = repository.removal_manifest(plan, intent)?;
        super::removal_cleanup::cleanup(intent, &manifest, || {
            hook(RemoveBoundary::AfterCleanupEntry)
        })
    })();
    let warning = cleaned.err().map(|_| {
        let path: String = intent
            .stage_path()
            .to_string_lossy()
            .chars()
            .take(4096)
            .collect();
        let mut warning = SdsError::new(
            "CLEANUP_REVIEW_REQUIRED",
            &format!(
                "Removal completed. Some files were kept at {path}. Review them before cleanup."
            ),
            false,
        );
        warning.operation_id = Some(plan.id.clone());
        warning
    });
    hook(RemoveBoundary::AfterCleanup)?;
    repository.finish_remove_cleanup(plan, intent, warning)
}
fn recovery_error() -> SdsError {
    SdsError::new(
        "RECOVERY_REQUIRED",
        "The interrupted removal could not be reconciled safely. Its files were kept for review.",
        false,
    )
}
