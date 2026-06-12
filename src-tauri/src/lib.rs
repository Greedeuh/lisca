mod clipboard;
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
            let tts = Arc::new(tts_manager);
            app.manage(tts.clone());

            // Auto-load default model if present
            tauri::async_runtime::spawn(async move {
                tts.auto_load().await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hotkey::hotkey_set,
            hotkey::hotkey_get,
            tts::tts_speak,
            tts::tts_stop,
            tts::tts_load_model,
            tts::tts_model_loaded,
            clipboard::read_selected_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
