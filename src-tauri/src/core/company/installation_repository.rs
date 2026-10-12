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
        let connection = Connection::open(path).map_err(|_| db_error())?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|_| db_error())?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS company_packages (hash TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_installations (id TEXT PRIMARY KEY, target_key TEXT NOT NULL UNIQUE, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS company_operations (id TEXT PRIMARY KEY, state TEXT NOT NULL, payload TEXT NOT NULL);
            PRAGMA user_version=1;").map_err(|_| db_error())?;
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
        transaction.execute("INSERT INTO company_installations(id,target_key,payload) VALUES (?1,?2,?3) ON CONFLICT(id) DO NOTHING", params![installation.id,installation.target.replace('\\',"/").to_lowercase(),serde_json::to_string(installation).map_err(|_| db_error())?]).map_err(|_| db_error())?;
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
