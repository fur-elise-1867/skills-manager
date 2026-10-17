use super::{
    contracts::{OperationResult, OperationState, PreparedOperation, Release},
    deployment::{
        fingerprint_tree, manifest_fingerprint, open_content_file, validate_relative, DirectoryPins,
    },
    error::SdsError,
    packages::{Manifest, VerifiedArtifact},
    targets::{is_reparse, validate_directory_path},
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct Repository {
    root: PathBuf,
    connection: Mutex<Connection>,
    _db_pin: File,
    _pins: DirectoryPins,
}
#[derive(Debug)]
pub struct CachedRelease {
    pub key: String,
    pub profile_id: String,
    pub release: Release,
    pub artifact: VerifiedArtifact,
}
pub(crate) struct PreparedCache {
    repository_root: PathBuf,
    cached: CachedRelease,
    insert: bool,
    copied: Option<PathBuf>,
    _temporary: Option<tempfile::TempDir>,
    _lock: File,
    _pins: DirectoryPins,
}
impl PreparedCache {
    /// Only rename and SQLite publication happen here. Caller owns session/cancel guards.
    pub(crate) fn publish(self, repository: &Repository) -> Result<CachedRelease, SdsError> {
        if repository.root() != self.repository_root {
            return Err(repository_error());
        }
        if self.insert {
            let mut connection = repository
                .connection
                .lock()
                .map_err(|_| repository_error())?;
            let tx = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| repository_error())?;
            if let Some(copied) = &self.copied {
                std::fs::rename(copied, &self.cached.artifact.root)
                    .map_err(|_| repository_error())?;
            }
            let release = &self.cached.release;
            tx.execute("INSERT INTO releases(cache_key,profile_id,skill_id,release_id,release_json,manifest_json,fingerprint,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![self.cached.key,self.cached.profile_id,release.skill_id,release.id,serde_json::to_string(release).map_err(|_|repository_error())?,serde_json::to_string(&self.cached.artifact.manifest).map_err(|_|repository_error())?,self.cached.artifact.fingerprint,chrono::Utc::now().to_rfc3339()]).map_err(|_|repository_error())?;
            tx.commit().map_err(|_| repository_error())?;
        }
        Ok(self.cached)
    }
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: String,
    pub profile_id: String,
    pub release: Release,
    pub cached_at: String,
}
struct Stored {
    key: String,
    profile_id: String,
    release: Release,
    manifest: Manifest,
    fingerprint: String,
}

impl Repository {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn coordinator_lock(&self) -> Result<File, SdsError> {
        self.exclusive_lock("operations.lock")
    }
    pub(crate) fn expire_precommit(&self, epoch: &str) -> Result<(), SdsError> {
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let stale = {
            let mut query=tx.prepare("SELECT operation_id,CASE WHEN length(CAST(result_json AS BLOB))<=65536 THEN result_json ELSE NULL END FROM operations WHERE runtime_epoch<>?1 AND intent_json IS NULL").map_err(|_|repository_error())?;
            let rows = query
                .query_map([epoch], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|_| repository_error())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| repository_error())?
        };
        for (id, body) in stale {
            let previous = parse_result(&body, &id)?;
            if previous.state.terminal() || previous.state == OperationState::RecoveryRequired {
                continue;
            }
            let result = OperationResult {
                operation_id: id.clone(),
                state: OperationState::Failed,
                stage: "review-required".into(),
                error: Some(super::journal::review()),
            };
            tx.execute(
                "UPDATE operations SET result_json=?1,updated_at=?2 WHERE operation_id=?3",
                params![
                    serde_json::to_string(&result).map_err(|_| operation_error())?,
                    chrono::Utc::now().to_rfc3339(),
                    id
                ],
            )
            .map_err(|_| repository_error())?;
        }
        tx.commit().map_err(|_| repository_error())
    }
    pub fn save_intent(
        &self,
        plan: &PreparedOperation,
        intent: &super::journal::CommitIntent,
    ) -> Result<(), SdsError> {
        intent.validate(plan)?;
        let current = self.load_operation(&plan.id)?;
        if serde_json::to_vec(&current).map_err(|_| operation_error())?
            != serde_json::to_vec(plan).map_err(|_| operation_error())?
        {
            return Err(operation_error());
        }
        let result = OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Committing,
            stage: "committing".into(),
            error: None,
        };
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let previous: String = transaction
            .query_row(
                "SELECT CASE WHEN length(CAST(result_json AS BLOB))<=65536 THEN result_json ELSE NULL END FROM operations WHERE operation_id=?1",
                [&plan.id],
                |row| row.get(0),
            )
            .map_err(|_| operation_error())?;
        if parse_result(&previous, &plan.id)?.state.terminal() {
            return Err(operation_error());
        }
        let written=transaction.execute("UPDATE operations SET intent_json=?1,result_json=?2,updated_at=?3 WHERE operation_id=?4 AND intent_json IS NULL",params![serde_json::to_string(intent).map_err(|_|operation_error())?,serde_json::to_string(&result).map_err(|_|operation_error())?,chrono::Utc::now().to_rfc3339(),plan.id]).map_err(|_|repository_error())?;
        if written != 1 {
            return Err(operation_error());
        }
        transaction.commit().map_err(|_| repository_error())
    }
    pub fn pending_intents(
        &self,
    ) -> Result<Vec<(PreparedOperation, super::journal::CommitIntent)>, SdsError> {
        let ids = {
            let connection = self.connection.lock().map_err(|_| repository_error())?;
            let mut query=connection.prepare("SELECT operation_id,CASE WHEN length(CAST(intent_json AS BLOB))<=65536 THEN intent_json ELSE NULL END FROM operations WHERE intent_json IS NOT NULL ORDER BY updated_at").map_err(|_|repository_error())?;
            let rows = query
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|_| repository_error())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| repository_error())?
        };
        ids.into_iter()
            .map(|(id, body)| {
                Ok((
                    self.load_operation(&id)?,
                    serde_json::from_str(&body).map_err(|_| operation_error())?,
                ))
            })
            .collect()
    }
    pub fn fail_intent(&self, result: &OperationResult) -> Result<(), SdsError> {
        if !matches!(
            result.state,
            OperationState::Failed | OperationState::Cancelled
        ) {
            return Err(operation_error());
        }
        let body = serde_json::to_string(result).map_err(|_| operation_error())?;
        parse_result(&body, &result.operation_id)?;
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let (previous, has_intent): (String, bool) = tx
            .query_row(
                "SELECT result_json,intent_json IS NOT NULL FROM operations WHERE operation_id=?1",
                [&result.operation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| operation_error())?;
        if !has_intent
            || parse_result(&previous, &result.operation_id)?
                .state
                .terminal()
        {
            return Err(operation_error());
        }
        tx.execute("UPDATE operations SET result_json=?1,intent_json=NULL,updated_at=?2 WHERE operation_id=?3",params![body,chrono::Utc::now().to_rfc3339(),result.operation_id]).map_err(|_|repository_error())?;
        tx.commit().map_err(|_| repository_error())
    }
    pub(crate) fn has_intent(&self, id: &str) -> Result<bool, SdsError> {
        self.connection
            .lock()
            .map_err(|_| repository_error())?
            .query_row(
                "SELECT intent_json IS NOT NULL FROM operations WHERE operation_id=?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|_| repository_error())
    }
    pub fn finish_deployment(
        &self,
        plan: &PreparedOperation,
        intent: &super::journal::CommitIntent,
    ) -> Result<(), SdsError> {
        intent.validate(plan)?;
        let cached = self
            .cached_release(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?
            .ok_or_else(operation_error)?;
        self.finish_deployment_verified(plan, intent, &cached)
    }
    pub(crate) fn finish_deployment_verified(
        &self,
        plan: &PreparedOperation,
        intent: &super::journal::CommitIntent,
        cached: &CachedRelease,
    ) -> Result<(), SdsError> {
        use super::journal::{target_key, Deployment};
        intent.validate(plan)?;
        if cached.key != intent.release_key
            || cached.artifact.fingerprint != intent.fingerprint
            || serde_json::to_value(&cached.release).map_err(|_| operation_error())?
                != serde_json::to_value(&plan.release).map_err(|_| operation_error())?
        {
            return Err(operation_error());
        }
        let deployment = Deployment {
            id: plan.id.clone(),
            release_key: cached.key.clone(),
            target: plan.target.clone(),
            identity: intent.stage_identity.clone(),
            root_identity: intent.root_identity.clone(),
            manifest: cached.artifact.manifest.clone(),
            fingerprint: intent.fingerprint.clone(),
            mcp: plan.mcp.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        };
        let result = OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Completed,
            stage: "completed".into(),
            error: None,
        };
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let (release_json,manifest_json):(String,String)=transaction.query_row("SELECT CASE WHEN length(CAST(release_json AS BLOB))<=2097152 THEN release_json ELSE NULL END,CASE WHEN length(CAST(manifest_json AS BLOB))<=268435456 THEN manifest_json ELSE NULL END FROM releases WHERE cache_key=?1",[&cached.key],|row|Ok((row.get(0)?,row.get(1)?))).map_err(|_|repository_error())?;
        if serde_json::from_str::<serde_json::Value>(&release_json)
            .map_err(|_| operation_error())?
            != serde_json::to_value(&plan.release).map_err(|_| operation_error())?
            || serde_json::from_str::<Manifest>(&manifest_json).map_err(|_| operation_error())?
                != cached.artifact.manifest
        {
            return Err(operation_error());
        }
        let (body,frozen,previous): (String,String,String) = transaction
            .query_row(
                "SELECT CASE WHEN length(CAST(intent_json AS BLOB))<=65536 THEN intent_json ELSE NULL END, CASE WHEN length(CAST(frozen_json AS BLOB))<=2097152 THEN frozen_json ELSE NULL END, CASE WHEN length(CAST(result_json AS BLOB))<=65536 THEN result_json ELSE NULL END FROM operations WHERE operation_id=?1",
                [&plan.id],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
            )
            .map_err(|_| operation_error())?;
        if serde_json::from_str::<serde_json::Value>(&body).map_err(|_| operation_error())?
            != serde_json::to_value(intent).map_err(|_| operation_error())?
        {
            return Err(operation_error());
        }
        if serde_json::from_str::<serde_json::Value>(&frozen).map_err(|_| operation_error())?
            != serde_json::to_value(plan).map_err(|_| operation_error())?
            || !matches!(
                parse_result(&previous, &plan.id)?.state,
                OperationState::Committing | OperationState::RecoveryRequired
            )
        {
            return Err(operation_error());
        }
        transaction.execute("INSERT INTO deployments(deployment_id,release_key,target_key,target_json,manifest_json,fingerprint,mcp_json,installed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![deployment.id,deployment.release_key,target_key(&intent.root_identity,&plan.release.install_name)?,serde_json::to_string(&deployment).map_err(|_|operation_error())?,serde_json::to_string(&deployment.manifest).map_err(|_|operation_error())?,deployment.fingerprint,serde_json::to_string(&deployment.mcp).map_err(|_|operation_error())?,deployment.installed_at]).map_err(|_|repository_error())?;
        transaction.execute("UPDATE operations SET intent_json=NULL,result_json=?1,updated_at=?2 WHERE operation_id=?3",params![serde_json::to_string(&result).map_err(|_|operation_error())?,chrono::Utc::now().to_rfc3339(),plan.id]).map_err(|_|repository_error())?;
        transaction.commit().map_err(|_| repository_error())
    }
    pub fn deployments(&self) -> Result<Vec<super::journal::Deployment>, SdsError> {
        let connection = self.connection.lock().map_err(|_| repository_error())?;
        let mut statement=connection.prepare("SELECT deployment_id,CASE WHEN length(CAST(target_json AS BLOB))<=268435456 THEN target_json ELSE NULL END,target_key FROM deployments ORDER BY installed_at").map_err(|_|repository_error())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|_| repository_error())?;
        let mut output = Vec::new();
        for row in rows {
            let (id, body, key) = row.map_err(|_| repository_error())?;
            let deployment: super::journal::Deployment =
                serde_json::from_str(&body).map_err(|_| operation_error())?;
            let name = deployment
                .target
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(operation_error)?;
            if id != deployment.id
                || super::journal::target_key(&deployment.root_identity, name)? != key
                || manifest_fingerprint(&deployment.manifest)? != deployment.fingerprint
            {
                return Err(operation_error());
            }
            super::targets::validate_directory_shape(&deployment.target.path)?;
            output.push(deployment);
        }
        Ok(output)
    }
    pub(crate) fn complete_existing(
        &self,
        plan: &PreparedOperation,
        deployment: &super::journal::Deployment,
    ) -> Result<(), SdsError> {
        validate_operation(plan)?;
        let result = OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Completed,
            stage: "already-installed".into(),
            error: None,
        };
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let body: String = tx
            .query_row(
                "SELECT target_json FROM deployments WHERE deployment_id=?1",
                [&deployment.id],
                |row| row.get(0),
            )
            .map_err(|_| operation_error())?;
        if serde_json::from_str::<serde_json::Value>(&body).map_err(|_| operation_error())?
            != serde_json::to_value(deployment).map_err(|_| operation_error())?
            || plan.expected_fingerprint.as_ref() != Some(&deployment.fingerprint)
            || deployment.release_key
                != cache_key(
                    &plan.context.profile_id,
                    &plan.release.skill_id,
                    &plan.release.id,
                )?
        {
            return Err(operation_error());
        }
        let (frozen, previous, intent): (String, String, Option<String>) = tx
            .query_row(
                "SELECT frozen_json,result_json,intent_json FROM operations WHERE operation_id=?1",
                [&plan.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| operation_error())?;
        if serde_json::from_str::<serde_json::Value>(&frozen).map_err(|_| operation_error())?
            != serde_json::to_value(plan).map_err(|_| operation_error())?
            || parse_result(&previous, &plan.id)?.state.terminal()
            || intent.is_some()
        {
            return Err(operation_error());
        }
        tx.execute(
            "UPDATE operations SET result_json=?1,updated_at=?2 WHERE operation_id=?3",
            params![
                serde_json::to_string(&result).map_err(|_| operation_error())?,
                chrono::Utc::now().to_rfc3339(),
                plan.id
            ],
        )
        .map_err(|_| repository_error())?;
        tx.commit().map_err(|_| repository_error())
    }
    pub fn create_operation(
        &self,
        plan: &PreparedOperation,
        initial: &OperationResult,
    ) -> Result<(), SdsError> {
        validate_operation(plan)?;
        if initial.operation_id != plan.id || initial.state != OperationState::Prepared {
            return Err(operation_error());
        }
        let body = serde_json::to_string(plan).map_err(|_| repository_error())?;
        let result = serde_json::to_string(initial).map_err(|_| repository_error())?;
        if body.len() > 2_097_152 {
            return Err(operation_error());
        }
        parse_result(&result, &plan.id)?;
        self.connection.lock().map_err(|_| repository_error())?.execute("INSERT INTO operations(operation_id,frozen_json,result_json,runtime_epoch,updated_at) VALUES(?1,?2,?3,?4,?5)",params![plan.id,body,result,plan.runtime_epoch,chrono::Utc::now().to_rfc3339()]).map_err(|_| repository_error())?;
        Ok(())
    }
    pub fn load_operation(&self, id: &str) -> Result<PreparedOperation, SdsError> {
        let connection = self.connection.lock().map_err(|_| repository_error())?;
        let body:String = connection.query_row("SELECT frozen_json FROM operations WHERE operation_id=?1 AND length(frozen_json)<=2097152",[id],|row|row.get(0)).map_err(|_| operation_error())?;
        let plan: PreparedOperation = serde_json::from_str(&body).map_err(|_| operation_error())?;
        if plan.id != id {
            return Err(operation_error());
        }
        validate_operation(&plan)?;
        Ok(plan)
    }
    pub fn operation_status(&self, id: &str) -> Result<OperationResult, SdsError> {
        let connection = self.connection.lock().map_err(|_| repository_error())?;
        let body:String = connection.query_row("SELECT result_json FROM operations WHERE operation_id=?1 AND length(result_json)<=65536",[id],|row|row.get(0)).map_err(|_| operation_error())?;
        parse_result(&body, id)
    }
    pub fn record_result(&self, result: &OperationResult) -> Result<(), SdsError> {
        // Successful deployment is committed atomically with its Library row in A07.
        if result.state == OperationState::Completed {
            return Err(operation_error());
        }
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let previous:String = transaction.query_row("SELECT result_json FROM operations WHERE operation_id=?1 AND length(result_json)<=65536",[&result.operation_id],|row|row.get(0)).map_err(|_| operation_error())?;
        let previous = parse_result(&previous, &result.operation_id)?;
        let next = serde_json::to_string(result).map_err(|_| operation_error())?;
        parse_result(&next, &result.operation_id)?;
        if previous.state.terminal() {
            if serde_json::to_string(&previous).map_err(|_| operation_error())? != next {
                return Err(operation_error());
            }
            return Ok(());
        }
        transaction
            .execute(
                "UPDATE operations SET result_json=?1,updated_at=?2 WHERE operation_id=?3",
                params![next, chrono::Utc::now().to_rfc3339(), result.operation_id],
            )
            .map_err(|_| repository_error())?;
        transaction.commit().map_err(|_| repository_error())
    }
    /// A dedicated SDS root, never the upstream settings/database location.
    pub fn open(root: &Path) -> Result<Self, SdsError> {
        let pins = DirectoryPins::create(root)?;
        let db = root.join("pilot.sqlite3");
        if let Ok(metadata) = std::fs::symlink_metadata(&db) {
            if is_reparse(&metadata) || !metadata.is_file() {
                return Err(repository_error());
            }
        }
        let db_pin = pin_database_file(&db, true)?;
        for suffix in ["-journal", "-wal", "-shm"] {
            let sidecar = root.join(format!("pilot.sqlite3{suffix}"));
            match std::fs::symlink_metadata(&sidecar) {
                Ok(_) => {
                    pin_database_file(&sidecar, false)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err(repository_error()),
            }
        }
        let connection = Connection::open_with_flags(
            db,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| repository_error())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| repository_error())?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE;",
            )
            .map_err(|_| repository_error())?;
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| repository_error())?;
        if version > 1 {
            return Err(repository_error());
        }
        if version == 0 {
            connection
                .execute_batch("BEGIN IMMEDIATE")
                .map_err(|_| repository_error())?;
            connection
                .execute_batch(include_str!("migrations/001_pilot.sql"))
                .and_then(|_| connection.execute_batch("COMMIT"))
                .map_err(|_| repository_error())?;
        }
        let cache = root.join("releases");
        let _cache_pins = DirectoryPins::create(&cache)?;
        Ok(Self {
            root: root.into(),
            connection: Mutex::new(connection),
            _db_pin: db_pin,
            _pins: pins,
        })
    }
    pub fn cache_release(
        &self,
        profile_id: &str,
        release: &Release,
        artifact: &VerifiedArtifact,
    ) -> Result<CachedRelease, SdsError> {
        self.prepare_cache(profile_id, release, artifact)?
            .publish(self)
    }
    pub(crate) fn prepare_cache(
        &self,
        profile_id: &str,
        release: &Release,
        artifact: &VerifiedArtifact,
    ) -> Result<PreparedCache, SdsError> {
        let pins = DirectoryPins::acquire(&self.root.join("releases"))?;
        let lock = self.cache_lock()?;
        let key = cache_key(profile_id, &release.skill_id, &release.id)?;
        let body = serde_json::to_string(release).map_err(|_| repository_error())?;
        if let Some(stored) = self.stored(profile_id, &release.skill_id, &release.id)? {
            if serde_json::to_string(&stored.release).map_err(|_| repository_error())? != body
                || stored.fingerprint != artifact.fingerprint
            {
                return Err(release_changed());
            }
            return Ok(PreparedCache {
                repository_root: self.root.clone(),
                cached: self.materialize(stored)?,
                insert: false,
                copied: None,
                _temporary: None,
                _lock: lock,
                _pins: pins,
            });
        }
        let (manifest, fingerprint) = fingerprint_tree(&artifact.root)?;
        if manifest != artifact.manifest
            || fingerprint != artifact.fingerprint
            || manifest_fingerprint(&manifest)? != fingerprint
        {
            return Err(cache_corrupt());
        }
        let cache_root = self.root.join("releases");
        let target = cache_root.join(&key);
        let (temporary, copied) = if std::fs::symlink_metadata(&target).is_ok() {
            if fingerprint_tree(&target)?.1 != fingerprint {
                return Err(cache_corrupt());
            }
            (None, None)
        } else {
            let temp = tempfile::Builder::new()
                .prefix(".cache-")
                .tempdir_in(&cache_root)
                .map_err(|_| repository_error())?;
            let copied = temp.path().join("tree");
            std::fs::create_dir(&copied).map_err(|_| repository_error())?;
            copy_manifest(&artifact.root, &copied, &manifest)?;
            if fingerprint_tree(&copied)?.1 != fingerprint {
                return Err(cache_corrupt());
            }
            (Some(temp), Some(copied))
        };
        Ok(PreparedCache {
            repository_root: self.root.clone(),
            cached: CachedRelease {
                key,
                profile_id: profile_id.into(),
                release: release.clone(),
                artifact: VerifiedArtifact {
                    root: target,
                    manifest,
                    fingerprint,
                },
            },
            insert: true,
            copied,
            _temporary: temporary,
            _lock: lock,
            _pins: pins,
        })
    }
    pub fn cached_release(
        &self,
        profile_id: &str,
        skill_id: &str,
        release_id: &str,
    ) -> Result<Option<CachedRelease>, SdsError> {
        let _pins = DirectoryPins::acquire(&self.root)?;
        self.stored(profile_id, skill_id, release_id)?
            .map(|stored| self.materialize(stored))
            .transpose()
    }
    pub fn list_library(&self) -> Result<Vec<LibraryEntry>, SdsError> {
        let connection = self.connection.lock().map_err(|_| repository_error())?;
        let mut statement = connection.prepare("SELECT CASE WHEN length(CAST(cache_key AS BLOB))=64 THEN cache_key ELSE NULL END,CASE WHEN length(CAST(profile_id AS BLOB))<=1024 THEN profile_id ELSE NULL END,CASE WHEN length(CAST(release_json AS BLOB))<=2097152 THEN release_json ELSE NULL END,CASE WHEN length(CAST(created_at AS BLOB))<=256 THEN created_at ELSE NULL END FROM releases ORDER BY created_at,cache_key").map_err(|_| repository_error())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| repository_error())?;
        let mut library = Vec::new();
        for row in rows {
            let (id, profile_id, body, cached_at) = row.map_err(|_| repository_error())?;
            let release: Release = serde_json::from_str(&body).map_err(|_| cache_corrupt())?;
            if cache_key(&profile_id, &release.skill_id, &release.id)? != id {
                return Err(cache_corrupt());
            }
            library.push(LibraryEntry {
                id,
                profile_id,
                release,
                cached_at,
            });
        }
        Ok(library)
    }
    fn stored(
        &self,
        profile_id: &str,
        skill_id: &str,
        release_id: &str,
    ) -> Result<Option<Stored>, SdsError> {
        let connection = self.connection.lock().map_err(|_| repository_error())?;
        let row: Option<(String,String,String,String)> = connection.query_row("SELECT CASE WHEN length(CAST(cache_key AS BLOB))=64 THEN cache_key ELSE '' END,CASE WHEN length(CAST(release_json AS BLOB))<=2097152 THEN release_json ELSE '' END,CASE WHEN length(CAST(manifest_json AS BLOB))<=268435456 THEN manifest_json ELSE '' END,CASE WHEN length(CAST(fingerprint AS BLOB))=64 THEN fingerprint ELSE '' END FROM releases WHERE profile_id=?1 AND skill_id=?2 AND release_id=?3",params![profile_id,skill_id,release_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(|_| repository_error())?;
        row.map(|(key, body, manifest, fingerprint)| {
            let release: Release = serde_json::from_str(&body).map_err(|_| cache_corrupt())?;
            let manifest: Manifest =
                serde_json::from_str(&manifest).map_err(|_| cache_corrupt())?;
            if key != cache_key(profile_id, skill_id, release_id)?
                || release.skill_id != skill_id
                || release.id != release_id
                || manifest_fingerprint(&manifest)? != fingerprint
            {
                return Err(cache_corrupt());
            }
            Ok(Stored {
                key,
                profile_id: profile_id.into(),
                release,
                manifest,
                fingerprint,
            })
        })
        .transpose()
    }
    fn materialize(&self, stored: Stored) -> Result<CachedRelease, SdsError> {
        let root = self.root.join("releases").join(&stored.key);
        let (manifest, fingerprint) = fingerprint_tree(&root).map_err(|_| cache_corrupt())?;
        if manifest != stored.manifest || fingerprint != stored.fingerprint {
            return Err(cache_corrupt());
        }
        Ok(CachedRelease {
            key: stored.key,
            profile_id: stored.profile_id,
            release: stored.release,
            artifact: VerifiedArtifact {
                root,
                manifest,
                fingerprint,
            },
        })
    }
    fn cache_lock(&self) -> Result<File, SdsError> {
        self.exclusive_lock("cache.lock")
    }
    fn exclusive_lock(&self, name: &str) -> Result<File, SdsError> {
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000).share_mode(3);
        }
        let lock = options
            .open(self.root.join(name))
            .map_err(|_| repository_error())?;
        let metadata = lock.metadata().map_err(|_| repository_error())?;
        if is_reparse(&metadata) || !metadata.is_file() {
            return Err(repository_error());
        }
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            SdsError::new(
                "OPERATION_BUSY",
                "Another process is using this SDS repository. Close it before retrying.",
                true,
            )
        })?;
        Ok(lock)
    }
}
fn pin_database_file(path: &Path, create: bool) -> Result<File, SdsError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .custom_flags(0x00200000)
            .share_mode(if create { 3 } else { 7 });
    }
    let file = if create {
        options.write(true).create_new(true);
        match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                options.create_new(false).write(false);
                options.open(path).map_err(|_| repository_error())?
            }
            Err(_) => return Err(repository_error()),
        }
    } else {
        options.open(path).map_err(|_| repository_error())?
    };
    let metadata = file.metadata().map_err(|_| repository_error())?;
    if is_reparse(&metadata) || !metadata.is_file() {
        return Err(repository_error());
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FileStandardInfo, GetFileInformationByHandleEx, FILE_STANDARD_INFO,
        };
        let mut info = FILE_STANDARD_INFO::default();
        // SAFETY: SDK writes the fixed standard-info structure into this live buffer.
        if unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileStandardInfo,
                (&mut info as *mut FILE_STANDARD_INFO).cast(),
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            )
        } == 0
            || info.NumberOfLinks != 1
        {
            return Err(repository_error());
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(repository_error());
        }
    }
    Ok(file)
}
pub(crate) fn cache_key(profile: &str, skill: &str, release: &str) -> Result<String, SdsError> {
    if [profile, skill, release]
        .iter()
        .any(|value| value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control))
    {
        return Err(repository_error());
    }
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(&(profile, skill, release)).map_err(|_| repository_error())?,
    )))
}
pub(crate) fn copy_manifest(
    source: &Path,
    target: &Path,
    manifest: &Manifest,
) -> Result<(), SdsError> {
    let _source_pins = DirectoryPins::acquire(source)?;
    let _target_pins = DirectoryPins::acquire(target)?;
    for entry in &manifest.entries {
        validate_relative(&entry.path)?;
        let path = target.join(&entry.path);
        if entry.sha256.is_none() {
            let _directory = DirectoryPins::create(&path)?;
            continue;
        }
        let source_path = source.join(&entry.path);
        let _read_pins = DirectoryPins::acquire(source_path.parent().ok_or_else(cache_corrupt)?)?;
        let _write_pins = DirectoryPins::create(path.parent().ok_or_else(cache_corrupt)?)?;
        let mut input = open_content_file(&source_path)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| repository_error())?;
        let mut size = 0u64;
        let mut buffer = [0u8; 65_536];
        loop {
            let n = input.read(&mut buffer).map_err(|_| cache_corrupt())?;
            if n == 0 {
                break;
            }
            size = size.checked_add(n as u64).ok_or_else(cache_corrupt)?;
            if size > entry.size_bytes {
                return Err(cache_corrupt());
            }
            output
                .write_all(&buffer[..n])
                .map_err(|_| repository_error())?;
        }
        if size != entry.size_bytes {
            return Err(cache_corrupt());
        }
        output.sync_all().map_err(|_| repository_error())?;
    }
    Ok(())
}
fn release_changed() -> SdsError {
    SdsError::new("RELEASE_CHANGED","This release is already cached with different immutable metadata. Review it with the Store administrator.",false)
}

