use crate::{
    commands::company::*,
    core::company::{
        config::CompanyConfig, installation_service::CompanyService, session::WindowsCredentials,
    },
};
use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let resource = app
                .path()
                .resolve(
                    "resources/company-store.json",
                    tauri::path::BaseDirectory::Resource,
                )
                .ok();
            let config = CompanyConfig::from_environment_with_resource(resource.as_deref())?;
            let credentials = Arc::new(WindowsCredentials::new(&config));
            app.manage(Arc::new(CompanyService::new(config, credentials)?));
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            company_environment,
            company_session,
            company_login,
            company_logout,
            company_store_search,
            company_store_detail,
            company_installations_list,
            company_operation_prepare,
            company_operation_confirm,
            company_operation_status,
            company_operation_cancel,
            company_projects_list,
            company_project_add,
            company_project_remove,
            company_project_install_prepare,
            company_installation_refresh,
            company_installation_open,
            company_updates_check,
            company_update_prepare,
            company_update_apply,
            company_backups_list,
            company_restore_prepare,
            company_local_import_prepare,
            company_storage_status,
            company_backup_delete,
            company_package_cleanup,
            company_remove_prepare,
            company_lifecycle_apply,
            company_operation_open_preview
        ])
        .run(tauri::generate_context!())
        .expect("Company Skills Manager startup failed");
}
