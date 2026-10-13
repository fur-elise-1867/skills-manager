use super::types::*;
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex, time::Duration};

pub struct CompanyRepository {
    connection: Mutex<Connection>,
}
fn db_error() -> CompanyError {
    CompanyError::new(
        "DATABASE_ERROR",
        "Không cập nhật được dữ liệu quản lý trên máy.",
    )
}
impl CompanyRepository {
    pub fn open(path: &Path) -> CompanyResult<Self> {
        let mut connection = Connection::open(path).map_err(|_| db_error())?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| db_error())?;
        if version > 4 {
            return Err(CompanyError::new(
                "DATABASE_VERSION_UNSUPPORTED",
                "Dữ liệu thuộc phiên bản app mới hơn. Không thay đổi dữ liệu này.",
            ));
        }
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|_| db_error())?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS company_packages (hash TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_installations (id TEXT PRIMARY KEY, target_key TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_operations (id TEXT PRIMARY KEY, state TEXT NOT NULL, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_catalog_cache (store_key TEXT NOT NULL, user_id TEXT NOT NULL, query TEXT NOT NULL, page INTEGER NOT NULL, fetched_at TEXT NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(store_key,user_id,query,page));
            CREATE TABLE IF NOT EXISTS company_projects (id TEXT PRIMARY KEY, path_key TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_backups (id TEXT PRIMARY KEY, installation_id TEXT NOT NULL, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_replacements (id TEXT PRIMARY KEY, state TEXT NOT NULL, payload TEXT NOT NULL);").map_err(|_| db_error())?;
        let transaction = connection.transaction().map_err(|_| db_error())?;
        let legacy = {
            let mut query = transaction
                .prepare("SELECT id,target_key,payload FROM company_installations")
                .map_err(|_| db_error())?;
            let rows = query
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|_| db_error())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| db_error())?
        };
        for (id, old_key, payload) in legacy {
            let item: Installation = serde_json::from_str(&payload).map_err(|_| db_error())?;
            let key = super::aipro_paths::path_key(Path::new(&item.target));
            if key != old_key {
                transaction.execute("UPDATE company_installations SET target_key=?2 WHERE id=?1", params![id,key]).map_err(|_| CompanyError::new("DATABASE_MIGRATION_FAILED", "Ownership cũ chồng lấn khi chuẩn hóa đường dẫn. Dữ liệu được giữ để kiểm tra."))?;
            }
        }
        transaction
            .pragma_update(None, "user_version", 4)
            .map_err(|_| db_error())?;
        transaction.commit().map_err(|_| db_error())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    pub fn installations(&self) -> CompanyResult<Vec<Installation>> {
        let connection = self.connection.lock().map_err(|_| db_error())?;
        let mut query = connection
            .prepare("SELECT payload FROM company_installations ORDER BY id")
            .map_err(|_| db_error())?;
        let rows = query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| db_error())?;
        rows.map(|row| serde_json::from_str(&row.map_err(|_| db_error())?).map_err(|_| db_error()))
            .collect()
    }
    pub fn projects(&self) -> CompanyResult<Vec<Project>> {
        let connection = self.connection.lock().map_err(|_| db_error())?;
        let mut query = connection
            .prepare("SELECT payload FROM company_projects ORDER BY path_key")
            .map_err(|_| db_error())?;
        let rows = query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| db_error())?;
        rows.map(|row| serde_json::from_str(&row.map_err(|_| db_error())?).map_err(|_| db_error()))
            .collect()
    }
    pub fn save_project(&self, project: &Project) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "INSERT INTO company_projects(id,path_key,payload) VALUES(?1,?2,?3)",
                params![
                    project.id,
                    super::aipro_paths::path_key(Path::new(&project.path)),
                    serde_json::to_string(project).map_err(|_| db_error())?
                ],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn remove_project(&self, id: &str) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute("DELETE FROM company_projects WHERE id=?1", [id])
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn save_installation(&self, item: &Installation) -> CompanyResult<()> {
        let changed = self
            .connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "UPDATE company_installations SET payload=?2 WHERE id=?1",
                params![
                    item.id,
                    serde_json::to_string(item).map_err(|_| db_error())?
                ],
            )
            .map_err(|_| db_error())?;
        if changed != 1 {
            return Err(db_error());
        }
        Ok(())
    }
    pub fn save_backup(&self, backup: &Backup) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "INSERT INTO company_backups(id,installation_id,payload) VALUES(?1,?2,?3)",
                params![
                    backup.id,
                    backup.installation_id,
                    serde_json::to_string(backup).map_err(|_| db_error())?
                ],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn backups(&self, id: &str) -> CompanyResult<Vec<Backup>> {
        let connection = self.connection.lock().map_err(|_| db_error())?;
        let mut query = connection
            .prepare(
                "SELECT payload FROM company_backups WHERE installation_id=?1 ORDER BY rowid DESC",
            )
            .map_err(|_| db_error())?;
        let rows = query
            .query_map([id], |row| row.get::<_, String>(0))
            .map_err(|_| db_error())?;
        rows.map(|row| serde_json::from_str(&row.map_err(|_| db_error())?).map_err(|_| db_error()))
            .collect()
    }
    pub fn record_replacement(&self, journal: &ReplaceJournal) -> CompanyResult<()> {
        let phase = serde_json::to_value(&journal.phase).map_err(|_| db_error())?;
        self.connection.lock().map_err(|_| db_error())?.execute(
            "INSERT INTO company_replacements(id,state,payload) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET state=excluded.state,payload=excluded.payload",
            params![journal.view.id, phase.as_str().ok_or_else(db_error)?, serde_json::to_string(journal).map_err(|_| db_error())?]).map_err(|_| db_error())?;
        Ok(())
    }
    pub fn commit_replacement(&self, journal: &ReplaceJournal) -> CompanyResult<()> {
        let mut connection = self.connection.lock().map_err(|_| db_error())?;
        let transaction = connection.transaction().map_err(|_| db_error())?;
        let changed = transaction
            .execute(
                "UPDATE company_installations SET payload=?2 WHERE id=?1",
                params![
                    journal.new.id,
                    serde_json::to_string(&journal.new).map_err(|_| db_error())?
                ],
            )
            .map_err(|_| db_error())?;
        if changed != 1 {
            return Err(db_error());
        }
        transaction
            .execute(
                "UPDATE company_replacements SET state='committed',payload=?2 WHERE id=?1",
                params![
                    journal.view.id,
                    serde_json::to_string(journal).map_err(|_| db_error())?
                ],
            )
            .map_err(|_| db_error())?;
        transaction.commit().map_err(|_| db_error())
    }
    pub fn finish_replacement(&self, id: &str) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "UPDATE company_replacements SET state='completed' WHERE id=?1",
                [id],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn pending_replacements(&self) -> CompanyResult<Vec<ReplaceJournal>> {
        let connection = self.connection.lock().map_err(|_| db_error())?;
        let mut query = connection.prepare("SELECT payload FROM company_replacements WHERE state <> 'completed' ORDER BY rowid").map_err(|_| db_error())?;
        let rows = query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| db_error())?;
        rows.map(|row| serde_json::from_str(&row.map_err(|_| db_error())?).map_err(|_| db_error()))
            .collect()
    }
    pub fn cache_catalog(
        &self,
        store: &str,
        user: &str,
        query: &str,
        page: u32,
        result: &SkillPage,
    ) -> CompanyResult<()> {
        let mut connection = self.connection.lock().map_err(|_| db_error())?;
        let transaction = connection.transaction().map_err(|_| db_error())?;
        let fetched_at = chrono::Utc::now().to_rfc3339();
        transaction.execute("INSERT INTO company_catalog_cache(store_key,user_id,query,page,fetched_at,payload) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(store_key,user_id,query,page) DO UPDATE SET fetched_at=excluded.fetched_at,payload=excluded.payload", params![store,user,query.trim(),page,fetched_at,serde_json::to_string(result).map_err(|_| db_error())?]).map_err(|_| db_error())?;
        transaction.execute("DELETE FROM company_catalog_cache WHERE rowid IN (SELECT rowid FROM company_catalog_cache WHERE store_key=?1 AND user_id=?2 ORDER BY fetched_at DESC LIMIT -1 OFFSET 100)", params![store,user]).map_err(|_| db_error())?;
        transaction.commit().map_err(|_| db_error())
    }
    pub fn cached_catalog(
        &self,
        store: &str,
        user: &str,
        query: &str,
        page: u32,
    ) -> CompanyResult<Option<SkillPage>> {
        use rusqlite::OptionalExtension;
        let connection = self.connection.lock().map_err(|_| db_error())?;
        let row: Option<(String, String)> = connection.query_row("SELECT payload,fetched_at FROM company_catalog_cache WHERE store_key=?1 AND user_id=?2 AND query=?3 AND page=?4", params![store,user,query.trim(),page], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|_| db_error())?;
        row.map(|(payload, fetched_at)| {
            let mut result: SkillPage = serde_json::from_str(&payload).map_err(|_| db_error())?;
            result.cache = Some(CatalogCache {
                fetched_at,
                stale: true,
            });
            Ok(result)
        })
        .transpose()
    }
    pub fn clear_catalog(&self, store: &str) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "DELETE FROM company_catalog_cache WHERE store_key=?1",
                [store],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn prepare(&self, journal: &InstallJournal) -> CompanyResult<()> {
        let payload = serde_json::to_string(journal).map_err(|_| db_error())?;
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "INSERT INTO company_operations(id,state,payload) VALUES (?1,'prepared',?2)",
                params![journal.view.id, payload],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn stage_created(&self, journal: &InstallJournal) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "UPDATE company_operations SET payload=?2 WHERE id=?1",
                params![
                    journal.view.id,
                    serde_json::to_string(journal).map_err(|_| db_error())?
                ],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn save_package(
        &self,
        meta: &PackageMeta,
        content_hash: &str,
        full_hash: &str,
    ) -> CompanyResult<()> {
        let payload = serde_json::json!({"package":meta,"contentHash":content_hash,"fullHash":full_hash,"algorithm":"content-v1"}).to_string();
        self.connection.lock().map_err(|_| db_error())?.execute("INSERT INTO company_packages(hash,payload) VALUES (?1,?2) ON CONFLICT(hash) DO NOTHING", params![meta.sha256,payload]).map_err(|_| db_error())?;
        Ok(())
    }
    pub fn commit(&self, journal: &InstallJournal) -> CompanyResult<()> {
        let mut connection = self.connection.lock().map_err(|_| db_error())?;
        let transaction = connection.transaction().map_err(|_| db_error())?;
        let installation = &journal.installation;
        let changed = transaction.execute("INSERT INTO company_installations(id,target_key,payload) VALUES (?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload WHERE company_installations.target_key=excluded.target_key", params![installation.id,super::aipro_paths::path_key(Path::new(&installation.target)),serde_json::to_string(installation).map_err(|_| db_error())?]).map_err(|_| db_error())?;
        if changed != 1 {
            return Err(db_error());
        }
        transaction
            .execute(
                "UPDATE company_operations SET state='committed' WHERE id=?1",
                params![journal.view.id],
            )
            .map_err(|_| db_error())?;
        transaction.commit().map_err(|_| db_error())
    }
    pub fn finish(&self, id: &str, state: &str) -> CompanyResult<()> {
        self.connection
            .lock()
            .map_err(|_| db_error())?
            .execute(
                "UPDATE company_operations SET state=?2 WHERE id=?1",
                params![id, state],
            )
            .map_err(|_| db_error())?;
        Ok(())
    }
    pub fn pending(&self) -> CompanyResult<Vec<InstallJournal>> {
        let connection = self.connection.lock().map_err(|_| db_error())?;
        let mut query = connection.prepare("SELECT payload FROM company_operations WHERE state='prepared' OR state='recovery_required'").map_err(|_| db_error())?;
        let rows = query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| db_error())?;
        rows.map(|row| serde_json::from_str(&row.map_err(|_| db_error())?).map_err(|_| db_error()))
            .collect()
    }
}
