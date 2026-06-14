mod hotkey;
mod overlay;
mod persist;
mod tts;

use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
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
            let tts = Arc::new(TtsManager::new(app_data_dir.clone(), resource_dir, app.handle().clone()));
            app.manage(tts.clone());
            tts.preload();

            // Initialize PiperModelManager
            let mut piper_manager = tts::piper_models::PiperModelManager::new(&app_data_dir);
            piper_manager.load_cached_voices();
            let piper_manager = Arc::new(tokio::sync::Mutex::new(piper_manager));
            app.manage(piper_manager);

            overlay::create_overlay(app.handle());

            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let window = match app.get_webview_window("main") {
                Some(w) => w,
                None => return Ok(()),
            };

            let win = window.clone();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win.hide();
                    let app_data_dir = app_handle.path().app_data_dir().expect("no app data dir");
                    let queue_config = tts::queue::load_queue_config(&app_data_dir);
                    let queue = tts::queue::load_queue(&app_data_dir);
                    if queue_config.show_overlay && !queue.is_empty() {
                        overlay::show_overlay(&app_handle);
                    }
                }
            });
            drop(window);

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon()
                    .expect("no default window icon — check tauri.conf.json bundle.icon")
                    .clone())
                .tooltip("Lisca")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        overlay::hide_overlay(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        overlay::hide_overlay(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hotkey::hotkey_set,
            hotkey::hotkey_get,
            tts::commands::tts_speak,
            tts::commands::tts_stop,
            tts::commands::tts_get_config,
            tts::commands::tts_set_config,
            tts::commands::tts_open_resource_dir,
            tts::commands::piper_fetch_voices,
            tts::commands::piper_download_model,
            tts::commands::piper_list_installed,
            tts::commands::piper_delete_model,
            tts::commands::tts_queue_add,
            tts::commands::tts_queue_remove,
            tts::commands::tts_queue_move,
            tts::commands::tts_queue_clear,
            tts::commands::tts_queue_state,
            tts::commands::tts_pause,
            tts::commands::tts_resume,
            tts::commands::tts_set_queue_config,
            tts::commands::tts_get_queue_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
