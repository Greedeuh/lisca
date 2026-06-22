// Lisca — Tauri v2 desktop app for text-to-speech.
// This crate re-exports all domain modules for the frontend and Tauri IPC layer.

pub mod actors;
pub mod app_paths;
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

use actors::AppActors;
use actors::queue_actor::QueueActor;
use actors::speech_player_actor::SpeechPlayerActor;
use actors::transcriber_actor::TranscriberActor;
use actix::Actor;
use app_paths::AppPaths;
use catalog::VoiceCatalog;
use commands::AppState;
use models::{KokoroFactory, ModelPool, PiperFactory};
use queue::Queue;
use std::sync::Arc;
use tauri::{Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use transcriber::UnifiedFactory;
use voice_prefs::VoiceMapping;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let (handle_tx, handle_rx) = std::sync::mpsc::sync_channel::<tauri::AppHandle>(1);
    let (actors_tx, actors_rx) = std::sync::mpsc::sync_channel::<AppActors>(1);

    spawn_actix_system(handle_rx, actors_tx);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| setup_app(app, handle_tx, actors_rx))
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
            commands::player_state,
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
            commands::playback_pause,
            commands::playback_resume,
            commands::playback_stop,
            commands::playback_skip,
            commands::playback_restart,
            commands::playback_replay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn spawn_actix_system(
    handle_rx: std::sync::mpsc::Receiver<tauri::AppHandle>,
    actors_tx: std::sync::mpsc::SyncSender<AppActors>,
) {
    std::thread::spawn(move || {
        let sys = actix::System::new();
        let _ = sys.block_on(async move {
            let app_handle = handle_rx.recv().expect("failed to receive AppHandle");
            let paths = AppPaths::resolve(&app_handle);

            let queue_config_path = paths.app_data_dir.join("queue_config.json");
            let queue_config = Queue::load_config(&queue_config_path);
            let queue = Queue::new()
                .with_config(queue_config)
                .with_config_path(queue_config_path);

            let player_config_path = paths.app_data_dir.join("player_config.json");
            let player_config = SpeechPlayerActor::load_config(&player_config_path);

            let voice_mapping_path = paths.app_data_dir.join("voice_mapping.json");
            let voice_mapping = Arc::new(tokio::sync::Mutex::new(
                VoiceMapping::load(&voice_mapping_path),
            ));

            let model_pool = Arc::new(tokio::sync::Mutex::new(ModelPool::new(
                4,
                Some(std::time::Duration::from_secs(300)),
            )));

            let piper_factory: Arc<dyn models::ModelFactory> =
                Arc::new(PiperFactory::new(paths.piper_models_dir.clone(), paths.app_data_dir.clone()));
            let shared_engine_path = paths.kokoro_models_dir.join("kokoro_engine.onnx");
            let kokoro_factory: Arc<dyn models::ModelFactory> =
                Arc::new(KokoroFactory::new(paths.kokoro_models_dir.clone(), shared_engine_path, paths.resource_dir.clone()));
            let unified_factory = Arc::new(UnifiedFactory::new(piper_factory, kokoro_factory));

            let catalog = Arc::new(VoiceCatalog::new(
                paths.piper_models_dir.clone(),
                paths.kokoro_models_dir.clone(),
                &paths.resource_dir,
            ));

            let queue_actor = QueueActor::new(queue, app_handle.clone()).start();
            let transcriber_actor = TranscriberActor::new(
                queue_actor.clone(),
                model_pool.clone(),
                unified_factory.clone(),
                voice_mapping.clone(),
                catalog.clone(),
                app_handle.clone(),
            )
            .start();
            let speech_player_actor = SpeechPlayerActor::new(
                queue_actor.clone(),
                app_handle.clone(),
                player_config.auto_read,
            )
            .with_config_path(player_config_path)
            .start();

            queue_actor.do_send(actors::messages::SetPlayerAddr {
                addr: speech_player_actor.clone(),
            });

            queue_actor.do_send(actors::messages::SetTranscriberAddr {
                addr: transcriber_actor.clone(),
            });

            actors_tx
                .send(AppActors {
                    queue: queue_actor,
                    player: speech_player_actor,
                    voice_mapping,
                })
                .expect("failed to send actors");

            tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)).await;
            Ok::<(), ()>(())
        });
        sys.run().unwrap();
    });
}

