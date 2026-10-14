use crate::core::company::{config::Environment, installation_service::CompanyService, types::*};
use std::sync::Arc;
use tauri::State;

type AppState<'a> = State<'a, Arc<CompanyService>>;
#[tauri::command]
pub async fn company_storage_status(state: AppState<'_>) -> CompanyResult<StorageSummary> {
    blocking(state, |app| app.storage_status()).await
}
#[tauri::command]
pub async fn company_backup_delete(state: AppState<'_>, id: String) -> CompanyResult<()> {
    blocking(state, move |app| app.delete_backup(&id)).await
}
#[tauri::command]
pub async fn company_package_cleanup(
    state: AppState<'_>,
    hashes: Vec<String>,
) -> CompanyResult<Vec<String>> {
    blocking(state, move |app| app.cleanup_packages(&hashes)).await
}
#[tauri::command]
pub async fn company_local_import_prepare(
    state: AppState<'_>,
    path: String,
    slug: String,
    project_id: Option<String>,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| {
        app.prepare_local_import(std::path::Path::new(&path), &slug, project_id.as_deref())
    })
    .await
}
#[tauri::command]
pub async fn company_restore_prepare(
    state: AppState<'_>,
    backup_id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.prepare_restore(&backup_id)).await
}
#[tauri::command]
pub async fn company_remove_prepare(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.prepare_remove(&id)).await
}
#[tauri::command]
pub async fn company_lifecycle_apply(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.apply_lifecycle(&id)).await
}
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

#[tauri::command]
pub async fn company_projects_list(state: AppState<'_>) -> CompanyResult<Vec<Project>> {
    blocking(state, |app| app.projects()).await
}
#[tauri::command]
pub async fn company_project_add(state: AppState<'_>, path: String) -> CompanyResult<Project> {
    blocking(state, move |app| {
        app.add_project(std::path::Path::new(&path))
    })
    .await
}
#[tauri::command]
pub async fn company_project_remove(state: AppState<'_>, id: String) -> CompanyResult<()> {
    blocking(state, move |app| app.remove_project(&id)).await
}
#[tauri::command]
pub async fn company_project_install_prepare(
    state: AppState<'_>,
    id: String,
    project_id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| {
        app.prepare_project_install(&id, &project_id)
    })
    .await
}
#[tauri::command]
pub async fn company_installation_refresh(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<Installation> {
    blocking(state, move |app| app.refresh_installation(&id)).await
}
#[tauri::command]
pub async fn company_installation_open(
    state: AppState<'_>,
    id: String,
    kind: String,
) -> CompanyResult<()> {
    blocking(state, move |app| {
        let path = app.installation_open_path(&id, &kind)?;
        tauri_plugin_opener::open_path(path, None::<&str>).map_err(|_| {
            CompanyError::new(
                "OPEN_FAILED",
                "Không mở được file. Dùng Mở thư mục rồi chọn ứng dụng phù hợp trên Windows.",
            )
        })
    })
    .await
}
#[tauri::command]
pub async fn company_updates_check(
    state: AppState<'_>,
    ids: Vec<String>,
) -> CompanyResult<Vec<Installation>> {
    blocking(state, move |app| app.check_updates(&ids)).await
}
#[tauri::command]
pub async fn company_update_prepare(
    state: AppState<'_>,
    id: String,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.prepare_update(&id)).await
}
#[tauri::command]
pub async fn company_update_apply(
    state: AppState<'_>,
    id: String,
    replace_customized: bool,
) -> CompanyResult<OperationView> {
    blocking(state, move |app| app.apply_update(&id, replace_customized)).await
}
#[tauri::command]
pub async fn company_backups_list(state: AppState<'_>, id: String) -> CompanyResult<Vec<Backup>> {
    blocking(state, move |app| app.backups(&id)).await
}
#[tauri::command]
pub async fn company_operation_open_preview(
    state: AppState<'_>,
    id: String,
    side: String,
    path: String,
) -> CompanyResult<()> {
    blocking(state, move |app| {
        let path = app.preview_open_path(&id, &side, &path)?;
        // Text preview always uses an editor, including source scripts whose OS association may run them.
        let editor = if path.is_file() {
            Some("notepad.exe")
        } else {
            None
        };
        tauri_plugin_opener::open_path(path, editor).map_err(|_| {
            CompanyError::new(
                "OPEN_FAILED",
                "Không mở được preview. Chọn mở thư mục để xem bằng công cụ trên máy.",
            )
        })
    })
    .await
}
