use super::{
    contracts::{
        InstallRequest, McpState, OperationResult, OperationState, PreparedOperation, Release,
        ReviewedRoot, SessionContext, Target,
    },
    error::SdsError,
    journal::review,
    repository::Repository,
    session::SessionService,
    store_client::StoreClient,
    targets::TargetService,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

type Jobs = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

pub struct OperationService {
    session: Arc<SessionService>,
    store: StoreClient,
    targets: Arc<TargetService>,
    repository: Arc<Repository>,
    epoch: String,
    jobs: Jobs,
    _coordinator: Option<std::fs::File>,
    startup_error: Option<SdsError>,
}
impl OperationService {
    pub fn new(
        session: Arc<SessionService>,
        targets: Arc<TargetService>,
        repository: Arc<Repository>,
    ) -> Self {
        let epoch = uuid::Uuid::new_v4().to_string();
        let (coordinator, startup_error) = match repository.coordinator_lock() {
            Ok(lock) => {
                let roots = targets
                    .settings()
                    .ok()
                    .and_then(|settings| settings.target)
                    .map(|target| vec![target.path])
                    .unwrap_or_default();
                let error = super::journal::recover_pending(&repository, &roots)
                    .and_then(|_| repository.expire_precommit(&epoch))
                    .err();
                (Some(lock), error)
            }
            Err(error) => (None, Some(error)),
        };
        Self {
            store: StoreClient::new(session.clone()),
            session,
            targets,
            repository,
            epoch,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            _coordinator: coordinator,
            startup_error,
        }
    }
    fn ensure_ready(&self) -> Result<(), SdsError> {
        match &self.startup_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
    pub fn status(&self, id: &str) -> Result<OperationResult, SdsError> {
        let jobs = self.jobs.lock().map_err(|_| operation_failed())?;
        self.status_for(id, jobs.contains_key(id))
    }
    fn status_for(&self, id: &str, active: bool) -> Result<OperationResult, SdsError> {
        let mut result = self.repository.operation_status(id)?;
        // The database records the deployment removal before resumable cleanup.
        // Callers must not present final success until cleanup has resolved.
        if result.state == OperationState::Completed && result.stage == "removal-cleanup" {
            if active {
                result.state = OperationState::Committing;
            } else {
                // Preserve the committed database marker/intent for restart recovery.
                // A failed final UPDATE must not leave callers waiting on an exited job.
                result.state = OperationState::RecoveryRequired;
                result.stage = "recovery-required".into();
                let mut error = SdsError::new("RECOVERY_REQUIRED", "Removal cleanup could not finish recording its result. Restart SDS to reconcile the retained journal.", false);
                error.operation_id = Some(id.into());
                result.error = Some(error);
            }
        }
        Ok(result)
    }
    pub(crate) fn record_dispatch_failure(
        &self,
        id: &str,
        mut error: SdsError,
    ) -> Result<OperationResult, SdsError> {
        let status = self.status(id)?;
        if error.code == "OPERATION_BUSY" {
            return Ok(status);
        }
        if status.state != OperationState::Prepared {
            return Ok(status);
        }
        error.operation_id = Some(id.into());
        let result = OperationResult {
            operation_id: id.into(),
            state: OperationState::Failed,
            stage: "review-required".into(),
            error: Some(error),
        };
        self.repository.record_result(&result)?;
        Ok(result)
    }
    pub(crate) fn saved_plan(&self, id: &str) -> Result<PreparedOperation, SdsError> {
        self.repository.load_operation(id)
    }
    pub async fn library(&self) -> Result<Vec<super::contracts::LibraryItem>, SdsError> {
        let repository = self.repository.clone();
        blocking(move || super::library::load(&repository)).await
    }
    pub async fn reveal_deployment(&self, id: String) -> Result<(), SdsError> {
        let repository = self.repository.clone();
        blocking(move || {
            super::library::with_deployment_path(&repository, &id, |path| {
                tauri_plugin_opener::reveal_item_in_dir(path).map_err(|_| {
                    SdsError::new(
                        "LOCATION_UNAVAILABLE",
                        "The destination could not be shown in File Explorer.",
                        false,
                    )
                })
            })
        })
        .await
    }
    pub async fn preview(&self, id: &str) -> Result<String, SdsError> {
        let (repository, id) = (self.repository.clone(), id.to_string());
        blocking(move || super::library::preview(&repository, &id)).await
    }
    pub async fn prepare_remove(&self, deployment_id: &str) -> Result<PreparedOperation, SdsError> {
        self.ensure_ready()?;
        let (repository, epoch, id) = (
            self.repository.clone(),
            self.epoch.clone(),
            deployment_id.to_string(),
        );
        blocking(move || super::removal::prepare(&repository, &epoch, &id)).await
    }
    pub fn cancel(&self, id: &str) -> Result<OperationResult, SdsError> {
        self.ensure_ready()?;
        let jobs = self.jobs.lock().map_err(|_| review())?;
        let status = self.status_for(id, jobs.contains_key(id))?;
        if status.state.terminal() {
            return Ok(status);
        }
        if let Some(cancel) = jobs.get(id) {
            cancel.store(true, Ordering::Release);
            return Ok(status);
        }
        if status.state != OperationState::Prepared {
            return Err(review());
        }
        let result = OperationResult {
            operation_id: id.into(),
            state: OperationState::Cancelled,
            stage: "cancelled".into(),
            error: None,
        };
        self.repository.record_result(&result)?;
        Ok(result)
    }
    pub async fn execute(&self, id: &str, ack_mcp: bool) -> Result<OperationResult, SdsError> {
        self.ensure_ready()?;
        let job = {
            let mut jobs = self.jobs.lock().map_err(|_| review())?;
            if jobs.contains_key(id) {
                return Err(SdsError::new(
                    "OPERATION_BUSY",
                    "This operation is already running.",
                    true,
                ));
            }
            let status = self.status_for(id, jobs.contains_key(id))?;
            if status.state.terminal() {
                return Ok(status);
            }
            if status.state != OperationState::Prepared {
                return Err(review());
            }
            let cancel = Arc::new(AtomicBool::new(false));
            jobs.insert(id.into(), cancel.clone());
            Job {
                id: id.into(),
                jobs: self.jobs.clone(),
                cancel,
            }
        };
        let plan = if self.repository.load_operation(id)?.kind == "remove" {
            self.reviewed_remove(id).await?
        } else {
            self.reviewed_install(id, ack_mcp).await?
        };
        let outcome = if plan.kind == "remove" {
            self.run_remove(&plan, job.cancel.clone()).await
        } else {
            self.run_install(&plan, job.cancel.clone()).await
        };
        match outcome {
            Ok(()) => self.status(id),
            Err(mut error) => {
                error.operation_id = Some(id.into());
                let has_intent = self.repository.has_intent(id)?;
                let state = if error.code == "CANCELLED" {
                    OperationState::Cancelled
                } else if has_intent {
                    OperationState::RecoveryRequired
                } else {
                    OperationState::Failed
                };
                let result = OperationResult {
                    operation_id: id.into(),
                    state,
                    stage: match state {
                        OperationState::Cancelled => "cancelled",
                        OperationState::RecoveryRequired => "recovery-required",
                        _ => "failed",
                    }
                    .into(),
                    error: Some(error),
                };
                if has_intent && state == OperationState::Cancelled {
                    self.repository.fail_intent(&result)?;
                } else {
                    self.repository.record_result(&result)?;
                }
                Ok(result)
            }
        }
    }
    fn progress(&self, id: &str, state: OperationState, stage: &str) -> Result<(), SdsError> {
        self.repository.record_result(&OperationResult {
            operation_id: id.into(),
            state,
            stage: stage.into(),
            error: None,
        })
    }
    async fn reviewed_remove(&self, id: &str) -> Result<PreparedOperation, SdsError> {
        let (repository, id, epoch) = (self.repository.clone(), id.to_string(), self.epoch.clone());
        blocking(move || {
            let plan = repository.load_operation(&id)?;
            if plan.runtime_epoch != epoch
                || repository.operation_status(&id)?.state != OperationState::Prepared
            {
                return Err(review());
            }
            let deployment = super::removal::reviewed_deployment(&repository, &plan)?;
            let _root = plan
                .reviewed_root
                .pin(plan.target.path.parent().ok_or_else(review)?)
                .map_err(|_| review())?;
            let status = super::library::inspect_deployment(&deployment).status;
            if (plan.expected_fingerprint.is_some()
                && status != super::contracts::DeploymentStatus::Installed)
                || (plan.expected_fingerprint.is_none()
                    && status != super::contracts::DeploymentStatus::Missing)
            {
                return Err(review());
            }
            Ok(plan)
        })
        .await
    }
    async fn run_remove(
        &self,
        plan: &PreparedOperation,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), SdsError> {
        check_cancel(&cancel)?;
        self.progress(&plan.id, OperationState::Validating, "checking-destination")?;
        let (repository, frozen) = (self.repository.clone(), plan.clone());
        if plan.expected_fingerprint.is_none() {
            return blocking(move || {
                check_cancel(&cancel)?;
                super::removal::remove_missing(&repository, &frozen)
            })
            .await;
        }
        let intent = blocking(move || super::removal::stage_remove(&repository, &frozen)).await?;
        let (repository, frozen) = (self.repository.clone(), plan.clone());
        blocking(move || {
            check_cancel(&cancel)?;
            super::removal::commit_remove(&repository, &frozen, &intent)
        })
        .await
    }
    async fn run_install(
        &self,
        plan: &PreparedOperation,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), SdsError> {
        use super::{
            journal::{CommitIntent, PreparedInstallCommit},
            packages::{prepare_package_cancellable, PackageLimits},
        };
        check_cancel(&cancel)?;
        self.progress(&plan.id, OperationState::Downloading, "downloading")?;
        let (repo, frozen) = (self.repository.clone(), plan.clone());
        let cached = blocking(move || {
            repo.cached_release(
                &frozen.context.profile_id,
                &frozen.release.skill_id,
                &frozen.release.id,
            )
        })
        .await?;
        let cached = if let Some(cached) = cached {
            if serde_json::to_value(&cached.release).map_err(|_| review())?
                != serde_json::to_value(&plan.release).map_err(|_| review())?
            {
                return Err(release_changed());
            }
            cached
        } else {
            let workspace = tempfile::Builder::new()
                .prefix(".operation-")
                .tempdir_in(self.repository.root())
                .map_err(|_| operation_failed())?;
            let archive = workspace.path().join("archive");
            self.store
                .download_for(&plan.context, &plan.release, &archive, &cancel)
                .await?;
            check_cancel(&cancel)?;
            self.progress(&plan.id, OperationState::Validating, "validating")?;
            let staging = workspace.path().join("verified");
            let (release, cancelled) = (plan.release.clone(), cancel.clone());
            let artifact = blocking(move || {
                prepare_package_cancellable(
                    &archive,
                    &release,
                    &staging,
                    &PackageLimits::default(),
                    &cancelled,
                )
            })
            .await?;
            check_cancel(&cancel)?;
            self.session
                .with_current_context(&plan.context, || Ok(()))
                .await?;
            let (repo, frozen) = (self.repository.clone(), plan.clone());
            let prepared = blocking(move || {
                repo.prepare_cache(&frozen.context.profile_id, &frozen.release, &artifact)
            })
            .await?;
            self.publish_cache(&plan.context, prepared, cancel.clone())
                .await?
        };
        check_cancel(&cancel)?;
        // Recheck permission/withdrawal and immutable metadata after any download.
        let current = self
            .authorized_release(
                &plan.context,
                &InstallRequest {
                    skill_id: plan.release.skill_id.clone(),
                    release_id: plan.release.id.clone(),
                },
            )
            .await?;
        if serde_json::to_value(current).map_err(|_| review())?
            != serde_json::to_value(&plan.release).map_err(|_| review())?
        {
            return Err(release_changed());
        }
        if self.destination(&plan.release)? != plan.target {
            return Err(review());
        }
        check_cancel(&cancel)?;
        if plan.expected_fingerprint.is_some() {
            let (repo, targets, frozen) =
                (self.repository.clone(), self.targets.clone(), plan.clone());
            return self
                .session
                .commit_for(&plan.context, move || {
                    check_cancel(&cancel)?;
                    targets.with_current_target(&frozen.target, || {
                        check_mcp(&targets, &frozen)?;
                        super::journal::confirm_existing(&repo, &frozen)
                    })
                })
                .await;
        }
        self.progress(&plan.id, OperationState::Staging, "staging")?;
        let (repo, frozen) = (self.repository.clone(), plan.clone());
        let intent =
            blocking(move || CommitIntent::stage_install(&repo, &frozen, &cached.artifact)).await?;
        let (repo, frozen) = (self.repository.clone(), plan.clone());
        let ready =
            blocking(move || PreparedInstallCommit::prepare(&repo, &frozen, &intent)).await?;
        let repo = self.repository.clone();
        let (targets, frozen) = (self.targets.clone(), plan.clone());
        self.session
            .commit_for(&plan.context, move || {
                check_cancel(&cancel)?;
                targets.with_current_target(&frozen.target, || {
                    check_mcp(&targets, &frozen)?;
                    ready.publish(&repo)
                })
            })
            .await
    }
    pub(crate) async fn publish_cache(
        &self,
        context: &SessionContext,
        prepared: super::repository::PreparedCache,
        cancel: Arc<AtomicBool>,
    ) -> Result<super::repository::CachedRelease, SdsError> {
        self.session
            .with_current_context(context, || {
                check_cancel(&cancel)?;
                prepared.publish(&self.repository)
            })
            .await
    }
    async fn authorized_release(
        &self,
        context: &SessionContext,
        request: &InstallRequest,
    ) -> Result<Release, SdsError> {
        let detail = self.store.detail_for(context, &request.skill_id).await?;
        if !detail.summary.metadata.can_download {
            return Err(SdsError::new(
                "DOWNLOAD_FORBIDDEN",
                "Your account cannot download this skill.",
                false,
            ));
        }
        let release = self
            .store
            .release_for(context, &request.skill_id, &request.release_id)
            .await?;
        super::deployment::validate_relative(&release.install_name)?;
        if release.install_name.contains('/') {
            return Err(SdsError::invalid("Invalid skill installation folder"));
        }
        self.session
            .with_current_context(context, || Ok(release))
            .await
    }
    fn destination(&self, release: &Release) -> Result<Target, SdsError> {
        let settings = self.targets.settings()?;
        let mut target = settings
            .target
            .ok_or_else(|| settings.error.unwrap_or_else(review))?;
        target.path = target.path.join(&release.install_name);
        Ok(target)
    }
    pub async fn prepare_install(
        &self,
        request: InstallRequest,
    ) -> Result<PreparedOperation, SdsError> {
        self.ensure_ready()?;
        let (_, context) = self.session.scope().await?;
        let release = self.authorized_release(&context, &request).await?;
        let target = self.destination(&release)?;
        let reviewed_root = ReviewedRoot::capture(target.path.parent().ok_or_else(review)?)?;
        let exists = match std::fs::symlink_metadata(&target.path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(review()),
        };
        let mcp = self.targets.inspect_mcp(release.required_mcps.as_deref());
        let mut plan = PreparedOperation {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "install".into(),
            deployment_id: None,
            release,
            target,
            reviewed_root,
            expected_fingerprint: None,
            requires_mcp_ack: !matches!(mcp.state, McpState::Configured | McpState::NotRequired),
            mcp,
            can_execute: true,
            conflict: None,
            context: context.clone(),
            runtime_epoch: self.epoch.clone(),
        };
        if exists {
            let (repo, frozen) = (self.repository.clone(), plan.clone());
            match blocking(move || super::journal::managed_install(&repo, &frozen)).await? {
                Some(deployment) => plan.expected_fingerprint = Some(deployment.fingerprint),
                None => {
                    plan.can_execute = false;
                    plan.conflict=Some("This destination contains unmanaged files, local edits or another release. Keep its files and choose a different destination.".into());
                }
            }
        } else {
            let (repo, frozen) = (self.repository.clone(), plan.clone());
            if blocking(move || super::journal::tracked_destination(&repo, &frozen)).await? {
                plan.can_execute = false;
                plan.conflict=Some("This managed destination is missing. Keep its Library record for review; automatic replacement is unavailable in the pilot.".into());
            }
        }
        self.session
            .with_current_context(&context, || {
                self.repository.create_operation(
                    &plan,
                    &OperationResult {
                        operation_id: plan.id.clone(),
                        state: OperationState::Prepared,
                        stage: "review".into(),
                        error: None,
                    },
                )?;
                Ok(plan)
            })
            .await
    }
    /// Re-authorize even when content will come from a previously verified cache.
    pub(crate) async fn reviewed_install(
        &self,
        id: &str,
        ack_mcp: bool,
    ) -> Result<PreparedOperation, SdsError> {
        self.ensure_ready()?;
        let plan = self.repository.load_operation(id)?;
        if plan.runtime_epoch != self.epoch
            || plan.kind != "install"
            || !plan.can_execute
            || self.status(id)?.state != OperationState::Prepared
        {
            return Err(review());
        }
        if plan.requires_mcp_ack && !ack_mcp {
            return Err(SdsError::new(
                "MCP_ACK_REQUIRED",
                "Review the MCP warning for this operation before installing.",
                false,
            ));
        }
        self.session
            .with_current_context(&plan.context, || Ok(()))
            .await?;
        let current = self
            .authorized_release(
                &plan.context,
                &InstallRequest {
                    skill_id: plan.release.skill_id.clone(),
                    release_id: plan.release.id.clone(),
                },
            )
            .await?;
        if serde_json::to_value(current).map_err(|_| review())?
            != serde_json::to_value(&plan.release).map_err(|_| review())?
        {
            return Err(SdsError::new(
                "RELEASE_CHANGED",
                "This release changed after review. Prepare and review the installation again.",
                false,
            ));
        }
        if self.destination(&plan.release)? != plan.target {
            return Err(review());
        }
        check_mcp(&self.targets, &plan)?;
        let _pins = plan
            .reviewed_root
            .pin(plan.target.path.parent().ok_or_else(review)?)
            .map_err(|_| review())?;
        if let Some(expected) = &plan.expected_fingerprint {
            let (repo, frozen) = (self.repository.clone(), plan.clone());
            let deployment = blocking(move || super::journal::managed_install(&repo, &frozen))
                .await?
                .ok_or_else(review)?;
            if &deployment.fingerprint != expected {
                return Err(review());
            }
        } else {
            match std::fs::symlink_metadata(&plan.target.path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                _ => return Err(review()),
            }
        }
        self.session
            .with_current_context(&plan.context, || Ok(plan.clone()))
            .await
    }
}

struct Job {
    id: String,
    jobs: Jobs,
    cancel: Arc<AtomicBool>,
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.remove(&self.id);
        }
    }
}
fn check_cancel(cancel: &AtomicBool) -> Result<(), SdsError> {
    if cancel.load(Ordering::Acquire) {
        Err(SdsError::new(
            "CANCELLED",
            "The installation was cancelled before commit.",
            false,
        ))
    } else {
        Ok(())
    }
}
fn operation_failed() -> SdsError {
    SdsError::new(
        "OPERATION_FAILED",
        "The installation could not finish. Review its status before retrying.",
        false,
    )
}
fn release_changed() -> SdsError {
    SdsError::new(
        "RELEASE_CHANGED",
        "This release changed after review. Prepare and review the installation again.",
        false,
    )
}
fn check_mcp(targets: &TargetService, plan: &PreparedOperation) -> Result<(), SdsError> {
    let current = targets.inspect_mcp(plan.release.required_mcps.as_deref());
    if current.state != plan.mcp.state
        || current.missing != plan.mcp.missing
        || current.reason != plan.mcp.reason
    {
        return Err(review());
    }
    Ok(())
}
async fn blocking<T: Send + 'static>(
    action: impl FnOnce() -> Result<T, SdsError> + Send + 'static,
) -> Result<T, SdsError> {
    tokio::task::spawn_blocking(action)
        .await
        .map_err(|_| operation_failed())?
}
