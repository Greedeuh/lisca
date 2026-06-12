mod model;
mod session;

pub use model::TtsModel;

use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;

enum TtsBackend {
    System(tts::Tts),
    Onnx(TtsModel),
}

pub struct TtsManager {
    backend: Mutex<TtsBackend>,
    _app_handle: AppHandle,
}

impl TtsManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let engine = tts::Tts::default().map_err(|e| format!("Failed to init TTS: {}", e))?;

        Ok(Self {
            backend: Mutex::new(TtsBackend::System(engine)),
            _app_handle: app_handle,
        })
    }

    /// Load an ONNX TTS model, replacing the system backend.
    pub async fn load_model(&self, model_path: &str) -> Result<(), String> {
        let path = std::path::Path::new(model_path);
        let tts_model = TtsModel::load(path)?;
        *self.backend.lock().await = TtsBackend::Onnx(tts_model);
        Ok(())
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let mut backend = self.backend.lock().await;
        match *backend {
            TtsBackend::System(ref mut engine) => {
                engine
                    .speak(text, false)
                    .map_err(|e| format!("TTS speak failed: {}", e))?;
                Ok(())
            }
            TtsBackend::Onnx(ref mut model) => {
                // TODO: tokenize text -> run model -> play audio via rodio
                let _ = model;
                Err("ONNX TTS not yet implemented - needs tokenizer".into())
            }
        }
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut backend = self.backend.lock().await;
        match *backend {
            TtsBackend::System(ref mut engine) => {
                engine.stop().map_err(|e| format!("TTS stop failed: {}", e))?;
                Ok(())
            }
            TtsBackend::Onnx(_) => Ok(()),
        }
    }
}

#[tauri::command]
pub async fn tts_speak(app: AppHandle, text: String) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.speak(&text).await
}

#[tauri::command]
pub async fn tts_stop(app: AppHandle) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.stop().await
}

#[tauri::command]
pub async fn tts_load_model(app: AppHandle, model_path: String) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.load_model(&model_path).await
}
