use super::profile::SdsProfile;
use tauri::Manager;

pub const COMMANDS: &[&str] = &["runtime_profile"];

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
    tauri::Builder::default()
        .manage(profile)
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
        .invoke_handler(tauri::generate_handler![super::runtime_profile])
        .run(tauri::generate_context!())
        .expect("Error running SDS desktop");
}
