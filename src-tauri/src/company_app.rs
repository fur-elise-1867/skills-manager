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
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let config = CompanyConfig::from_environment()?;
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
            company_operation_cancel
        ])
        .run(tauri::generate_context!())
        .expect("Company Skills Manager startup failed");
}
