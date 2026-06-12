use std::sync::Arc;
use tauri::Manager;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::clipboard;
use crate::hotkey;
use crate::tts::{self, TtsManager};

#[tauri::command]
pub fn set_hotkey(app: AppHandle, shortcut: String) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    let (mods, code) = hotkey::parse_shortcut(&shortcut)?;
    let sc = Shortcut::new(Some(mods), code);

    let tts = app.state::<Arc<TtsManager>>().inner().clone();
    app.global_shortcut()
        .on_shortcut(sc, move |app_handle, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let tts = tts.clone();
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    match clipboard::read_text(&app_handle) {
                        Ok(text) if !text.is_empty() => {
                            if let Err(e) = tts.speak(&text).await {
                                eprintln!("TTS error: {}", e);
                            }
                        }
                        Ok(_) => eprintln!("Clipboard is empty"),
                        Err(e) => eprintln!("Clipboard error: {}", e),
                    }
                });
            }
        })
        .map_err(|e| e.to_string())?;

    hotkey::save_hotkey(&app, &shortcut)
}

#[tauri::command]
pub fn load_hotkey(app: AppHandle) -> Result<Option<String>, String> {
    hotkey::load_hotkey(&app)
}

#[tauri::command]
pub async fn update_tts_config(
    app: AppHandle,
    rate: Option<f32>,
    volume: Option<f32>,
    voice: Option<String>,
) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    let current = {
        let cfg = tts.config.lock().await;
        tts::TtsConfig {
            rate: rate.unwrap_or(cfg.rate),
            volume: volume.unwrap_or(cfg.volume),
            voice: voice.unwrap_or_else(|| cfg.voice.clone()),
        }
    };
    tts.update_config(current).await;
    Ok(())
}

#[tauri::command]
pub async fn speak_text(app: AppHandle, text: String) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.speak(&text).await
}

#[tauri::command]
pub async fn stop_speaking(app: AppHandle) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.stop().await
}

#[tauri::command]
pub fn read_selected_text(app: AppHandle) -> Result<String, String> {
    clipboard::read_text(&app)
}

#[tauri::command]
pub async fn list_voices(app: AppHandle) -> Result<Vec<String>, String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.list_voices().await
}
