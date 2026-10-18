use super::{
    contracts::{FileIdentity, OperationResult, OperationState, PreparedOperation},
    deployment::{fingerprint_tree, DirectoryPins},
    error::SdsError,
    journal::{identity, review, RenameGuard},
    packages::Manifest,
    repository::Repository,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheDeletePlan {
    pub id: String,
    pub release_key: String,
    pub expected_fingerprint: String,
    pub runtime_epoch: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Saved {
    plan: CacheDeletePlan,
    result: OperationResult,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Intent {
    identity: FileIdentity,
    manifest: Manifest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheBoundary {
    AfterIntent,
    AfterRename,
    AfterDatabase,
    AfterCleanupEntry,
    AfterCleanup,
}
fn references(repository: &Repository, key: &str) -> Result<bool, SdsError> {
    if repository
        .deployments()?
        .iter()
        .any(|d| d.release_key == key)
    {
        return Ok(true);
    }
    let c = repository.connection.lock().map_err(|_| review())?;
    let mut q = c
        .prepare("SELECT frozen_json,result_json,intent_json IS NOT NULL OR EXISTS(SELECT 1 FROM replacement_intents r WHERE r.operation_id=operations.operation_id AND r.committed<>2) FROM operations")
        .map_err(|_| review())?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })
        .map_err(|_| review())?;
    for row in rows {
        let (p, s, durable_intent) = row.map_err(|_| review())?;
        let p: PreparedOperation = serde_json::from_str(&p).map_err(|_| review())?;
        let s: OperationResult = serde_json::from_str(&s).map_err(|_| review())?;
        if (!s.state.terminal() || durable_intent)
            && super::repository::cache_key(
                &p.context.profile_id,
                &p.release.skill_id,
                &p.release.id,
            )? == key
        {
            return Ok(true);
        }
    }
    Ok(false)
}
/// Called while owning the publication lease. A crashed pre-database deletion
/// still owns the original artifact, possibly renamed to its tombstone; do not
/// reserve or republish that key until recovery reconciles the durable intent.
/// After its DB commit, cleanup only owns the old tombstone, so a new cache or
/// snapshot-backed deployment with the same release key can be published safely.
pub(crate) fn ensure_reference_allowed(repository: &Repository, key: &str) -> Result<(), SdsError> {
    let c = repository.connection.lock().map_err(|_| review())?;
    let mut q = c
        .prepare(
            "SELECT plan_json FROM cache_cleanup WHERE intent_json IS NOT NULL AND committed=0",
        )
        .map_err(|_| review())?;
    let rows = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| review())?;
    for row in rows {
        let saved: Saved =
            serde_json::from_str(&row.map_err(|_| review())?).map_err(|_| review())?;
        if saved.plan.release_key == key {
            return Err(SdsError::new(
                "OPERATION_BUSY",
                "An interrupted cache deletion must be reconciled before using this release.",
                true,
            ));
        }
    }
    Ok(())
}
fn in_use() -> SdsError {
    SdsError::new(
        "RELEASE_IN_USE",
        "A deployment or pending operation still uses this release.",
        false,
    )
}
pub fn prepare(
    repository: &Repository,
    epoch: &str,
    key: &str,
) -> Result<CacheDeletePlan, SdsError> {
    let _lock = repository.cache_cleanup_lock()?;
    if references(repository, key)? {
        return Err(in_use());
    }
    let cache = repository.cached_key(key)?;
    let plan = CacheDeletePlan {
        id: uuid::Uuid::new_v4().to_string(),
        release_key: cache.key,
        expected_fingerprint: cache.artifact.fingerprint,
        runtime_epoch: epoch.into(),
    };
    let saved = Saved {
        plan: plan.clone(),
        result: OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Prepared,
            stage: "review".into(),
            error: None,
        },
    };
    repository
        .connection
        .lock()
        .map_err(|_| review())?
        .execute(
            "INSERT INTO cache_cleanup(id,plan_json) VALUES(?1,?2)",
            rusqlite::params![
                plan.id,
                serde_json::to_string(&saved).map_err(|_| review())?
            ],
        )
        .map_err(|_| review())?;
    Ok(plan)
}
pub fn execute(
    repository: &Repository,
    epoch: &str,
    id: &str,
) -> Result<OperationResult, SdsError> {
    execute_inner(repository, Some(epoch), id, &mut |_| Ok(()))
}
#[cfg(test)]
pub(crate) fn execute_test_hook(
    repository: &Repository,
    epoch: &str,
    id: &str,
    mut hook: impl FnMut(CacheBoundary) -> Result<(), SdsError>,
) -> Result<OperationResult, SdsError> {
    execute_inner(repository, Some(epoch), id, &mut hook)
}
fn execute_inner(
    repository: &Repository,
    epoch: Option<&str>,
    id: &str,
    hook: &mut impl FnMut(CacheBoundary) -> Result<(), SdsError>,
) -> Result<OperationResult, SdsError> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(review());
    }
    let _cache = repository.cache_cleanup_lock()?;
    let parent = repository.root().join("releases");
    let _pins = DirectoryPins::acquire(&parent)?;
    let (body, intent, committed): (String, Option<String>, u32) = repository
        .connection
        .lock()
        .map_err(|_| review())?
        .query_row(
            "SELECT plan_json,intent_json,committed FROM cache_cleanup WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| review())?;
    let mut saved: Saved = serde_json::from_str(&body).map_err(|_| review())?;
    if saved.plan.id != id
        || saved.plan.release_key.len() != 64
        || !saved
            .plan
            .release_key
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(review());
    }
    if committed == 2 {
        return Ok(saved.result);
    }
    if epoch.is_some_and(|e| e != saved.plan.runtime_epoch) && intent.is_none() {
        return Err(review());
    }
    if committed == 0 && references(repository, &saved.plan.release_key)? {
        return Err(in_use());
    }
    let original = parent.join(&saved.plan.release_key);
    let tombstone = parent.join(format!(".deleted-{id}"));
    let intent = if let Some(body) = intent {
        serde_json::from_str::<Intent>(&body).map_err(|_| review())?
    } else {
        let cache = repository.cached_key(&saved.plan.release_key)?;
        if cache.artifact.fingerprint != saved.plan.expected_fingerprint {
            return Err(review());
        }
        let pin = DirectoryPins::acquire(&original)?;
        let intent = Intent {
            identity: identity(pin.leaf())?,
            manifest: cache.artifact.manifest,
        };
        repository
            .connection
            .lock()
            .map_err(|_| review())?
            .execute(
                "UPDATE cache_cleanup SET intent_json=?1 WHERE id=?2",
                rusqlite::params![serde_json::to_string(&intent).map_err(|_| review())?, id],
            )
            .map_err(|_| review())?;
        intent
    };
    if super::deployment::manifest_fingerprint(&intent.manifest)? != saved.plan.expected_fingerprint
    {
        return Err(review());
    }
    hook(CacheBoundary::AfterIntent)?;
    if committed == 0 {
        let exists = |p: &std::path::Path| -> Result<bool, SdsError> {
            match std::fs::symlink_metadata(p) {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(_) => Err(review()),
            }
        };
        if exists(&original)? && !exists(&tombstone)? {
            let pin = DirectoryPins::acquire(&original)?;
            if identity(pin.leaf())? != intent.identity
                || fingerprint_tree(&original)?.1 != saved.plan.expected_fingerprint
            {
                return Err(review());
            }
            drop(pin);
            let rename = RenameGuard::open(&original)?;
            if identity(&rename.file)? != intent.identity {
                return Err(review());
            }
            rename.rename_to(&tombstone)?;
            drop(rename);
            hook(CacheBoundary::AfterRename)?;
        } else if exists(&original)? || !exists(&tombstone)? {
            return Err(review());
        }
        let pin = DirectoryPins::acquire(&tombstone)?;
        if identity(pin.leaf())? != intent.identity
            || fingerprint_tree(&tombstone)?.1 != saved.plan.expected_fingerprint
        {
            return Err(review());
        }
        drop(pin);
        let mut c = repository.connection.lock().map_err(|_| review())?;
        let tx = c.transaction().map_err(|_| review())?;
        tx.execute(
            "DELETE FROM cache_access WHERE release_key=?1",
            [&saved.plan.release_key],
        )
        .map_err(|_| review())?;
        tx.execute(
            "DELETE FROM releases WHERE cache_key=?1",
            [&saved.plan.release_key],
        )
        .map_err(|_| review())?;
        tx.execute("UPDATE cache_cleanup SET committed=1 WHERE id=?1", [id])
            .map_err(|_| review())?;
        tx.commit().map_err(|_| review())?;
        hook(CacheBoundary::AfterDatabase)?;
    }
    super::removal_cleanup::cleanup_tree(&tombstone, &intent.identity, &intent.manifest, || {
        hook(CacheBoundary::AfterCleanupEntry)
    })?;
    hook(CacheBoundary::AfterCleanup)?;
    saved.result = OperationResult {
        operation_id: id.into(),
        state: OperationState::Completed,
        stage: "cache-deleted".into(),
        error: None,
    };
    repository
        .connection
        .lock()
        .map_err(|_| review())?
        .execute(
            "UPDATE cache_cleanup SET committed=2,plan_json=?1 WHERE id=?2",
            rusqlite::params![serde_json::to_string(&saved).map_err(|_| review())?, id],
        )
        .map_err(|_| review())?;
    Ok(saved.result)
}
pub fn recover_pending(repository: &Repository) -> Result<(), SdsError> {
    let ids = {
        let c = repository.connection.lock().map_err(|_| review())?;
        let mut q = c
            .prepare("SELECT id FROM cache_cleanup WHERE intent_json IS NOT NULL AND committed<>2")
            .map_err(|_| review())?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| review())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|_| review())?
    };
    for id in ids {
        if execute_inner(repository, None, &id, &mut |_| Ok(())).is_err() {
            let c = repository.connection.lock().map_err(|_| review())?;
            let body: String = c
                .query_row(
                    "SELECT plan_json FROM cache_cleanup WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .map_err(|_| review())?;
            let mut saved: Saved = serde_json::from_str(&body).map_err(|_| review())?;
            saved.result = OperationResult {
                operation_id: id.clone(),
                state: OperationState::RecoveryRequired,
                stage: "recovery-required".into(),
                error: Some(review()),
            };
            c.execute(
                "UPDATE cache_cleanup SET plan_json=?1 WHERE id=?2",
                rusqlite::params![serde_json::to_string(&saved).map_err(|_| review())?, id],
            )
            .map_err(|_| review())?;
        }
    }
    Ok(())
}
impl super::operations::OperationService {
    pub async fn prepare_cache_delete(&self, key: String) -> Result<CacheDeletePlan, SdsError> {
        self.ensure_ready()?;
        let (repo, epoch) = (self.repository.clone(), self.epoch.clone());
        super::operations::blocking(move || prepare(&repo, &epoch, &key)).await
    }
    pub async fn execute_cache_delete(&self, id: String) -> Result<OperationResult, SdsError> {
        self.ensure_ready()?;
        let (repo, epoch) = (self.repository.clone(), self.epoch.clone());
        super::operations::blocking(move || execute(&repo, &epoch, &id)).await
    }
}
