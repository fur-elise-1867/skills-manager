use super::{
    contracts::{FileIdentity, OperationResult, OperationState, PreparedOperation, Release},
    deployment::{fingerprint_tree, DirectoryPins},
    error::SdsError,
    journal::{identity, review, target_lock, Deployment, RenameGuard},
    packages::{Manifest, VerifiedArtifact},
    repository::{cache_key, copy_manifest, Repository},
    snapshots::{SnapshotReason, SnapshotService},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplacementIntent {
    pub version: u32,
    pub operation_id: String,
    pub root_identity: FileIdentity,
    pub stage_identity: FileIdentity,
    pub new_fingerprint: String,
    pub old_identity: Option<FileIdentity>,
    pub old_fingerprint: Option<String>,
    pub old_manifest: Option<Manifest>,
    pub new_manifest: Manifest,
    pub deployment: Deployment,
    pub release: Release,
    pub snapshot_id: Option<String>,
}
impl ReplacementIntent {
    pub fn stage_path(&self) -> PathBuf {
        self.directory().join("new")
    }
    pub fn rollback_path(&self) -> PathBuf {
        self.directory().join("old")
    }
    fn directory(&self) -> PathBuf {
        self.deployment
            .target
            .path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".sds-skills-manager-replacements")
            .join(&self.operation_id)
    }
    fn validate(&self, plan: &PreparedOperation) -> Result<(), SdsError> {
        if self.version != 2
            || self.operation_id != plan.id
            || uuid::Uuid::parse_str(&self.operation_id).is_err()
            || !["update", "restore"].contains(&plan.kind.as_str())
            || self.deployment.target != plan.target
            || self.deployment.root_identity != self.root_identity
            || self.deployment.identity != self.stage_identity
            || self.old_fingerprint != plan.expected_fingerprint
            || self.deployment.release_key
                != cache_key(
                    &plan.context.profile_id,
                    &plan.release.skill_id,
                    &plan.release.id,
                )?
            || self.deployment.id != plan.deployment_id.clone().ok_or_else(review)?
            || super::deployment::manifest_fingerprint(&self.new_manifest)? != self.new_fingerprint
            || super::deployment::manifest_fingerprint(&self.deployment.manifest)?
                != self.deployment.fingerprint
            || serde_json::to_value(&self.release).map_err(|_| review())?
                != serde_json::to_value(&plan.release).map_err(|_| review())?
        {
            return Err(review());
        }
        super::repository::validate_operation(plan)?;
        super::targets::validate_directory_path(self.directory().parent().ok_or_else(review)?)?;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementBoundary {
    AfterIntent,
    OldRenamed,
    NewRenamed,
    DatabaseCommitted,
    CleanupEntry,
    CleanupFinished,
}
pub fn stage(
    repository: &Arc<Repository>,
    plan: &PreparedOperation,
    source: &VerifiedArtifact,
) -> Result<ReplacementIntent, SdsError> {
    stage_inner(repository, plan, source, true)
}
pub(crate) fn stage_prepared(
    repository: &Arc<Repository>,
    plan: &PreparedOperation,
    source: &VerifiedArtifact,
) -> Result<ReplacementIntent, SdsError> {
    stage_inner(repository, plan, source, false)
}
fn stage_inner(
    repository: &Arc<Repository>,
    plan: &PreparedOperation,
    source: &VerifiedArtifact,
    publish: bool,
) -> Result<ReplacementIntent, SdsError> {
    super::repository::validate_operation(plan)?;
    if !plan.can_execute || !["update", "restore"].contains(&plan.kind.as_str()) {
        return Err(review());
    }
    let parent = plan.target.path.parent().ok_or_else(review)?;
    let pins = plan.reviewed_root.pin(parent).map_err(|_| review())?;
    let root_pins = DirectoryPins::create(parent)?;
    let root_identity = identity(root_pins.leaf())?;
    let _lock = target_lock(repository, &root_identity, &plan.release.install_name)?;
    let options = repository.options(plan)?;
    let existing = repository
        .deployments()?
        .into_iter()
        .find(|d| Some(&d.id) == plan.deployment_id.as_ref());
    let (old_identity, old_manifest) = match &existing {
        Some(d) => {
            if d.target != plan.target || d.root_identity != root_identity {
                return Err(review());
            }
            let target = DirectoryPins::acquire(&plan.target.path)?;
            if identity(target.leaf())? != d.identity {
                return Err(review());
            }
            let (m, f) = fingerprint_tree(&plan.target.path)?;
            if Some(&f) != plan.expected_fingerprint.as_ref() {
                return Err(review());
            }
            if f != d.fingerprint && !options.protect_current {
                return Err(SdsError::new(
                    "LOCAL_EDITS",
                    "Back up current local edits before replacement.",
                    false,
                ));
            }
            if options.protect_current {
                SnapshotService::new(repository.clone()).capture_locked(
                    d,
                    &f,
                    if plan.kind == "restore" {
                        SnapshotReason::BeforeRestore
                    } else {
                        SnapshotReason::BeforeUpdate
                    },
                )?;
            }
            (Some(d.identity.clone()), Some(m))
        }
        None => {
            if plan.expected_fingerprint.is_some()
                || std::fs::symlink_metadata(&plan.target.path).is_ok()
            {
                return Err(review());
            }
            (None, None)
        }
    };
    let (baseline_manifest, baseline_fingerprint) = if let Some(id) = &options.snapshot_id {
        let snapshot = SnapshotService::new(repository.clone()).verify(id)?;
        if snapshot.target != plan.target
            || snapshot.deployment_id != plan.deployment_id.clone().ok_or_else(review)?
            || snapshot.profile_id != plan.context.profile_id
            || snapshot.release.skill_id != plan.release.skill_id
            || snapshot.root_identity != root_identity
        {
            return Err(review());
        }
        if snapshot.content_fingerprint != source.fingerprint {
            return Err(review());
        }
        (snapshot.baseline_manifest, snapshot.baseline_fingerprint)
    } else {
        let cached = repository
            .cached_release(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?
            .ok_or_else(review)?;
        if cached.artifact.fingerprint != source.fingerprint {
            return Err(review());
        }
        (source.manifest.clone(), source.fingerprint.clone())
    };
    if fingerprint_tree(&source.root)? != (source.manifest.clone(), source.fingerprint.clone()) {
        return Err(review());
    }
    let mut intent = ReplacementIntent {
        version: 2,
        operation_id: plan.id.clone(),
        root_identity: root_identity.clone(),
        stage_identity: root_identity.clone(),
        new_fingerprint: source.fingerprint.clone(),
        old_identity,
        old_fingerprint: plan.expected_fingerprint.clone(),
        old_manifest,
        new_manifest: source.manifest.clone(),
        release: plan.release.clone(),
        snapshot_id: options.snapshot_id,
        deployment: Deployment {
            id: plan.deployment_id.clone().ok_or_else(review)?,
            release_key: cache_key(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?,
            target: plan.target.clone(),
            identity: root_identity.clone(),
            root_identity,
            manifest: baseline_manifest,
            fingerprint: baseline_fingerprint,
            mcp: plan.mcp.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        },
    };
    let _dir = DirectoryPins::create(&intent.directory())?;
    std::fs::create_dir(intent.stage_path()).map_err(|_| review())?;
    copy_manifest(&source.root, &intent.stage_path(), &source.manifest)?;
    if fingerprint_tree(&intent.stage_path())?
        != (source.manifest.clone(), source.fingerprint.clone())
        || fingerprint_tree(&source.root)? != (source.manifest.clone(), source.fingerprint.clone())
    {
        return Err(review());
    }
    let stage = DirectoryPins::acquire(&intent.stage_path())?;
    intent.stage_identity = identity(stage.leaf())?;
    intent.deployment.identity = intent.stage_identity.clone();
    if intent.stage_identity.volume != intent.root_identity.volume {
        return Err(review());
    }
    intent.validate(plan)?;
    drop(pins);
    if publish {
        repository.save_replacement(plan, &intent)?;
    }
    Ok(intent)
}
pub fn commit(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &ReplacementIntent,
) -> Result<(), SdsError> {
    commit_inner(repository, plan, intent, &mut |_| Ok(()))
}
#[cfg(test)]
pub(crate) fn commit_test_hook(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &ReplacementIntent,
    mut hook: impl FnMut(ReplacementBoundary) -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    commit_inner(repository, plan, intent, &mut hook)
}
fn matches(path: &Path, id: &FileIdentity, fingerprint: &str) -> Result<bool, SdsError> {
    if absent(path)? {
        return Ok(false);
    }
    let p = DirectoryPins::acquire(path)?;
    Ok(identity(p.leaf())? == *id && fingerprint_tree(path)?.1 == fingerprint)
}
fn absent(path: &Path) -> Result<bool, SdsError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Ok(_) => Ok(false),
        Err(_) => Err(review()),
    }
}
fn commit_inner(
    repository: &Repository,
    plan: &PreparedOperation,
    intent: &ReplacementIntent,
    hook: &mut impl FnMut(ReplacementBoundary) -> Result<(), SdsError>,
) -> Result<(), SdsError> {
    intent.validate(plan)?;
    let parent = plan.target.path.parent().ok_or_else(review)?;
    let root = DirectoryPins::acquire(parent)?;
    if identity(root.leaf())? != intent.root_identity {
        return Err(review());
    }
    let _lock = target_lock(
        repository,
        &intent.root_identity,
        &plan.release.install_name,
    )?;
    let _directory = DirectoryPins::acquire(&intent.directory())?;
    let (saved, committed) = repository.replacement(&plan.id)?.ok_or_else(review)?;
    if serde_json::to_value(saved).map_err(|_| review())?
        != serde_json::to_value(intent).map_err(|_| review())?
    {
        return Err(review());
    }
    if !committed {
        hook(ReplacementBoundary::AfterIntent)?;
        let new_at_target = matches(
            &plan.target.path,
            &intent.stage_identity,
            &intent.new_fingerprint,
        )?;
        if !new_at_target {
            if !matches(
                &intent.stage_path(),
                &intent.stage_identity,
                &intent.new_fingerprint,
            )? {
                return Err(review());
            }
            if let (Some(old), Some(fingerprint)) = (&intent.old_identity, &intent.old_fingerprint)
            {
                if matches(&plan.target.path, old, fingerprint)? && absent(&intent.rollback_path())?
                {
                    let rename = RenameGuard::open(&plan.target.path)?;
                    if identity(&rename.file)? != *old {
                        return Err(review());
                    }
                    rename.rename_to(&intent.rollback_path())?;
                    drop(rename);
                    hook(ReplacementBoundary::OldRenamed)?;
                } else if !absent(&plan.target.path)?
                    || !matches(&intent.rollback_path(), old, fingerprint)?
                {
                    return Err(review());
                }
            } else if !absent(&plan.target.path)? || !absent(&intent.rollback_path())? {
                return Err(review());
            }
            let rename = RenameGuard::open(&intent.stage_path())?;
            if identity(&rename.file)? != intent.stage_identity {
                return Err(review());
            }
            rename.rename_to(&plan.target.path)?;
            drop(rename);
            hook(ReplacementBoundary::NewRenamed)?;
        } else if !absent(&intent.stage_path())? {
            return Err(review());
        }
        if !matches(
            &plan.target.path,
            &intent.stage_identity,
            &intent.new_fingerprint,
        )? {
            return Err(review());
        }
        if let (Some(old), Some(fingerprint)) = (&intent.old_identity, &intent.old_fingerprint) {
            if !matches(&intent.rollback_path(), old, fingerprint)? {
                return Err(review());
            }
        }
        repository.finish_replacement(plan, intent)?;
        hook(ReplacementBoundary::DatabaseCommitted)?;
    }
    if !matches(
        &plan.target.path,
        &intent.stage_identity,
        &intent.new_fingerprint,
    )? {
        return Err(review());
    }
    if let (Some(old), Some(manifest)) = (&intent.old_identity, &intent.old_manifest) {
        super::removal_cleanup::cleanup_tree(&intent.rollback_path(), old, manifest, || {
            hook(ReplacementBoundary::CleanupEntry)
        })?;
    }
    hook(ReplacementBoundary::CleanupFinished)?;
    repository.complete_replacement(plan)?;
    Ok(())
}
pub fn recover_pending(
    repository: &Repository,
    approved_roots: &[PathBuf],
) -> Result<(), SdsError> {
    let ids = repository.replacement_ids()?;
    for id in ids {
        let plan = repository.load_operation(&id)?;
        let (intent, _) = repository.replacement(&id)?.ok_or_else(review)?;
        let result = if !approved_roots
            .iter()
            .any(|root| Some(root.as_path()) == plan.target.path.parent())
        {
            Err(review())
        } else {
            commit(repository, &plan, &intent)
        };
        if result.is_err() {
            repository.record_result(&OperationResult{operation_id:id,state:OperationState::RecoveryRequired,stage:"recovery-required".into(),error:Some(SdsError::new("RECOVERY_REQUIRED","Retained replacement evidence cannot be reconciled safely. Keep current files and contact support.",false))})?;
        }
    }
    Ok(())
}
impl Repository {
    pub(crate) fn save_replacement(
        &self,
        plan: &PreparedOperation,
        intent: &ReplacementIntent,
    ) -> Result<(), SdsError> {
        if serde_json::to_value(self.load_operation(&plan.id)?).map_err(|_| review())?
            != serde_json::to_value(plan).map_err(|_| review())?
        {
            return Err(review());
        }
        let mut c = self.connection.lock().map_err(|_| review())?;
        let tx = c.transaction().map_err(|_| review())?;
        tx.execute(
            "INSERT INTO replacement_intents VALUES(?1,?2,0)",
            rusqlite::params![
                plan.id,
                serde_json::to_string(intent).map_err(|_| review())?
            ],
        )
        .map_err(|_| review())?;
        tx.execute(
            "UPDATE operations SET result_json=?1 WHERE operation_id=?2",
            rusqlite::params![
                serde_json::to_string(&OperationResult {
                    operation_id: plan.id.clone(),
                    state: OperationState::Committing,
                    stage: "committing".into(),
                    error: None
                })
                .map_err(|_| review())?,
                plan.id
            ],
        )
        .map_err(|_| review())?;
        tx.commit().map_err(|_| review())
    }
    pub(crate) fn replacement(
        &self,
        id: &str,
    ) -> Result<Option<(ReplacementIntent, bool)>, SdsError> {
        use rusqlite::OptionalExtension;
        let body:Option<(String,bool)>=self.connection.lock().map_err(|_|review())?.query_row("SELECT CASE WHEN length(CAST(intent_json AS BLOB))<=268435456 THEN intent_json ELSE NULL END,committed FROM replacement_intents WHERE operation_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|review())?;
        body.map(|(b, c)| Ok((serde_json::from_str(&b).map_err(|_| review())?, c)))
            .transpose()
    }
    fn replacement_ids(&self) -> Result<Vec<String>, SdsError> {
        let c = self.connection.lock().map_err(|_| review())?;
        let mut q = c
            .prepare("SELECT operation_id FROM replacement_intents WHERE committed<>2")
            .map_err(|_| review())?;
        let rows = q.query_map([], |r| r.get(0)).map_err(|_| review())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|_| review())
    }
    fn finish_replacement(
        &self,
        plan: &PreparedOperation,
        intent: &ReplacementIntent,
    ) -> Result<(), SdsError> {
        let mut c = self.connection.lock().map_err(|_| review())?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| review())?;
        let d = &intent.deployment;
        tx.execute(
            "INSERT OR IGNORE INTO releases VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                d.release_key,
                plan.context.profile_id,
                plan.release.skill_id,
                plan.release.id,
                serde_json::to_string(&plan.release).map_err(|_| review())?,
                serde_json::to_string(&d.manifest).map_err(|_| review())?,
                d.fingerprint,
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .map_err(|_| review())?;
        tx.execute("DELETE FROM deployments WHERE deployment_id=?1", [&d.id])
            .map_err(|_| review())?;
        tx.execute(
            "INSERT INTO deployments VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                d.id,
                d.release_key,
                super::journal::target_key(&intent.root_identity, &plan.release.install_name)?,
                serde_json::to_string(d).map_err(|_| review())?,
                serde_json::to_string(&d.manifest).map_err(|_| review())?,
                d.fingerprint,
                serde_json::to_string(&d.mcp).map_err(|_| review())?,
                d.installed_at
            ],
        )
        .map_err(|_| review())?;
        tx.execute(
            "UPDATE replacement_intents SET committed=1 WHERE operation_id=?1",
            [&plan.id],
        )
        .map_err(|_| review())?;
        tx.commit().map_err(|_| review())
    }
    fn complete_replacement(&self, plan: &PreparedOperation) -> Result<(), SdsError> {
        let mut c = self.connection.lock().map_err(|_| review())?;
        let tx = c.transaction().map_err(|_| review())?;
        tx.execute(
            "UPDATE replacement_intents SET committed=2 WHERE operation_id=?1",
            [&plan.id],
        )
        .map_err(|_| review())?;
        tx.execute(
            "UPDATE operations SET result_json=?1 WHERE operation_id=?2",
            rusqlite::params![
                serde_json::to_string(&OperationResult {
                    operation_id: plan.id.clone(),
                    state: OperationState::Completed,
                    stage: "completed".into(),
                    error: None
                })
                .map_err(|_| review())?,
                plan.id
            ],
        )
        .map_err(|_| review())?;
        tx.commit().map_err(|_| review())
    }
}
