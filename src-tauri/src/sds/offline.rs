use super::{
    contracts::*,
    error::SdsError,
    journal::review,
    lifecycle::{CacheInstallContext, LifecycleOptions, OperationContext},
    operations::{blocking, check_cancel, check_mcp, OperationService},
    repository::Repository,
};
use std::sync::{atomic::AtomicBool, Arc};
impl Repository {
    pub(crate) fn revoke_cache_skill(
        &self,
        c: &SessionContext,
        skill: &str,
    ) -> Result<(), SdsError> {
        self.connection.lock().map_err(|_|review())?.execute("DELETE FROM cache_access WHERE profile_id=?1 AND account_id=?2 AND release_key IN (SELECT cache_key FROM releases WHERE profile_id=?1 AND skill_id=?3)",rusqlite::params![c.profile_id,c.account_id,skill]).map_err(|_|review())?;
        Ok(())
    }
    pub(crate) fn grant_cache(&self, c: &SessionContext, key: &str) -> Result<(), SdsError> {
        self.connection
            .lock()
            .map_err(|_| review())?
            .execute(
                "INSERT OR REPLACE INTO cache_access VALUES(?1,?2,?3,?4)",
                rusqlite::params![
                    c.profile_id,
                    c.account_id,
                    key,
                    chrono::Utc::now().to_rfc3339()
                ],
            )
            .map_err(|_| review())?;
        Ok(())
    }
    pub(crate) fn has_cache_grant(&self, c: &CacheInstallContext) -> Result<bool, SdsError> {
        self.connection.lock().map_err(|_|review())?.query_row("SELECT EXISTS(SELECT 1 FROM cache_access WHERE profile_id=?1 AND account_id=?2 AND release_key=?3)",rusqlite::params![c.profile_id,c.account_id,c.release_key],|r|r.get(0)).map_err(|_|review())
    }
    pub(crate) fn cached_key(
        &self,
        key: &str,
    ) -> Result<super::repository::CachedRelease, SdsError> {
        let e = self
            .list_library()?
            .into_iter()
            .find(|e| e.id == key)
            .ok_or_else(|| {
                SdsError::new("CACHE_MISSING", "This release is no longer cached.", false)
            })?;
        self.cached_release(&e.profile_id, &e.release.skill_id, &e.release.id)?
            .ok_or_else(review)
    }
}
impl OperationService {
    pub(crate) async fn validate_cached_context(
        &self,
        c: &CacheInstallContext,
    ) -> Result<(), SdsError> {
        if c.runtime_epoch != self.epoch {
            return Err(review());
        }
        let status = self.session.status().await?.status;
        if matches!(status, SessionStatus::SignedOut | SessionStatus::Expired) {
            return Err(SdsError::new(
                "SESSION_EXPIRED",
                "Sign in before deploying a cached Store release.",
                false,
            ));
        }
        if status != SessionStatus::Offline {
            return Err(SdsError::new(
                "REVIEW_REQUIRED",
                "The Store connection changed. Review this release online again.",
                false,
            ));
        }
        let (_, current) = self.session.scope().await?;
        if current.profile_id != c.profile_id
            || current.account_id != c.account_id
            || current.generation != c.generation
            || !self.repository.has_cache_grant(c)?
        {
            return Err(SdsError::new(
                "CACHE_NOT_AUTHORIZED",
                "This account has no prior verified download grant for the cached release.",
                false,
            ));
        }
        Ok(())
    }
    pub async fn prepare_cached_install(
        &self,
        key: &str,
        target: TargetSelection,
    ) -> Result<PreparedOperation, SdsError> {
        self.ensure_ready()?;
        let status = self.session.status().await?.status;
        if matches!(status, SessionStatus::SignedOut | SessionStatus::Expired) {
            return Err(SdsError::new(
                "SESSION_EXPIRED",
                "Sign in before deploying a cached Store release.",
                false,
            ));
        }
        let (_, context) = self.session.scope().await?;
        let c = CacheInstallContext {
            profile_id: context.profile_id.clone(),
            account_id: context.account_id.clone(),
            generation: context.generation,
            runtime_epoch: self.epoch.clone(),
            release_key: key.into(),
        };
        self.validate_cached_context(&c).await?;
        let (repo, key) = (self.repository.clone(), key.to_string());
        let lookup_key = key.clone();
        let cached = blocking(move || repo.cached_key(&lookup_key)).await?;
        if cached.profile_id != context.profile_id {
            return Err(review());
        }
        let destination = self.destination(&cached.release, &target)?;
        let reviewed_root = ReviewedRoot::capture(destination.path.parent().ok_or_else(review)?)?;
        let mcp = self
            .targets
            .inspect_mcp(cached.release.required_mcps.as_deref());
        let mut plan = PreparedOperation {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "install".into(),
            deployment_id: None,
            release: cached.release,
            target: destination,
            reviewed_root,
            expected_fingerprint: None,
            mcp: mcp.clone(),
            requires_mcp_ack: !matches!(mcp.state, McpState::Configured | McpState::NotRequired),
            can_execute: true,
            conflict: None,
            context: context.clone(),
            runtime_epoch: self.epoch.clone(),
        };
        let _lock = self.repository.cache_lock()?;
        self.repository.cached_key(key.as_str())?;
        if std::fs::symlink_metadata(&plan.target.path).is_ok() {
            match super::journal::managed_install(&self.repository, &plan)? {
                Some(d) => plan.expected_fingerprint = Some(d.fingerprint),
                None => {
                    plan.can_execute = false;
                    plan.conflict =
                        Some("The destination contains unmanaged files or local edits.".into());
                }
            }
        } else if super::journal::tracked_destination(&self.repository, &plan)? {
            plan.can_execute = false;
            plan.conflict =
                Some("The recorded destination is missing; review its Library record.".into());
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
                self.repository.save_options(
                    &plan.id,
                    &LifecycleOptions {
                        operation_context: Some(OperationContext::Cached { context: c }),
                        ..Default::default()
                    },
                )?;
                Ok(plan)
            })
            .await
    }
    pub(crate) async fn run_cached(
        &self,
        plan: &PreparedOperation,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), SdsError> {
        let context = match self.repository.options(plan)?.operation_context {
            Some(OperationContext::Cached { context }) => context,
            _ => return Err(review()),
        };
        self.validate_cached_context(&context).await?;
        check_cancel(&cancel)?;
        self.progress(
            &plan.id,
            OperationState::Validating,
            "verifying-cached-copy",
        )?;
        let (repo, key) = (self.repository.clone(), context.release_key.clone());
        let cached = blocking(move || repo.cached_key(&key)).await?;
        if serde_json::to_value(&cached.release).map_err(|_| review())?
            != serde_json::to_value(&plan.release).map_err(|_| review())?
        {
            return Err(review());
        }
        if plan.expected_fingerprint.is_some() {
            let (repo, targets, frozen) =
                (self.repository.clone(), self.targets.clone(), plan.clone());
            return self
                .session
                .commit_cached_for(&plan.context, move || {
                    check_cancel(&cancel)?;
                    if !repo.has_cache_grant(&context)? {
                        return Err(SdsError::new(
                            "CACHE_NOT_AUTHORIZED",
                            "The prior download grant changed before commit.",
                            false,
                        ));
                    }
                    targets.with_current_target(&frozen.target, || {
                        check_mcp(&targets, &frozen)?;
                        super::journal::confirm_existing(&repo, &frozen)
                    })
                })
                .await;
        }
        let (repo, frozen) = (self.repository.clone(), plan.clone());
        let intent = blocking(move || {
            super::journal::CommitIntent::stage_install(&repo, &frozen, &cached.artifact)
        })
        .await?;
        let (repo, frozen) = (self.repository.clone(), plan.clone());
        let ready = blocking(move || {
            super::journal::PreparedInstallCommit::prepare(&repo, &frozen, &intent)
        })
        .await?;
        let (repo, targets, frozen) = (self.repository.clone(), self.targets.clone(), plan.clone());
        self.session
            .commit_cached_for(&plan.context, move || {
                check_cancel(&cancel)?;
                if !repo.has_cache_grant(&context)? {
                    return Err(SdsError::new(
                        "CACHE_NOT_AUTHORIZED",
                        "The prior download grant changed before commit.",
                        false,
                    ));
                }
                targets.with_current_target(&frozen.target, || {
                    check_mcp(&targets, &frozen)?;
                    ready.publish(&repo)
                })
            })
            .await
    }
}
