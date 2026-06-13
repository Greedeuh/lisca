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
            let app_data_dir = app.path().app_data_dir().expect("no app data dir");
            let resource_dir = app.path().resource_dir().expect("no resource dir");
            let tts = Arc::new(TtsManager::new(app_data_dir.clone(), resource_dir));
            app.manage(tts.clone());
            tts.preload();

            // Initialize PiperModelManager
            let mut piper_manager = tts::piper_models::PiperModelManager::new(&app_data_dir);
            piper_manager.load_cached_voices();
            let piper_manager = Arc::new(tokio::sync::Mutex::new(piper_manager));
            app.manage(piper_manager);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hotkey::hotkey_set,
            hotkey::hotkey_get,
            tts::tts_speak,
            tts::tts_stop,
            tts::tts_get_config,
            tts::tts_set_config,
            tts::tts_open_resource_dir,
            tts::piper_fetch_voices,
            tts::piper_download_model,
            tts::piper_list_installed,
            tts::piper_delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