pub(crate) fn validate_operation(plan: &PreparedOperation) -> Result<(), SdsError> {
    if uuid::Uuid::parse_str(&plan.id).is_err()
        || !["install", "remove"].contains(&plan.kind.as_str())
        || plan.runtime_epoch.is_empty()
        || plan.runtime_epoch.len() > 256
        || plan.context.account_id.is_empty()
        || plan.target.agent_key != "aipro"
        || plan.target.workspace_id != "global"
    {
        return Err(operation_error());
    }
    cache_key(
        &plan.context.profile_id,
        &plan.release.skill_id,
        &plan.release.id,
    )?;
    validate_relative(&plan.release.install_name)?;
    if plan.release.install_name.contains('/') {
        return Err(operation_error());
    }
    validate_directory_path(&plan.target.path)?;
    plan.reviewed_root
        .validate_shape(plan.target.path.parent().ok_or_else(operation_error)?)?;
    if plan.target.path.file_name().and_then(|name| name.to_str())
        != Some(&plan.release.install_name)
    {
        return Err(operation_error());
    }
    Ok(())
}
fn parse_result(body: &str, id: &str) -> Result<OperationResult, SdsError> {
    if body.len() > 65_536 {
        return Err(operation_error());
    }
    let result: OperationResult = serde_json::from_str(body).map_err(|_| operation_error())?;
    if result.operation_id != id || result.stage.is_empty() || result.stage.len() > 256 {
        return Err(operation_error());
    }
    Ok(result)
}
fn operation_error() -> SdsError {
    SdsError::new(
        "OPERATION_INVALID",
        "The saved operation cannot be used. Prepare and review this action again.",
        false,
    )
}
fn cache_corrupt() -> SdsError {
    SdsError::new("CACHE_CORRUPT","The cached release contents do not match the verified manifest. Keep existing installations and download a verified release again.",false)
}
fn repository_error() -> SdsError {
    SdsError::new(
        "REPOSITORY_UNAVAILABLE",
        "The SDS library could not be read or saved. Check disk space and permissions.",
        false,
    )
}