fn setup_app(
    app: &mut tauri::App,
    handle_tx: std::sync::mpsc::SyncSender<tauri::AppHandle>,
    actors_rx: std::sync::mpsc::Receiver<AppActors>,
) -> Result<(), Box<dyn std::error::Error>> {
    let paths = match app.path().app_data_dir() {
        Ok(app_data_dir) => {
            if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                log::warn!("Failed to create app data dir: {e}");
            }
            log::info!("App data dir: {}", app_data_dir.display());

            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|e| {
                    log::error!("Failed to resolve resource dir: {e}");
                    app_data_dir.clone()
                });

            AppPaths {
                piper_models_dir: app_data_dir.join("piper_models"),
                kokoro_models_dir: app_data_dir.join("kokoro"),
                resource_dir,
                app_data_dir,
            }
        }
        Err(e) => {
            log::error!("Failed to resolve app data dir: {e}");
            return Err(e.into());
        }
    };

    let catalog = VoiceCatalog::new(
        paths.piper_models_dir.clone(),
        paths.kokoro_models_dir.clone(),
        &paths.resource_dir,
    );

    handle_tx
        .send(app.handle().clone())
        .expect("failed to send AppHandle to actix thread");

    let actors = actors_rx.recv().expect("failed to receive actors from actix thread");

    let hotkey_config = crate::hotkey::load_hotkey(&paths.app_data_dir.join("hotkey.txt"));

    let model_pool = Arc::new(tokio::sync::Mutex::new(ModelPool::new(
        4,
        Some(std::time::Duration::from_secs(300)),
    )));

    let state = AppState {
        catalog,
        voice_mapping: actors.voice_mapping.clone(),
        app_data_dir: paths.app_data_dir,
        model_pool: model_pool.clone(),
    };

    app.manage(actors);
    app.manage(state);

    setup_overlay_listener(app);
    spawn_model_eviction(model_pool);
    crate::overlay::create_overlay(app.handle())?;
    setup_main_window(app)?;
    tray::create_tray(app.handle())?;
    register_global_shortcut(app, hotkey_config);

    Ok(())
}

fn setup_overlay_listener(app: &tauri::App) {
    let handle = app.handle().clone();
    app.listen("item_added", move |_| {
        let h = handle.clone();
        tauri::async_runtime::spawn(async move {
            let main_hidden = h
                .get_webview_window("main")
                .map(|w| !w.is_visible().unwrap_or(true))
                .unwrap_or(true);
            if main_hidden {
                if let Err(e) = crate::overlay::show_overlay(&h) {
                    log::warn!("Failed to show overlay: {e}");
                }
            }
        });
    });
}

fn spawn_model_eviction(model_pool: Arc<tokio::sync::Mutex<ModelPool>>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut pool = model_pool.lock().await;
            pool.evict_expired();
        }
    });
}

fn setup_main_window(app: &tauri::App) -> Result<tauri::WebviewWindow, String> {
    let main_window = WebviewWindowBuilder::new(
        app.handle(),
        "main",
        WebviewUrl::App("index.html".into()),
    )
    .title("Lisca")
    .inner_size(800.0, 600.0)
    .build()
    .map_err(|e| e.to_string())?;

    {
        let win = main_window.clone();
        let app_handle = app.handle().clone();
        main_window.on_window_event(move |event| {
            let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };

            api.prevent_close();
            if let Err(e) = win.hide() {
                log::warn!("Failed to hide main window: {e}");
            }

            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let actors = handle.state::<AppActors>();
                if let Ok(has_playable) = actors.queue.send(actors::messages::HasPlayableItems).await {
                    if has_playable {
                        if let Err(e) = overlay::show_overlay(&handle) {
                            log::warn!("Failed to show overlay: {e}");
                        }
                    }
                }
            });
        });
    }

    Ok(main_window)
}

fn register_global_shortcut(
    app: &tauri::App,
    hotkey_config: Option<crate::hotkey::ShortcutConfig>,
) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let Some(config) = hotkey_config else {
        return;
    };

    let shortcut_str = config.to_string_repr();
    log::info!("Registering global shortcut: {shortcut_str}");
    let app_handle = app.handle().clone();
    let Ok(shortcut) = shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() else {
        log::error!("Failed to parse shortcut string: {shortcut_str}");
        return;
    };

    if let Err(e) = app_handle.global_shortcut().on_shortcut(
        shortcut,
        move |_app, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                if let Some(text) = clipboard::auto_copy_and_read(&_app) {
                    let actors = _app.state::<AppActors>();
                    let queue = actors.queue.clone();
                    tauri::async_runtime::spawn(async move {
                        use actors::messages::AddText;
                        match queue.send(AddText { text }).await {
                            Ok(Ok(id)) => {
                                log::info!("Added item {id} via hotkey");
                            }
                            Ok(Err(e)) => {
                                log::error!("Failed to add text to queue: {e}");
                            }
                            Err(e) => {
                                log::error!("Actor mailbox error: {e}");
                            }
                        }
                    });
                } else {
                    log::warn!("No text selected or clipboard copy failed");
                }
            }
        },
    ) {
        log::error!("Failed to register new shortcut: {e}");
    }
}
