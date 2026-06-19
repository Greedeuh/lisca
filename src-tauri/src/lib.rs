// Lisca — Tauri v2 desktop app for text-to-speech.
// This crate re-exports all domain modules for the frontend and Tauri IPC layer.

pub mod catalog;
pub mod clipboard;
pub mod commands;
pub mod errors;
pub mod hotkey;
pub mod models;
pub mod overlay;
pub mod persist;
pub mod queue;
pub mod speech_player;
pub mod tray;
pub mod transcriber;
pub mod voice_prefs;

use catalog::VoiceCatalog;
use commands::AppState;
use queue::{Queue, QueueControllable};
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use voice_prefs::VoiceMapping;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let app_data_dir = match app.path().app_data_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    log::error!("Failed to resolve app data dir: {e}");
                    return Err(e.into());
                }
            };
            if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                log::warn!("Failed to create app data dir: {e}");
            }
            log::info!("App data dir: {}", app_data_dir.display());

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

            let hotkey_config = crate::hotkey::load_hotkey(&app_data_dir.join("hotkey.txt"));

            let state = AppState {
                catalog,
                queue: Mutex::new(queue),
                voice_mapping: Mutex::new(voice_mapping),
                app_data_dir,
            };
            app.manage(state);

            // Create overlay window upfront (needed when main window is hidden)
            crate::overlay::create_overlay(app.handle())?;

            // Create main window programmatically (config-created windows
            // are not reliably findable via get_webview_window)
            let main_window = WebviewWindowBuilder::new(app.handle(), "main", WebviewUrl::App("index.html".into()))
                .title("Lisca")
                .inner_size(800.0, 600.0)
                .build()
                .map_err(|e| e.to_string())?;

            // Intercept window close: always hide to tray (quit via tray menu only)
            {
                let win = main_window.clone();
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                        return;
                    };

                    let state = app_handle.state::<AppState>();
                    let (has_items, show_overlay) = match state.queue.lock() {
                        Ok(queue) => (!queue.is_empty(), queue.config().show_overlay),
                        Err(e) => {
                            log::error!("Failed to lock queue: {e}");
                            return;
                        }
                    };

                    api.prevent_close();
                    if let Err(e) = win.hide() {
                        log::warn!("Failed to hide main window: {e}");
                    }
                    if has_items && show_overlay {
                        if let Err(e) = overlay::show_overlay(&app_handle) {
                            log::warn!("Failed to show overlay: {e}");
                        }
                    }
                });
            }

            // Create system tray
            tray::create_tray(app.handle())?;

            // Register global shortcut if configured
            use tauri_plugin_clipboard_manager::ClipboardExt;
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            if let Some(config) = hotkey_config {
                let shortcut_str = config.to_string_repr();
                log::info!("Registering global shortcut: {shortcut_str}");
                let app_handle = app.handle().clone();
                if let Ok(shortcut) = shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                    if let Err(e) = app_handle.global_shortcut().on_shortcut(
                        shortcut,
                        move |_app, _shortcut, event| {
                            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                                if let Ok(text) = _app.clipboard().read_text() {
                                    let text = text.to_string();
                                    if !text.is_empty() {
                                        let state = _app.state::<commands::AppState>();
                                        let result = state
                                            .queue
                                            .lock()
                                            .map(|mut q| q.add_text(text));
                                        match result {
                                            Ok(Ok(_)) => {}
                                            Ok(Err(e)) => {
                                                log::error!("Failed to add text to queue: {e}");
                                            }
                                            Err(e) => {
                                                log::error!("Failed to lock queue: {e}");
                                            }
                                        }
                                    }
                                } else {
                                    log::warn!("Failed to read clipboard");
                                }
                            }
                        },
                    ) {
                        log::error!("Failed to register global shortcut: {e}");
                    }
                } else {
                    log::error!("Failed to parse shortcut string: {shortcut_str}");
                }
            }

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
            commands::create_overlay_window,
            commands::show_overlay_window,
            commands::hide_overlay_window,
            commands::toggle_overlay_window,
            commands::queue_toggle_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
