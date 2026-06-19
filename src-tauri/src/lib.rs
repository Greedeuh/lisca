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
use models::{KokoroFactory, ModelPool, PiperFactory};
use queue::{Queue, QueueControllable};
use speech_player::PlaybackEvent;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use transcriber::{TranscriptionEvent, UnifiedFactory};
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
            let catalog = VoiceCatalog::new(piper_models_dir.clone(), kokoro_models_dir.clone());

            let queue_config_path = app_data_dir.join("queue_config.json");
            let queue_config = Queue::load_config(&queue_config_path);
            let initial_auto_read = queue_config.auto_read;
            let queue = Arc::new(tokio::sync::Mutex::new(
                Queue::new()
                    .with_config(queue_config)
                    .with_config_path(queue_config_path),
            ));

            let voice_mapping_path = app_data_dir.join("voice_mapping.json");
            let voice_mapping = Arc::new(tokio::sync::Mutex::new(VoiceMapping::load(&voice_mapping_path)));

            let hotkey_config = crate::hotkey::load_hotkey(&app_data_dir.join("hotkey.txt"));

            let model_pool = Arc::new(tokio::sync::Mutex::new(ModelPool::new(
                4,
                Some(std::time::Duration::from_secs(300)),
            )));

            // Create factories
            let piper_factory: Arc<dyn models::ModelFactory> = Arc::new(PiperFactory::new(
                piper_models_dir,
                app_data_dir.clone(),
            ));

            let shared_engine_path = kokoro_models_dir.join("kokoro_engine.onnx");
            let kokoro_factory: Arc<dyn models::ModelFactory> = Arc::new(
                KokoroFactory::new(kokoro_models_dir, shared_engine_path),
            );

            let unified_factory = Arc::new(UnifiedFactory::new(piper_factory, kokoro_factory));

            // Spawn transcriber — shares the same queue and voice_mapping via Arc
            let app_handle_for_transcriber = app.handle().clone();
            let model_pool_for_transcriber = model_pool.clone();
            let voice_mapping_for_transcriber = voice_mapping.clone();
            let unified_factory_for_transcriber = unified_factory.clone();
            let queue_for_transcriber = queue.clone();

            let transcriber_handle = transcriber::spawn_transcriber(
                queue_for_transcriber,
                model_pool_for_transcriber,
                unified_factory_for_transcriber,
                voice_mapping_for_transcriber,
                {
                    // Clone for speech player wake
                    let app_handle_for_speech = app_handle_for_transcriber.clone();
                    move |event| {
                        let app_handle = app_handle_for_speech.clone();
                        match event {
                            TranscriptionEvent::Started { id, text } => {
                                log::debug!("Transcription started for item {id}");
                                let _ = app_handle.emit("transcription_started", (id, text));
                            }
                            TranscriptionEvent::Completed { id } => {
                                log::debug!("Transcription completed for item {id}");
                                let _ = app_handle.emit("transcription_completed", id);
                                let _ = app_handle.emit("queue_updated", ());
                                // Wake speech player to process the new Speech item
                                let state = app_handle.state::<AppState>();
                                let speech_handle = state.speech_player_handle.clone();
                                {
                                    let guard = speech_handle.try_lock();
                                    if let Ok(handle) = guard {
                                        if let Some(ref h) = *handle {
                                            h.wake();
                                        }
                                    }
                                }
                            }
                            TranscriptionEvent::Error { id, error } => {
                                log::error!("Transcription error for item {id}: {error}");
                                let _ = app_handle.emit("transcription_error", (id, error));
                                let _ = app_handle.emit("queue_updated", ());
                            }
                        }
                    }
                },
            );

            // Spawn speech player
            let auto_read = Arc::new(AtomicBool::new(initial_auto_read));
            let queue_for_speech = queue.clone();
            let speech_player_handle = speech_player::spawn_speech_player(
                queue_for_speech,
                auto_read.clone(),
                move |event| match event {
                    PlaybackEvent::Started { id } => {
                        log::debug!("Playback started for item {id}");
                    }
                    PlaybackEvent::ItemCompleted { id } => {
                        log::debug!("Playback completed for item {id}");
                    }
                    _ => {}
                },
            );

            // Create AppState — shares queue and voice_mapping with transcriber
            let state = AppState {
                catalog,
                queue: queue.clone(),
                voice_mapping: voice_mapping.clone(),
                app_data_dir,
                model_pool: model_pool.clone(),
                transcriber_handle: Arc::new(std::sync::Mutex::new(Some(transcriber_handle))),
                speech_player_handle: Arc::new(std::sync::Mutex::new(Some(speech_player_handle))),
                auto_read,
            };
            app.manage(state);

            // Spawn periodic model pool eviction task
            {
                let model_pool = model_pool.clone();
                tauri::async_runtime::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                    loop {
                        interval.tick().await;
                        let mut pool = model_pool.lock().await;
                        pool.evict_expired();
                    }
                });
            }

            // Create overlay window upfront (needed when main window is hidden)
            crate::overlay::create_overlay(app.handle())?;

            // Create main window programmatically
            let main_window = WebviewWindowBuilder::new(
                app.handle(),
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("Lisca")
            .inner_size(800.0, 600.0)
            .build()
            .map_err(|e| e.to_string())?;

            // Intercept window close: always hide to tray
            // Uses try_lock (non-blocking) since we're outside the tokio runtime
            {
                let win = main_window.clone();
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
                        return;
                    };

                    let state = app_handle.state::<AppState>();
                    let (has_items, show_overlay) = match state.queue.try_lock() {
                        Ok(queue) => (!queue.is_empty(), queue.config().show_overlay),
                        Err(_) => (false, true), // lock held, default to hiding
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
                if let Ok(shortcut) =
                    shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>()
                {
                    if let Err(e) = app_handle.global_shortcut().on_shortcut(
                        shortcut,
                        move |_app, _shortcut, event| {
                            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                                if let Ok(text) = _app.clipboard().read_text() {
                                    let text = text.to_string();
                                    if !text.is_empty() {
                                        let state = _app.state::<AppState>();
                                        let queue = state.queue.clone();
                                        let transcriber = state.transcriber_handle.clone();
                                        tauri::async_runtime::spawn(async move {
                                            let mut q = queue.lock().await;
                                            match q.add_text(text) {
                                                Ok(id) => {
                                                    drop(q);
                                                    log::info!("Added item {id} via hotkey");
                                                    if let Ok(handle) = transcriber.lock() {
                                                        if let Some(ref h) = *handle {
                                                            h.wake();
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Failed to add text to queue: {e}");
                                                }
                                            }
                                        });
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
