use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TtsConfig {
    pub rate: f32,
    pub volume: f32,
    pub voice: String,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            rate: 200.0,
            volume: 1.0,
            voice: String::new(),
        }
    }
}

pub struct TtsManager {
    engine: Mutex<tts::Tts>,
    pub config: Arc<Mutex<TtsConfig>>,
    _app_handle: AppHandle,
}

impl TtsManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let engine = tts::Tts::default().map_err(|e| format!("Failed to init TTS: {}", e))?;

        Ok(Self {
            engine: Mutex::new(engine),
            config: Arc::new(Mutex::new(TtsConfig::default())),
            _app_handle: app_handle,
        })
    }

    pub async fn update_config(&self, config: TtsConfig) {
        let mut engine = self.engine.lock().await;
        let _ = engine.set_rate(config.rate);
        let _ = engine.set_volume(config.volume);
        if !config.voice.is_empty() {
            if let Ok(voices) = engine.voices() {
                if let Some(v) = voices.iter().find(|v| v.name() == config.voice) {
                    let _ = engine.set_voice(v);
                }
            }
        }
        drop(engine);
        *self.config.lock().await = config;
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let mut engine = self.engine.lock().await;
        engine
            .speak(text, false)
            .map_err(|e| format!("TTS speak failed: {}", e))?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut engine = self.engine.lock().await;
        engine.stop().map_err(|e| format!("TTS stop failed: {}", e))?;
        Ok(())
    }

    pub async fn list_voices(&self) -> Result<Vec<String>, String> {
        let engine = self.engine.lock().await;
        let voices = engine
            .voices()
            .map_err(|e| format!("Failed to list voices: {}", e))?;
        Ok(voices.into_iter().map(|v| v.name()).collect())
    }
}
