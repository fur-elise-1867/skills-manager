use super::{
    contracts::*,
    deployment::DirectoryPins,
    error::SdsError,
    journal::review,
    lifecycle::{LifecycleOptions, OperationContext},
    operations::{blocking, check_cancel, check_mcp, OperationService},
    packages::VerifiedArtifact,
    snapshots::{SnapshotReason, SnapshotService},
};
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicBool, Arc};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LocalEdits {
    Keep,
    BackupAndReplace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    pub deployment_id: String,
    pub release_id: String,
    pub local_edits: LocalEdits,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedUpdateItem {
    pub deployment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<PreparedOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SdsError>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedUpdateSelection {
    pub selection_id: String,
    pub items: Vec<PreparedUpdateItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSelectionItem {
    pub deployment_id: String,
    pub operation_id: Option<String>,
    pub result: Option<OperationResult>,
    pub preflight_error: Option<SdsError>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSelectionResult {
    pub selection_id: String,
    pub items: Vec<UpdateSelectionItem>,
}
impl OperationService {
    pub async fn check_updates(&self) -> Result<super::updates_metadata::UpdateCheck, SdsError> {
        self.ensure_ready()?;
        let (_, context) = self.session.scope().await?;
        let known = self
            .repository
            .list_library()?
            .into_iter()
            .filter(|e| e.profile_id == context.profile_id)
            .map(|e| super::updates_metadata::KnownRelease {
                skill_id: e.release.skill_id,
                release_id: e.release.id,
            })
            .collect::<Vec<_>>();
        let check = self.store.check_updates(&known).await?;
        for item in &check.items {
            if matches!(
                item.status,
                super::updates_metadata::UpdateStatus::Unavailable
                    | super::updates_metadata::UpdateStatus::UnknownRelease
            ) {
                self.repository
                    .revoke_cache_skill(&context, &item.skill_id)?;
            }
        }
        Ok(check)
    }
    pub fn snapshots(&self) -> Result<Vec<super::snapshots::SnapshotRecord>, SdsError> {
        self.ensure_ready()?;
        SnapshotService::new(self.repository.clone()).list()
    }
    pub async fn capture_snapshot(
        &self,
        id: String,
        expected: String,
        reason: SnapshotReason,
    ) -> Result<super::snapshots::SnapshotRecord, SdsError> {
        self.ensure_ready()?;
        let repo = self.repository.clone();
        let targets = self.targets.clone();
        blocking(move || {
            let result = (|| {
                let d = super::removal::deployment(&repo, &id)?;
                targets.with_current_target(&d.target, || {
                    SnapshotService::new(repo.clone()).capture(&id, &expected, reason)
                })
            })();
            if let Err(error) = &result {
                log::warn!(
                    "snapshot deployment={id:?} result=failed code={:?}",
                    error.code
                );
            }
            result
        })
        .await
    }
    fn save_lifecycle(
        &self,
        plan: PreparedOperation,
        options: LifecycleOptions,
    ) -> Result<PreparedOperation, SdsError> {
        self.repository.create_operation(
            &plan,
            &OperationResult {
                operation_id: plan.id.clone(),
                state: OperationState::Prepared,
                stage: "review".into(),
                error: None,
            },
        )?;
        self.repository.save_options(&plan.id, &options)?;
        Ok(plan)
    }
    pub async fn prepare_restore(
        &self,
        snapshot_id: &str,
        deployment_id: &str,
        protect_current: bool,
    ) -> Result<PreparedOperation, SdsError> {
        self.ensure_ready()?;
        let snapshot = SnapshotService::new(self.repository.clone()).verify(snapshot_id)?;
        if snapshot.deployment_id != deployment_id {
            return Err(review());
        }
        self.targets
            .with_current_target(&snapshot.target, || Ok(()))?;
        let root = DirectoryPins::acquire(snapshot.target.path.parent().ok_or_else(review)?)?;
        if super::journal::identity(root.leaf())? != snapshot.root_identity {
            return Err(review());
        }
        let existing = self
            .repository
            .deployments()?
            .into_iter()
            .find(|d| d.id == deployment_id);
        let mut expected = None;
        if let Some(d) = existing {
            if d.target != snapshot.target
                || d.release_key
                    != super::repository::cache_key(
                        &snapshot.profile_id,
                        &snapshot.release.skill_id,
                        &self
                            .repository
                            .list_library()?
                            .into_iter()
                            .find(|e| e.id == d.release_key)
                            .ok_or_else(review)?
                            .release
                            .id,
                    )?
            {
                return Err(review());
            }
            let observation = super::library::inspect_deployment(&d);
            if !matches!(
                observation.status,
                DeploymentStatus::Installed | DeploymentStatus::Modified
            ) {
                return Err(review());
            }
            expected = observation.expected_fingerprint;
            if observation.status == DeploymentStatus::Modified && !protect_current {
                return Err(SdsError::new(
                    "LOCAL_EDITS",
                    "Back up current edits before restoring this snapshot.",
                    false,
                ));
            }
        } else if std::fs::symlink_metadata(&snapshot.target.path).is_ok() {
            return Err(SdsError::new(
                "TARGET_CONFLICT",
                "The recorded destination contains unmanaged files.",
                false,
            ));
        }
        let mcp = self
            .targets
            .inspect_mcp(snapshot.release.required_mcps.as_deref());
        let plan = PreparedOperation {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "restore".into(),
            deployment_id: Some(deployment_id.into()),
            release: snapshot.release,
            target: snapshot.target.clone(),
            reviewed_root: ReviewedRoot::capture(
                snapshot.target.path.parent().ok_or_else(review)?,
            )?,
            expected_fingerprint: expected,
            requires_mcp_ack: !matches!(mcp.state, McpState::Configured | McpState::NotRequired),
            mcp,
            can_execute: true,
            conflict: None,
            context: SessionContext {
                profile_id: snapshot.profile_id,
                account_id: "local-library".into(),
                generation: 0,
            },
            runtime_epoch: self.epoch.clone(),
        };
        self.save_lifecycle(
            plan,
            LifecycleOptions {
                snapshot_id: Some(snapshot_id.into()),
                protect_current,
                operation_context: Some(OperationContext::LocalMaintenance {
                    runtime_epoch: self.epoch.clone(),
                }),
                ..Default::default()
            },
        )
    }
    pub async fn prepare_update(
        &self,
        request: UpdateRequest,
    ) -> Result<PreparedOperation, SdsError> {
        self.ensure_ready()?;
        // Keep current is deliberately a non-mutation: no release/package request.
        if request.local_edits == LocalEdits::Keep {
            return Err(SdsError::new(
                "KEEP_CURRENT",
                "The current deployment was kept.",
                false,
            ));
        }
        let d = super::removal::deployment(&self.repository, &request.deployment_id)?;
        let old = self
            .repository
            .list_library()?
            .into_iter()
            .find(|e| e.id == d.release_key)
            .ok_or_else(review)?;
        let (_, context) = self.session.scope().await?;
        if old.profile_id != context.profile_id {
            return Err(review());
        }
        let release = self
            .authorized_release(
                &context,
                &InstallRequest {
                    skill_id: old.release.skill_id,
                    release_id: request.release_id,
                    target: TargetSelection {
                        agent_key: d.target.agent_key.clone(),
                        workspace_id: d.target.workspace_id.clone(),
                    },
                },
            )
            .await?;
        if release.install_name != old.release.install_name {
            return Err(SdsError::new("INSTALL_NAME_CHANGED","This release changes its installation folder. Keep the current deployment and contact the publisher.",false));
        }
        self.targets.with_current_target(&d.target, || Ok(()))?;
        let observation = super::library::inspect_deployment(&d);
        if !matches!(
            observation.status,
            DeploymentStatus::Installed | DeploymentStatus::Modified
        ) {
            return Err(review());
        }
        if release.id == old.release.id && observation.status == DeploymentStatus::Installed {
            return Err(SdsError::new(
                "ALREADY_INSTALLED",
                "This release is already installed.",
                false,
            ));
        }
        let mcp = self.targets.inspect_mcp(release.required_mcps.as_deref());
        let plan = PreparedOperation {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "update".into(),
            deployment_id: Some(d.id),
            release,
            target: d.target.clone(),
            reviewed_root: ReviewedRoot::capture(d.target.path.parent().ok_or_else(review)?)?,
            expected_fingerprint: observation.expected_fingerprint,
            mcp: mcp.clone(),
            requires_mcp_ack: !matches!(mcp.state, McpState::Configured | McpState::NotRequired),
            can_execute: true,
            conflict: None,
            context: context.clone(),
            runtime_epoch: self.epoch.clone(),
        };
        self.session
            .with_current_context(&context, || {
                self.save_lifecycle(
                    plan,
                    LifecycleOptions {
                        protect_current: observation.status == DeploymentStatus::Modified,
                        operation_context: Some(OperationContext::Store {
                            context: context.clone(),
                        }),
                        ..Default::default()
                    },
                )
            })
            .await
    }
    pub(crate) async fn reviewed_lifecycle(
        &self,
        id: &str,
        ack: bool,
    ) -> Result<PreparedOperation, SdsError> {
        let plan = self.repository.load_operation(id)?;
        if plan.runtime_epoch != self.epoch
            || !plan.can_execute
            || self.status(id)?.state != OperationState::Prepared
        {
            return Err(review());
        }
        if plan.requires_mcp_ack && !ack {
            return Err(SdsError::new(
                "MCP_ACK_REQUIRED",
                "Acknowledge this release's MCP warning before continuing.",
                false,
            ));
        }
        self.targets.with_current_target(&plan.target, || Ok(()))?;
        let _pins = plan
            .reviewed_root
            .pin(plan.target.path.parent().ok_or_else(review)?)
            .map_err(|_| review())?;
        if let Some(expected) = &plan.expected_fingerprint {
            if plan.kind == "install" {
                let d =
                    super::journal::managed_install(&self.repository, &plan)?.ok_or_else(review)?;
                if &d.fingerprint != expected {
                    return Err(review());
                }
            } else {
                let d = super::removal::deployment(
                    &self.repository,
                    plan.deployment_id.as_ref().ok_or_else(review)?,
                )?;
                if d.target != plan.target
                    || super::library::inspect_deployment(&d)
                        .expected_fingerprint
                        .as_ref()
                        != Some(expected)
                {
                    return Err(review());
                }
            }
        } else if std::fs::symlink_metadata(&plan.target.path).is_ok() {
            return Err(review());
        }
        match self
            .repository
            .options(&plan)?
            .operation_context
            .ok_or_else(review)?
        {
            OperationContext::Store { context } => {
                self.session
                    .with_current_context(&context, || Ok(()))
                    .await?;
                let current = self
                    .authorized_release(
                        &context,
                        &InstallRequest {
                            skill_id: plan.release.skill_id.clone(),
                            release_id: plan.release.id.clone(),
                            target: Default::default(),
                        },
                    )
                    .await?;
                if serde_json::to_value(current).map_err(|_| review())?
                    != serde_json::to_value(&plan.release).map_err(|_| review())?
                {
                    return Err(review());
                }
                check_mcp(&self.targets, &plan)?;
            }
            OperationContext::Cached { context } => {
                self.validate_cached_context(&context).await?;
                check_mcp(&self.targets, &plan)?;
            }
            OperationContext::LocalMaintenance { runtime_epoch } => {
                if runtime_epoch != self.epoch || plan.kind != "restore" {
                    return Err(review());
                }
                check_mcp(&self.targets, &plan)?;
                SnapshotService::new(self.repository.clone()).verify(
                    self.repository
                        .options(&plan)?
                        .snapshot_id
                        .as_deref()
                        .ok_or_else(review)?,
                )?;
            }
        }
        Ok(plan)
    }
    pub(crate) async fn run_restore(
        &self,
        plan: &PreparedOperation,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), SdsError> {
        let id = self
            .repository
            .options(plan)?
            .snapshot_id
            .ok_or_else(review)?;
        let service = SnapshotService::new(self.repository.clone());
        let artifact = blocking(move || service.artifact(&id)).await?;
        self.run_replacement(plan, artifact, cancel).await
    }
    pub(crate) async fn run_replacement(
        &self,
        plan: &PreparedOperation,
        artifact: VerifiedArtifact,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), SdsError> {
        check_cancel(&cancel)?;
        self.progress(&plan.id, OperationState::Staging, "staging")?;
        let (repo, frozen) = (self.repository.clone(), plan.clone());
        let intent =
            blocking(move || super::replacement::stage_prepared(&repo, &frozen, &artifact)).await?;
        let (repo, targets, frozen) = (self.repository.clone(), self.targets.clone(), plan.clone());
        let publish = move || {
            check_cancel(&cancel)?;
            targets.with_current_target(&frozen.target, || {
                check_mcp(&targets, &frozen)?;
                repo.save_replacement(&frozen, &intent)?;
                super::replacement::commit(&repo, &frozen, &intent)
            })
        };
        if plan.kind == "update" {
            self.session.commit_for(&plan.context, publish).await
        } else {
            blocking(publish).await
        }
    }
    pub async fn prepare_updates(
        &self,
        requests: Vec<UpdateRequest>,
    ) -> Result<PreparedUpdateSelection, SdsError> {
        self.ensure_ready()?;
        if requests.is_empty() || requests.len() > 1000 {
            return Err(SdsError::invalid("Select between one and 1000 deployments"));
        }
        let mut ids = std::collections::HashSet::new();
        let mut items = vec![];
        for request in requests {
            if !ids.insert(request.deployment_id.clone()) {
                return Err(SdsError::invalid("A deployment may only be selected once"));
            }
            let deployment_id = request.deployment_id.clone();
            match self.prepare_update(request).await {
                Ok(p) => items.push(PreparedUpdateItem {
                    deployment_id,
                    operation: Some(p.clone()),
                    operation_id: Some(p.id),
                    error: None,
                }),
                Err(error) => items.push(PreparedUpdateItem {
                    deployment_id,
                    operation: None,
                    operation_id: None,
                    error: Some(error),
                }),
            }
        }
        let selection = PreparedUpdateSelection {
            selection_id: uuid::Uuid::new_v4().to_string(),
            items,
        };
        self.repository
            .connection
            .lock()
            .map_err(|_| review())?
            .execute(
                "INSERT INTO update_selections VALUES(?1,?2)",
                rusqlite::params![
                    selection.selection_id,
                    serde_json::to_string(&selection).map_err(|_| review())?
                ],
            )
            .map_err(|_| review())?;
        Ok(selection)
    }
    pub(crate) fn selection(&self, id: &str) -> Result<PreparedUpdateSelection, SdsError> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(review());
        }
        let body: String = self
            .repository
            .connection
            .lock()
            .map_err(|_| review())?
            .query_row(
                "SELECT selection_json FROM update_selections WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| review())?;
        serde_json::from_str(&body).map_err(|_| review())
    }
    pub fn update_selection_status(&self, id: &str) -> Result<UpdateSelectionResult, SdsError> {
        let s = self.selection(id)?;
        Ok(UpdateSelectionResult {
            selection_id: s.selection_id,
            items: s
                .items
                .into_iter()
                .map(|i| {
                    Ok(UpdateSelectionItem {
                        deployment_id: i.deployment_id,
                        result: i
                            .operation_id
                            .as_deref()
                            .map(|id| self.status(id))
                            .transpose()?,
                        operation_id: i.operation_id,
                        preflight_error: i.error,
                    })
                })
                .collect::<Result<Vec<_>, SdsError>>()?,
        })
    }
    pub async fn execute_updates(
        &self,
        id: &str,
        acknowledged: Vec<String>,
    ) -> Result<Vec<OperationResult>, SdsError> {
        let s = self.selection(id)?;
        let mut results = vec![];
        for i in s.items {
            if let Some(id) = i.operation_id {
                let status = self.status(&id)?;
                if status.state != OperationState::Prepared {
                    results.push(status);
                    continue;
                }
                let result = match self.execute(&id, acknowledged.contains(&id)).await {
                    Ok(r) => r,
                    Err(e) => self.record_dispatch_failure(&id, e)?,
                };
                results.push(result);
            }
        }
        Ok(results)
    }
    pub fn cancel_update_selection(&self, id: &str) -> Result<UpdateSelectionResult, SdsError> {
        for i in self.selection(id)?.items {
            if let Some(id) = i.operation_id {
                let status = self.status(&id)?;
                if !status.state.terminal() && status.state != OperationState::RecoveryRequired {
                    let _ = self.cancel(&id)?;
                }
            }
        }
        self.update_selection_status(id)
    }
}
