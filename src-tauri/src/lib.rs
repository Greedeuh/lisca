mod clipboard;
mod hotkey;
mod overlay;
mod persist;
mod tts;

use std::sync::Arc;
use tauri::Manager;
use tts::TtsManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(setup_app)
        .invoke_handler(register_commands())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir().expect("no app data dir");
    let resource_dir = app.path().resource_dir().expect("no resource dir");

    setup_tts(app, &app_data_dir, resource_dir);
    setup_piper_models(app, &app_data_dir);
    overlay::create_overlay(app.handle());
    setup_close_handler(app);
    setup_tray(app)?;

    Ok(())
}

fn setup_tts(app: &mut tauri::App, app_data_dir: &std::path::Path, resource_dir: std::path::PathBuf) {
    let tts = Arc::new(TtsManager::new(
        app_data_dir.to_path_buf(),
        resource_dir,
        app.handle().clone(),
    ));
    app.manage(tts.clone());
    tts.preload();
}

fn setup_piper_models(app: &mut tauri::App, app_data_dir: &std::path::Path) {
    let mut manager = tts::piper::PiperModelManager::new(app_data_dir);
    manager.load_cached_voices();
    let models = manager.list_installed();
    app.manage(Arc::new(tokio::sync::Mutex::new(manager)));

    let tts = app.state::<Arc<TtsManager>>();
    tts.refresh_installed_models(models);
}

fn setup_close_handler(app: &mut tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let win = window.clone();
    let app_handle = app.handle().clone();
    window.on_window_event(move |event| {
        let tauri::WindowEvent::CloseRequested { api, .. } = event else {
            return;
        };

        api.prevent_close();
        let _ = win.hide();

        let app_data_dir = app_handle.path().app_data_dir().expect("no app data dir");
        let queue = tts::queue::load_queue(&app_data_dir);
        if tts::queue::load_queue_config(&app_data_dir).show_overlay && !queue.is_empty() {
            overlay::show_overlay(&app_handle);
        }
    });
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("no default window icon — check tauri.conf.json bundle.icon")
                .clone(),
        )
        .tooltip("Lisca")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
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
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    overlay::hide_overlay(app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn register_commands() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![
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
        tts::commands::tts_get_voice_mapping,
        tts::commands::tts_set_voice_mapping,
    ]
}
