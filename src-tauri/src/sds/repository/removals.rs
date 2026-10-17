use super::{
    cache_key, operation_error, parse_result, repository_error, validate_operation, Repository,
};
use crate::sds::{
    contracts::{OperationResult, OperationState, PreparedOperation},
    error::SdsError,
    journal::{CommitIntent, Deployment},
    packages::Manifest,
};
use rusqlite::{params, OptionalExtension, Transaction};

impl Repository {
    pub(crate) fn removal_manifest(
        &self,
        plan: &PreparedOperation,
        intent: &CommitIntent,
    ) -> Result<Manifest, SdsError> {
        intent.validate(plan)?;
        if plan.kind != "remove" {
            return Err(operation_error());
        }
        let stored = self
            .stored(
                &plan.context.profile_id,
                &plan.release.skill_id,
                &plan.release.id,
            )?
            .ok_or_else(operation_error)?;
        if stored.fingerprint != intent.fingerprint
            || serde_json::to_value(stored.release).map_err(|_| operation_error())?
                != serde_json::to_value(&plan.release).map_err(|_| operation_error())?
        {
            return Err(operation_error());
        }
        Ok(stored.manifest)
    }
    pub(crate) fn finish_remove(
        &self,
        plan: &PreparedOperation,
        intent: Option<&CommitIntent>,
        deployment: &Deployment,
    ) -> Result<(), SdsError> {
        validate_operation(plan)?;
        if plan.kind != "remove"
            || plan.deployment_id.as_deref() != Some(&deployment.id)
            || deployment.target != plan.target
            || deployment.release_key
                != cache_key(
                    &plan.context.profile_id,
                    &plan.release.skill_id,
                    &plan.release.id,
                )?
        {
            return Err(operation_error());
        }
        if let Some(intent) = intent {
            intent.validate(plan)?;
            if intent.fingerprint != deployment.fingerprint
                || intent.stage_identity != deployment.identity
                || intent.root_identity != deployment.root_identity
            {
                return Err(operation_error());
            }
        } else if plan.expected_fingerprint.is_some() {
            return Err(operation_error());
        }
        let result = OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Completed,
            stage: if intent.is_some() {
                "removed"
            } else {
                "record-removed"
            }
            .into(),
            error: None,
        };
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        let previous = check_operation(&tx, plan, intent)?;
        if previous.state.terminal()
            || (intent.is_some()
                && !matches!(
                    previous.state,
                    OperationState::Committing | OperationState::RecoveryRequired
                ))
        {
            return Err(operation_error());
        }
        let body:String=tx.query_row("SELECT CASE WHEN length(CAST(target_json AS BLOB))<=268435456 THEN target_json ELSE NULL END FROM deployments WHERE deployment_id=?1",[&deployment.id],|row|row.get(0)).map_err(|_|operation_error())?;
        if serde_json::from_str::<serde_json::Value>(&body).map_err(|_| operation_error())?
            != serde_json::to_value(deployment).map_err(|_| operation_error())?
        {
            return Err(operation_error());
        }
        let removed = tx
            .execute(
                "DELETE FROM deployments WHERE deployment_id=?1",
                [&deployment.id],
            )
            .map_err(|_| repository_error())?;
        if removed != 1 {
            return Err(operation_error());
        }
        // Keep a durable remove intent until its owned quarantine is safely cleaned.
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
    pub(crate) fn finish_remove_cleanup(
        &self,
        plan: &PreparedOperation,
        intent: &CommitIntent,
        warning: Option<SdsError>,
    ) -> Result<OperationResult, SdsError> {
        if plan.kind != "remove" {
            return Err(operation_error());
        }
        intent.validate(plan)?;
        let mut connection = self.connection.lock().map_err(|_| repository_error())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| repository_error())?;
        if check_operation(&tx, plan, Some(intent))?.state != OperationState::Completed {
            return Err(operation_error());
        }
        let remaining: Option<String> = tx
            .query_row(
                "SELECT deployment_id FROM deployments WHERE deployment_id=?1",
                [plan.deployment_id.as_deref().ok_or_else(operation_error)?],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| repository_error())?;
        if remaining.is_some() {
            return Err(operation_error());
        }
        let kept = warning.is_some();
        let result = OperationResult {
            operation_id: plan.id.clone(),
            state: OperationState::Completed,
            stage: if kept {
                "removed-files-kept"
            } else {
                "removed"
            }
            .into(),
            error: warning,
        };
        tx.execute("UPDATE operations SET result_json=?1,intent_json=CASE WHEN ?2 THEN intent_json ELSE NULL END,updated_at=?3 WHERE operation_id=?4",params![serde_json::to_string(&result).map_err(|_|operation_error())?,kept,chrono::Utc::now().to_rfc3339(),plan.id]).map_err(|_|repository_error())?;
        tx.commit().map_err(|_| repository_error())?;
        Ok(result)
    }
}
fn check_operation(
    tx: &Transaction<'_>,
    plan: &PreparedOperation,
    intent: Option<&CommitIntent>,
) -> Result<OperationResult, SdsError> {
    let (frozen,result,saved):(String,String,Option<String>)=tx.query_row("SELECT CASE WHEN length(CAST(frozen_json AS BLOB))<=2097152 THEN frozen_json ELSE NULL END,CASE WHEN length(CAST(result_json AS BLOB))<=65536 THEN result_json ELSE NULL END,CASE WHEN intent_json IS NULL THEN NULL WHEN length(CAST(intent_json AS BLOB))<=65536 THEN intent_json ELSE '' END FROM operations WHERE operation_id=?1",[&plan.id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).map_err(|_|operation_error())?;
    if serde_json::from_str::<serde_json::Value>(&frozen).map_err(|_| operation_error())?
        != serde_json::to_value(plan).map_err(|_| operation_error())?
    {
        return Err(operation_error());
    }
    let expected = intent
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| operation_error())?;
    let saved = saved
        .map(|value| serde_json::from_str::<serde_json::Value>(&value))
        .transpose()
        .map_err(|_| operation_error())?;
    if expected != saved {
        return Err(operation_error());
    }
    parse_result(&result, &plan.id)
}
