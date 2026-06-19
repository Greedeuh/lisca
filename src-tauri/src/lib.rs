// Lisca — Tauri v2 desktop app for text-to-speech.
// This crate re-exports all domain modules for the frontend and Tauri IPC layer.

pub mod catalog;
pub mod clipboard;
pub mod commands;
pub mod hotkey;
pub mod models;
pub mod persist;
pub mod queue;
pub mod speech_player;
pub mod transcriber;
pub mod voice_prefs;

use catalog::VoiceCatalog;
use commands::AppState;
use queue::Queue;
use std::sync::Mutex;
use tauri::Manager;
use voice_prefs::VoiceMapping;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();

            let piper_models_dir = app_data_dir.join("piper_models");
            let kokoro_models_dir = app_data_dir.join("kokoro");
            let catalog = VoiceCatalog::new(piper_models_dir, kokoro_models_dir);

            let queue_config_path = app_data_dir.join("queue_config.json");
            let queue_config = Queue::load_config(&queue_config_path);
            let queue = Queue::new()
                .with_config(queue_config)
                .with_config_path(queue_config_path);

            let voice_mapping_path = app_data_dir.join("voice_mapping.json");
            let voice_mapping = VoiceMapping::load(&voice_mapping_path);

            let state = AppState {
                catalog,
                queue: Mutex::new(queue),
                voice_mapping: Mutex::new(voice_mapping),
                app_data_dir,
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_catalog_voices,
            commands::list_installed_voices,
            commands::install_voice,
            commands::uninstall_voice,
            commands::queue_state,
            commands::queue_add,
            commands::queue_remove,
            commands::queue_move,
            commands::queue_clear,
            commands::queue_toggle_auto_read,
            commands::get_voice_preference,
            commands::set_voice_preference,
            commands::set_fallback_voice,
            commands::get_hotkey,
            commands::save_hotkey_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
