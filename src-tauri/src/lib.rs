mod clipboard;
mod commands;
mod hotkey;
mod tts;

use std::sync::Arc;
use tauri::Manager;
use tts::TtsManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let tts_manager =
                TtsManager::new(app.handle().clone()).map_err(|e| format!("TTS init: {}", e))?;
            app.manage(Arc::new(tts_manager));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_hotkey,
            commands::load_hotkey,
            commands::update_tts_config,
            commands::speak_text,
            commands::stop_speaking,
            commands::list_voices,
            commands::read_selected_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
