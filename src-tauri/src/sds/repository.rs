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
}
#[derive(Debug)]
pub struct CachedRelease {
    pub key: String,
    pub profile_id: String,
    pub release: Release,
    pub artifact: VerifiedArtifact,
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
        validate_directory_path(root)?;
        std::fs::create_dir_all(root).map_err(|_| repository_error())?;
        let _pins = DirectoryPins::acquire(root)?;
        let db = root.join("pilot.sqlite3");
        if let Ok(metadata) = std::fs::symlink_metadata(&db) {
            if is_reparse(&metadata) || !metadata.is_file() {
                return Err(repository_error());
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
        validate_directory_path(&cache)?;
        std::fs::create_dir_all(cache).map_err(|_| repository_error())?;
        Ok(Self {
            root: root.into(),
            connection: Mutex::new(connection),
        })
    }
    pub fn cache_release(
        &self,
        profile_id: &str,
        release: &Release,
        artifact: &VerifiedArtifact,
    ) -> Result<CachedRelease, SdsError> {
        let _pins = DirectoryPins::acquire(&self.root)?;
        let _lock = self.cache_lock()?;
        let key = cache_key(profile_id, &release.skill_id, &release.id)?;
        let body = serde_json::to_string(release).map_err(|_| repository_error())?;
        if let Some(stored) = self.stored(profile_id, &release.skill_id, &release.id)? {
            if serde_json::to_string(&stored.release).map_err(|_| repository_error())? != body
                || stored.fingerprint != artifact.fingerprint
            {
                return Err(release_changed());
            }
            return self.materialize(stored);
        }
        let (manifest, fingerprint) = fingerprint_tree(&artifact.root)?;
        if manifest != artifact.manifest
            || fingerprint != artifact.fingerprint
            || manifest_fingerprint(&manifest)? != fingerprint
        {
            return Err(cache_corrupt());
        }
        let cache_root = self.root.join("releases");
        let _cache_pins = DirectoryPins::acquire(&cache_root)?;
        let target = cache_root.join(&key);
        if std::fs::symlink_metadata(&target).is_ok() {
            if fingerprint_tree(&target)?.1 != fingerprint {
                return Err(cache_corrupt());
            }
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
            std::fs::rename(&copied, &target).map_err(|_| repository_error())?;
        }
        let manifest_json = serde_json::to_string(&manifest).map_err(|_| repository_error())?;
        let created = chrono::Utc::now().to_rfc3339();
        let connection = self.connection.lock().map_err(|_| repository_error())?;
        connection.execute("INSERT INTO releases(cache_key,profile_id,skill_id,release_id,release_json,manifest_json,fingerprint,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![key,profile_id,release.skill_id,release.id,body,manifest_json,fingerprint,created]).map_err(|_| repository_error())?;
        drop(connection);
        self.materialize(Stored {
            key,
            profile_id: profile_id.into(),
            release: release.clone(),
            manifest,
            fingerprint,
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
        let mut statement = connection.prepare("SELECT cache_key,profile_id,release_json,created_at FROM releases ORDER BY created_at,cache_key").map_err(|_| repository_error())?;
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
        let row: Option<(String,String,String,String)> = connection.query_row("SELECT cache_key,release_json,manifest_json,fingerprint FROM releases WHERE profile_id=?1 AND skill_id=?2 AND release_id=?3",params![profile_id,skill_id,release_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(|_| repository_error())?;
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
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000);
        }
        let lock = options
            .open(self.root.join("cache.lock"))
            .map_err(|_| repository_error())?;
        if is_reparse(&lock.metadata().map_err(|_| repository_error())?) {
            return Err(repository_error());
        }
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            SdsError::new(
                "OPERATION_BUSY",
                "Another process is updating the SDS cache. Try again.",
                true,
            )
        })?;
        Ok(lock)
    }
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
            std::fs::create_dir_all(&path).map_err(|_| repository_error())?;
            continue;
        }
        let source_path = source.join(&entry.path);
        let _read_pins = DirectoryPins::acquire(source_path.parent().ok_or_else(cache_corrupt)?)?;
        std::fs::create_dir_all(path.parent().ok_or_else(cache_corrupt)?)
            .map_err(|_| repository_error())?;
        let _write_pins = DirectoryPins::acquire(path.parent().ok_or_else(cache_corrupt)?)?;
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

fn validate_operation(plan: &PreparedOperation) -> Result<(), SdsError> {
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
