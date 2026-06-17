use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

use super::{PiperCatalog, InstalledModel, VoiceCatalog};
use crate::tts::ModelsOrchestrator;

type SharedPiperCatalog = Arc<tokio::sync::Mutex<PiperCatalog>>;

#[tauri::command]
pub async fn piper_fetch_voices(app: AppHandle) -> Result<VoiceCatalog, String> {
    let catalog = app.state::<SharedPiperCatalog>();
    let mut catalog = catalog.lock().await;
    catalog.fetch_voices().await.cloned()
}

#[tauri::command]
pub async fn piper_download_model(
    app: AppHandle,
    voice_key: String,
) -> Result<InstalledModel, String> {
    let catalog = app.state::<SharedPiperCatalog>();
    let catalog = catalog.lock().await;
    catalog.download_voice(&voice_key, &app).await
}

#[tauri::command]
pub async fn piper_list_installed(app: AppHandle) -> Result<Vec<InstalledModel>, String> {
    let catalog = app.state::<SharedPiperCatalog>();
    let catalog = catalog.lock().await;
    let models = catalog.list_installed();
    let tts = app.state::<Arc<ModelsOrchestrator>>();
    tts.refresh_installed_models(models.clone());
    Ok(models)
}

#[tauri::command]
pub async fn piper_delete_model(app: AppHandle, voice_key: String) -> Result<(), String> {
    let catalog = app.state::<SharedPiperCatalog>();
    let catalog = catalog.lock().await;
    catalog.delete_model(&voice_key)?;
    let models = catalog.list_installed();
    let tts = app.state::<Arc<ModelsOrchestrator>>();
    tts.refresh_installed_models(models);
    Ok(())
}
