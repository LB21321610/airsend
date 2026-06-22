pub mod app_state;
pub mod commands;
pub mod discovery;
pub mod models;
pub mod platform;
pub mod protocol;
pub mod security;
pub mod storage;
pub mod transfer;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let app_data_dir = app.path().app_data_dir()?;
            let state = app_state::AppState::load(app_data_dir.join("airsend.json"))
                .map_err(|err| Box::<dyn std::error::Error>::from(err))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_snapshot,
            commands::get_local_device,
            commands::list_devices,
            commands::pairing_code,
            commands::trust_peer,
            commands::send_files,
            commands::accept_transfer,
            commands::reject_transfer,
            commands::set_save_directory,
            commands::set_auto_accept
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
