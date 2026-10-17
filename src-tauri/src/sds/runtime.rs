use super::profile::SdsProfile;
use tauri::Manager;

pub const COMMANDS: &[&str] = &[
    "runtime_profile",
    "sds_get_connection",
    "sds_set_connection",
    "sds_session_status",
    "sds_login",
    "sds_logout",
    "sds_catalog_taxonomy",
    "sds_catalog_search",
    "sds_skill_detail",
    "sds_get_target_settings",
    "sds_set_target_override",
    "sds_library",
    "sds_library_preview",
    "sds_prepare_install",
    "sds_prepare_remove",
    "sds_execute",
    "sds_cancel",
    "sds_operation_status",
    "sds_reveal_deployment",
    "sds_reveal_logs",
];

/// A mismatched binary/config pair exposes only its identity, never storage.
pub fn run_mismatch(context: tauri::Context<tauri::Wry>) {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![super::runtime_profile])
        .run(context)
        .expect("Cannot show desktop profile mismatch");
}

pub fn run() {
    let profile = SdsProfile::resolve().expect("Invalid SDS build profile");
    std::fs::create_dir_all(profile.data_root()).expect("Cannot create SDS data directory");
    let logs = profile.data_root().join("logs");
    let services = super::commands::SdsServices::from_profile(&profile)
        .expect("Cannot initialize SDS services");
    tauri::Builder::default()
        .manage(profile)
        .manage(services)
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Folder {
                        path: logs,
                        file_name: Some("sds".into()),
                    },
                )])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            super::runtime_profile,
            super::commands::sds_get_connection,
            super::commands::sds_set_connection,
            super::commands::sds_session_status,
            super::commands::sds_login,
            super::commands::sds_logout,
            super::commands::sds_catalog_taxonomy,
            super::commands::sds_catalog_search,
            super::commands::sds_skill_detail,
            super::commands::sds_get_target_settings,
            super::commands::sds_set_target_override,
            super::commands::sds_library,
            super::commands::sds_library_preview,
            super::commands::sds_prepare_install,
            super::commands::sds_prepare_remove,
            super::commands::sds_execute,
            super::commands::sds_cancel,
            super::commands::sds_operation_status,
            super::commands::sds_reveal_deployment,
            super::commands::sds_reveal_logs
        ])
        .run(tauri::generate_context!())
        .expect("Error running SDS desktop");
}
