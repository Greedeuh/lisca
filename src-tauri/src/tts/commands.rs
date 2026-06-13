use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

use super::config::BackendConfig;
use super::piper_models;
use super::queue::{QueueConfig, QueueItem, QueueSnapshot};
use super::TtsManager;

type SharedPiperModelManager = Arc<tokio::sync::Mutex<piper_models::PiperModelManager>>;

#[tauri::command]
pub async fn tts_speak(app: AppHandle, text: String) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.speak(&text).await
}

#[tauri::command]
pub fn tts_stop(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.stop();
}

#[tauri::command]
pub fn tts_get_config(app: AppHandle) -> Result<BackendConfig, String> {
    let tts = app.state::<Arc<TtsManager>>();
    Ok(tts.get_config())
}

#[tauri::command]
pub fn tts_set_config(app: AppHandle, config: BackendConfig) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.set_backend(config)
}

#[tauri::command]
pub fn tts_open_resource_dir(app: AppHandle) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    let dir = &tts.resource_dir;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn piper_fetch_voices(app: AppHandle) -> Result<piper_models::VoiceCatalog, String> {
    let manager = app.state::<SharedPiperModelManager>();
    let mut manager = manager.lock().await;
    manager.fetch_voices().await.cloned()
}

#[tauri::command]
pub async fn piper_download_model(
    app: AppHandle,
    voice_key: String,
) -> Result<piper_models::InstalledModel, String> {
    let manager = app.state::<SharedPiperModelManager>();
    let manager = manager.lock().await;
    manager.download_voice(&voice_key, &app).await
}

#[tauri::command]
pub async fn piper_list_installed(app: AppHandle) -> Result<Vec<piper_models::InstalledModel>, String> {
    let manager = app.state::<SharedPiperModelManager>();
    let manager = manager.lock().await;
    Ok(manager.list_installed())
}

#[tauri::command]
pub async fn piper_delete_model(app: AppHandle, voice_key: String) -> Result<(), String> {
    let manager = app.state::<SharedPiperModelManager>();
    let manager = manager.lock().await;
    manager.delete_model(&voice_key)
}

#[tauri::command]
pub async fn tts_queue_add(app: AppHandle, text: String) -> Result<QueueItem, String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_add(text).await
}

#[tauri::command]
pub async fn tts_queue_remove(app: AppHandle, id: u32) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_remove(id).await;
}

#[tauri::command]
pub async fn tts_queue_move(app: AppHandle, id: u32, index: usize) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_move(id, index).await;
}

#[tauri::command]
pub async fn tts_queue_clear(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_clear().await;
}

#[tauri::command]
pub async fn tts_queue_state(app: AppHandle) -> QueueSnapshot {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_state().await
}

#[tauri::command]
pub fn tts_pause(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.pause();
}

#[tauri::command]
pub async fn tts_resume(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.resume();
}

#[tauri::command]
pub fn tts_set_queue_config(app: AppHandle, config: QueueConfig) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.set_queue_config(config)
}

#[tauri::command]
pub fn tts_get_queue_config(app: AppHandle) -> QueueConfig {
    let tts = app.state::<Arc<TtsManager>>();
    tts.get_queue_config()
}
