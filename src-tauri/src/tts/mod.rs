mod kokoro;
mod session;

use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;

use kokoro::KokoroModel;

pub struct TtsManager {
    backend: Mutex<Option<KokoroModel>>,
    system_tts: Mutex<tts::Tts>,
    _app_handle: AppHandle,
}

impl TtsManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let engine = tts::Tts::default().map_err(|e| format!("Failed to init system TTS: {}", e))?;

        Ok(Self {
            backend: Mutex::new(None),
            system_tts: Mutex::new(engine),
            _app_handle: app_handle,
        })
    }

    /// Load a Kokoro ONNX model and voice file.
    pub async fn load_model(&self, model_path: &str, voice_path: &str) -> Result<(), String> {
        let model_path = model_path.to_string();
        let voice_path = voice_path.to_string();

        let model = tokio::task::spawn_blocking(move || {
            KokoroModel::load(
                std::path::Path::new(&model_path),
                std::path::Path::new(&voice_path),
            )
        })
        .await
        .map_err(|e| format!("Task: {}", e))?
        .map_err(|e| format!("Load: {}", e))?;

        *self.backend.lock().await = Some(model);
        Ok(())
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        // Try Kokoro first
        {
            let mut backend = self.backend.lock().await;
            if let Some(ref mut model) = *backend {
                let audio = model.synthesize(text, 1.0)?;
                let sample_rate = model.sample_rate();
                drop(backend);

                // Play audio via rodio
                self.play_audio(&audio, sample_rate).await?;
                return Ok(());
            }
        }

        // Fallback to system TTS
        let mut engine = self.system_tts.lock().await;
        engine
            .speak(text, false)
            .map_err(|e| format!("System TTS: {}", e))?;
        Ok(())
    }

    async fn play_audio(&self, samples: &[f32], sample_rate: u32) -> Result<(), String> {
        let samples = samples.to_vec();
        tokio::task::spawn_blocking(move || {
            use rodio::buffer::SamplesBuffer;
            use rodio::{OutputStream, Sink};

            let (_stream, handle) = OutputStream::try_default()
                .map_err(|e| format!("Audio output: {}", e))?;
            let sink = Sink::try_new(&handle)
                .map_err(|e| format!("Audio sink: {}", e))?;

            // Convert f32 samples to i16 for rodio
            let i16_samples: Vec<i16> = samples
                .iter()
                .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();

            let buffer = SamplesBuffer::new(1, sample_rate, i16_samples);
            sink.append(buffer);
            sink.sleep_until_end();

            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Task: {}", e))?
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut engine = self.system_tts.lock().await;
        engine.stop().map_err(|e| format!("Stop: {}", e))?;
        Ok(())
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
pub async fn tts_load_model(
    app: AppHandle,
    model_path: String,
    voice_path: String,
) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.load_model(&model_path, &voice_path).await
}
