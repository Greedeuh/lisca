use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::clipboard;
use crate::tts::TtsManager;

#[derive(Debug, Serialize, Deserialize)]
struct HotkeyConfig {
    shortcut: String,
}

fn settings_path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_data_dir().expect("no app data dir");
    dir.join("lisca").join("settings.json")
}

fn save_config(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&HotkeyConfig {
        shortcut: shortcut.to_string(),
    })
    .map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_config(app: &AppHandle) -> Result<Option<String>, String> {
    let path = settings_path(app);
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: HotkeyConfig = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(Some(config.shortcut))
}

fn parse(shortcut: &str) -> Result<(Modifiers, Code), String> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    let mut mods = Modifiers::empty();
    let mut key = "";

    for part in &parts {
        match *part {
            "Control" | "Ctrl" => mods |= Modifiers::CONTROL,
            "Alt" => mods |= Modifiers::ALT,
            "Shift" => mods |= Modifiers::SHIFT,
            "Super" | "Meta" | "Win" | "Cmd" => mods |= Modifiers::SUPER,
            other => key = other,
        }
    }

    if key.is_empty() {
        return Err("No key specified".into());
    }

    let code = match key.to_uppercase().as_str() {
        "A" => Code::KeyA, "B" => Code::KeyB, "C" => Code::KeyC,
        "D" => Code::KeyD, "E" => Code::KeyE, "F" => Code::KeyF,
        "G" => Code::KeyG, "H" => Code::KeyH, "I" => Code::KeyI,
        "J" => Code::KeyJ, "K" => Code::KeyK, "L" => Code::KeyL,
        "M" => Code::KeyM, "N" => Code::KeyN, "O" => Code::KeyO,
        "P" => Code::KeyP, "Q" => Code::KeyQ, "R" => Code::KeyR,
        "S" => Code::KeyS, "T" => Code::KeyT, "U" => Code::KeyU,
        "V" => Code::KeyV, "W" => Code::KeyW, "X" => Code::KeyX,
        "Y" => Code::KeyY, "Z" => Code::KeyZ,
        "0" => Code::Digit0, "1" => Code::Digit1, "2" => Code::Digit2,
        "3" => Code::Digit3, "4" => Code::Digit4, "5" => Code::Digit5,
        "6" => Code::Digit6, "7" => Code::Digit7, "8" => Code::Digit8,
        "9" => Code::Digit9,
        "SPACE" => Code::Space,
        "F1" => Code::F1, "F2" => Code::F2, "F3" => Code::F3, "F4" => Code::F4,
        "F5" => Code::F5, "F6" => Code::F6, "F7" => Code::F7, "F8" => Code::F8,
        "F9" => Code::F9, "F10" => Code::F10, "F11" => Code::F11, "F12" => Code::F12,
        other => return Err(format!("Unknown key: {}", other)),
    };

    Ok((mods, code))
}

#[tauri::command]
pub fn set(app: AppHandle, shortcut: String) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    let (mods, code) = parse(&shortcut)?;
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

    save_config(&app, &shortcut)
}

#[tauri::command]
pub fn get(app: AppHandle) -> Result<Option<String>, String> {
    load_config(&app)
}
