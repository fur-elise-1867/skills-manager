use super::{
    contracts::{FileIdentity, Release, Target},
    deployment::{fingerprint_tree, DirectoryPins},
    error::SdsError,
    journal::{identity, review, target_lock},
    packages::{Manifest, VerifiedArtifact},
    repository::{copy_manifest, Repository},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotReason {
    Manual,
    BeforeUpdate,
    BeforeRestore,
    BeforeRemove,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRecord {
    pub id: String,
    pub deployment_id: String,
    pub profile_id: String,
    pub release: Release,
    pub target: Target,
    pub captured_at: String,
    pub reason: SnapshotReason,
    pub content_manifest: Manifest,
    pub content_fingerprint: String,
    pub baseline_manifest: Manifest,
    pub baseline_fingerprint: String,
    pub root_identity: FileIdentity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotBoundary {
    BeforeCopy,
    AfterCopy,
}
pub struct SnapshotService {
    repository: Arc<Repository>,
}
impl SnapshotService {
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
    }
    pub fn capture(
        &self,
        deployment_id: &str,
        expected_fingerprint: &str,
        reason: SnapshotReason,
    ) -> Result<SnapshotRecord, SdsError> {
        let d = super::removal::deployment(&self.repository, deployment_id)?;
        let _parent = DirectoryPins::acquire(d.target.path.parent().ok_or_else(review)?)?;
        if identity(_parent.leaf())? != d.root_identity {
            return Err(review());
        }
        let _lock = target_lock(
            &self.repository,
            &d.root_identity,
            d.target
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(review)?,
        )?;
        self.capture_locked(&d, expected_fingerprint, reason)
    }
    pub(crate) fn capture_locked(
        &self,
        d: &super::journal::Deployment,
        expected: &str,
        reason: SnapshotReason,
    ) -> Result<SnapshotRecord, SdsError> {
        self.capture_inner(d, expected, reason, &mut |_| Ok(()))
    }
    #[cfg(test)]
    pub(crate) fn capture_test_hook(
        &self,
        deployment_id: &str,
        expected: &str,
        reason: SnapshotReason,
        mut hook: impl FnMut(SnapshotBoundary) -> Result<(), SdsError>,
    ) -> Result<SnapshotRecord, SdsError> {
        let d = super::removal::deployment(&self.repository, deployment_id)?;
        let root = DirectoryPins::acquire(d.target.path.parent().ok_or_else(review)?)?;
        if identity(root.leaf())? != d.root_identity {
            return Err(review());
        }
        let _lock = target_lock(
            &self.repository,
            &d.root_identity,
            d.target
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(review)?,
        )?;
        self.capture_inner(&d, expected, reason, &mut hook)
    }
    fn capture_inner(
        &self,
        d: &super::journal::Deployment,
        expected: &str,
        reason: SnapshotReason,
        hook: &mut impl FnMut(SnapshotBoundary) -> Result<(), SdsError>,
    ) -> Result<SnapshotRecord, SdsError> {
        let _source = DirectoryPins::acquire(&d.target.path)?;
        if identity(_source.leaf())? != d.identity {
            return Err(review());
        }
        let (manifest, before) = fingerprint_tree(&d.target.path)?;
        if before != expected
            || manifest.entries.len() > super::packages::PackageLimits::default().entries
        {
            return Err(review());
        }
        let entry = self
            .repository
            .list_library()?
            .into_iter()
            .find(|e| e.id == d.release_key)
            .ok_or_else(review)?;
        let snapshots = self.repository.root().join("snapshots");
        let _dest = DirectoryPins::create(&snapshots)?;
        let temp = tempfile::Builder::new()
            .prefix(".capture-")
            .tempdir_in(&snapshots)
            .map_err(|_| failed())?;
        let copy = temp.path().join("tree");
        std::fs::create_dir(&copy).map_err(|_| failed())?;
        hook(SnapshotBoundary::BeforeCopy)?;
        copy_manifest(&d.target.path, &copy, &manifest).map_err(|_| failed())?;
        hook(SnapshotBoundary::AfterCopy)?;
        if fingerprint_tree(&copy)? != (manifest.clone(), before.clone())
            || fingerprint_tree(&d.target.path)? != (manifest.clone(), before.clone())
            || identity(_source.leaf())? != d.identity
        {
            return Err(review());
        }
        let record = SnapshotRecord {
            id: uuid::Uuid::new_v4().to_string(),
            deployment_id: d.id.clone(),
            profile_id: entry.profile_id,
            release: entry.release,
            target: d.target.clone(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            reason,
            content_manifest: manifest,
            content_fingerprint: before,
            baseline_manifest: d.manifest.clone(),
            baseline_fingerprint: d.fingerprint.clone(),
            root_identity: d.root_identity.clone(),
        };
        std::fs::rename(&copy, snapshots.join(&record.id)).map_err(|_| failed())?;
        self.repository
            .connection
            .lock()
            .map_err(|_| failed())?
            .execute(
                "INSERT INTO snapshots VALUES(?1,?2)",
                rusqlite::params![
                    record.id,
                    serde_json::to_string(&record).map_err(|_| failed())?
                ],
            )
            .map_err(|_| failed())?;
        Ok(record)
    }
    pub fn list(&self) -> Result<Vec<SnapshotRecord>, SdsError> {
        let c = self.repository.connection.lock().map_err(|_| failed())?;
        let mut q = c
            .prepare("SELECT record_json FROM snapshots ORDER BY rowid DESC")
            .map_err(|_| failed())?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| failed())?;
        rows.map(|r| serde_json::from_str(&r.map_err(|_| failed())?).map_err(|_| failed()))
            .collect()
    }
    fn record(&self, id: &str) -> Result<SnapshotRecord, SdsError> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(corrupt());
        }
        let body:String=self.repository.connection.lock().map_err(|_|corrupt())?.query_row("SELECT CASE WHEN length(CAST(record_json AS BLOB))<=268435456 THEN record_json ELSE NULL END FROM snapshots WHERE id=?1",[id],|r|r.get(0)).map_err(|_|corrupt())?;
        let record: SnapshotRecord = serde_json::from_str(&body).map_err(|_| corrupt())?;
        if record.id != id
            || super::deployment::manifest_fingerprint(&record.content_manifest)?
                != record.content_fingerprint
            || super::deployment::manifest_fingerprint(&record.baseline_manifest)?
                != record.baseline_fingerprint
        {
            return Err(corrupt());
        }
        Ok(record)
    }
    pub fn verify(&self, id: &str) -> Result<SnapshotRecord, SdsError> {
        let record = self.record(id)?;
        if fingerprint_tree(&self.repository.root().join("snapshots").join(id))
            .map_err(|_| corrupt())?
            != (
                record.content_manifest.clone(),
                record.content_fingerprint.clone(),
            )
        {
            return Err(corrupt());
        }
        Ok(record)
    }
    pub fn artifact(&self, id: &str) -> Result<VerifiedArtifact, SdsError> {
        let s = self.verify(id)?;
        Ok(VerifiedArtifact {
            root: self.repository.root().join("snapshots").join(id),
            manifest: s.content_manifest,
            fingerprint: s.content_fingerprint,
        })
    }
}
fn failed() -> SdsError {
    SdsError::new(
        "SNAPSHOT_FAILED",
        "The snapshot could not be completed. Current skill files were kept.",
        false,
    )
}
fn corrupt() -> SdsError {
    SdsError::new(
        "SNAPSHOT_CORRUPT",
        "The snapshot is missing or its contents changed. Current skill files were kept.",
        false,
    )
}
