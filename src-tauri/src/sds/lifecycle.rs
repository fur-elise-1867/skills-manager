use super::{
    contracts::{PreparedOperation, SessionContext},
    error::SdsError,
    journal::review,
    repository::Repository,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInstallContext {
    pub profile_id: String,
    pub account_id: String,
    pub generation: u64,
    pub runtime_epoch: String,
    pub release_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OperationContext {
    Store {
        context: SessionContext,
    },
    LocalMaintenance {
        #[serde(rename = "runtimeEpoch")]
        runtime_epoch: String,
    },
    Cached {
        context: CacheInstallContext,
    },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct LifecycleOptions {
    pub operation_context: Option<OperationContext>,
    pub snapshot_id: Option<String>,
    pub protect_current: bool,
    pub backup: bool,
    pub acknowledge_local_edits: bool,
    pub observed_manifest: Option<super::packages::Manifest>,
}
impl Repository {
    pub(crate) fn save_options(
        &self,
        id: &str,
        options: &LifecycleOptions,
    ) -> Result<(), SdsError> {
        self.connection
            .lock()
            .map_err(|_| review())?
            .execute(
                "INSERT INTO lifecycle_options VALUES(?1,?2)",
                rusqlite::params![id, serde_json::to_string(options).map_err(|_| review())?],
            )
            .map_err(|_| review())?;
        Ok(())
    }
    pub(crate) fn options(&self, plan: &PreparedOperation) -> Result<LifecycleOptions, SdsError> {
        use rusqlite::OptionalExtension;
        let body: Option<String> = self
            .connection
            .lock()
            .map_err(|_| review())?
            .query_row(
                "SELECT options_json FROM lifecycle_options WHERE operation_id=?1",
                [&plan.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| review())?;
        let mut options = body
            .map(|b| serde_json::from_str::<LifecycleOptions>(&b).map_err(|_| review()))
            .transpose()?
            .unwrap_or_default();
        if options.operation_context.is_none() {
            options.operation_context = Some(if plan.kind == "remove" {
                OperationContext::LocalMaintenance {
                    runtime_epoch: plan.runtime_epoch.clone(),
                }
            } else {
                OperationContext::Store {
                    context: plan.context.clone(),
                }
            });
        }
        Ok(options)
    }
}
