mod hotkey;
mod tts;

use std::sync::Arc;
use tauri::Manager;
use tts::TtsManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let tts = Arc::new(TtsManager::new());
            app.manage(tts.clone());
            tts.preload();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hotkey::hotkey_set,
            hotkey::hotkey_get,
            tts::tts_speak,
            tts::tts_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
