use crate::core::company::{config::Environment, installation_service::CompanyService, types::*};
use std::sync::Arc;
use tauri::State;

type AppState<'a> = State<'a, Arc<CompanyService>>;
async fn blocking<T: Send + 'static>(
    state: AppState<'_>,
    action: impl FnOnce(Arc<CompanyService>) -> CompanyResult<T> + Send + 'static,
) -> CompanyResult<T> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || action(app))
        .await
        .map_err(|_| CompanyError::new("INTERNAL_ERROR", "Không hoàn tất được thao tác native."))?
}
#[tauri::command]
pub async fn company_environment(state: AppState<'_>) -> CompanyResult<Environment> {
    blocking(state, |app| {
        let mut env = app.config.environment();
        env.recovery_warnings = app.recovery_warnings();
        Ok(env)
    })
    .await
}
#[tauri::command]
pub async fn company_session(state: AppState<'_>) -> CompanyResult<SessionView> {
    blocking(state, |app| Ok(app.current_session())).await
}
#[tauri::command]
pub async fn company_login(
    state: AppState<'_>,
    username: String,
    password: String,
) -> CompanyResult<SessionView> {
    blocking(state, move |app| app.login(&username, &password)).await
}
#[tauri::command]
pub async fn company_logout(state: AppState<'_>) -> CompanyResult<()> {
    blocking(state, |app| app.logout()).await
}
#[tauri::command]
pub async fn company_store_search(
    state: AppState<'_>,
    query: String,
    page: u32,
) -> CompanyResult<SkillPage> {
    blocking(state, move |app| app.search(&query, page)).await
}
#[tauri::command]
pub async fn company_store_detail(state: AppState<'_>, id: String) -> CompanyResult<Skill> {
    blocking(state, move |app| app.detail(&id)).await
}
#[tauri::command]
pub async fn company_installations_list(state: AppState<'_>) -> CompanyResult<Vec<Installation>> {
    blocking(state, |app| app.installations()).await
}
#[tauri::command]
pub async fn company_operation_prepare(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.prepare_install(&id)).await
}
#[tauri::command]
pub async fn company_operation_confirm(
    state: AppState<'_>,
    id: String,
    bypass_mcp: bool,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.confirm(&id, bypass_mcp)).await
}
#[tauri::command]
pub async fn company_operation_status(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.status(&id)).await
}
#[tauri::command]
pub async fn company_operation_cancel(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.cancel(&id)).await
}
