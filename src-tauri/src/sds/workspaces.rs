use super::{
    contracts::{FileIdentity, WorkspaceRef},
    deployment::DirectoryPins,
    error::SdsError,
    journal::{identity, review},
    repository::Repository,
};
use rusqlite::params;
use std::{path::Path, sync::Arc};
pub struct WorkspaceService {
    repository: Arc<Repository>,
}
impl WorkspaceService {
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
    }
    pub fn add(&self, root: &Path) -> Result<WorkspaceRef, SdsError> {
        let pins = DirectoryPins::acquire(root)?;
        let ident = identity(pins.leaf())?;
        let body = serde_json::to_string(&ident).map_err(|_| review())?;
        let c = self.repository.connection.lock().map_err(|_| review())?;
        use rusqlite::OptionalExtension;
        if let Some((id, root)) = c
            .query_row(
                "SELECT id,root FROM workspaces WHERE identity_json=?1",
                [&body],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| review())?
        {
            return Ok(WorkspaceRef {
                id,
                root: root.into(),
            });
        }
        let id = uuid::Uuid::new_v4().to_string();
        c.execute(
            "INSERT INTO workspaces VALUES(?1,?2,?3)",
            params![id, root.to_str().ok_or_else(review)?, body],
        )
        .map_err(|_| review())?;
        Ok(WorkspaceRef {
            id,
            root: root.into(),
        })
    }
    pub fn list(&self) -> Result<Vec<WorkspaceRef>, SdsError> {
        let c = self.repository.connection.lock().map_err(|_| review())?;
        let mut q = c
            .prepare("SELECT id,root FROM workspaces ORDER BY root")
            .map_err(|_| review())?;
        let rows = q
            .query_map([], |r| {
                Ok(WorkspaceRef {
                    id: r.get(0)?,
                    root: r.get::<_, String>(1)?.into(),
                })
            })
            .map_err(|_| review())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|_| review())
    }
    pub fn verified(&self, id: &str) -> Result<WorkspaceRef, SdsError> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(review());
        }
        let (root, body): (String, String) = self
            .repository
            .connection
            .lock()
            .map_err(|_| review())?
            .query_row(
                "SELECT root,identity_json FROM workspaces WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| review())?;
        let root = std::path::PathBuf::from(root);
        let pins = DirectoryPins::acquire(&root).map_err(|_| review())?;
        if identity(pins.leaf())?
            != serde_json::from_str::<FileIdentity>(&body).map_err(|_| review())?
        {
            return Err(review());
        }
        Ok(WorkspaceRef {
            id: id.into(),
            root,
        })
    }
    pub fn remove(&self, id: &str) -> Result<(), SdsError> {
        if self
            .repository
            .deployments()?
            .iter()
            .any(|d| d.target.workspace_id == id)
            || self.repository.workspace_pending(id)?
        {
            return Err(SdsError::new("WORKSPACE_IN_USE","Remove deployments and reconcile pending operations before forgetting this project.",false));
        }
        self.repository
            .connection
            .lock()
            .map_err(|_| review())?
            .execute("DELETE FROM workspaces WHERE id=?1", [id])
            .map_err(|_| review())?;
        Ok(())
    }
}
impl Repository {
    pub(crate) fn workspace_pending(&self, id: &str) -> Result<bool, SdsError> {
        let c = self.connection.lock().map_err(|_| review())?;
        let mut q = c
            .prepare("SELECT frozen_json,result_json FROM operations")
            .map_err(|_| review())?;
        let rows = q
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|_| review())?;
        for row in rows {
            let (p, s) = row.map_err(|_| review())?;
            let p: super::contracts::PreparedOperation =
                serde_json::from_str(&p).map_err(|_| review())?;
            let s: super::contracts::OperationResult =
                serde_json::from_str(&s).map_err(|_| review())?;
            if p.target.workspace_id == id && !s.state.terminal() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
