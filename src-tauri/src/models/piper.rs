// Piper TTS model backend using ORT (ONNX Runtime).
// Each voice has its own ONNX model file.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{Model, ModelFactory};

pub struct PiperModel {
    session: ort::session::Session,
    sample_rate: u32,
}

impl PiperModel {
    pub fn new(model_path: &Path, sample_rate: u32) -> Result<Self, String> {
        let session = ort::session::Session::builder()
            .map_err(|e| format!("failed to create ORT session builder: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("failed to load model {}: {e}", model_path.display()))?;
        Ok(Self { session, sample_rate })
    }
}

impl Model for PiperModel {
    fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
        Err("PiperModel synthesis not yet implemented".to_string())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

pub struct PiperFactory {
    models_dir: PathBuf,
    default_sample_rate: u32,
}

impl PiperFactory {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            models_dir,
            default_sample_rate: 22050,
        }
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.default_sample_rate = sample_rate;
        self
    }
}

impl ModelFactory for PiperFactory {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
        let model_path = self.models_dir.join(voice_key).join("model.onnx");
        let model = PiperModel::new(&model_path, self.default_sample_rate)?;
        Ok(Arc::new(Mutex::new(model)))
    }

    fn is_installed(&self, voice_key: &str) -> bool {
        self.models_dir
            .join(voice_key)
            .join("model.onnx")
            .exists()
    }

    fn installed_voices(&self) -> Vec<String> {
        std::fs::read_dir(&self.models_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter(|e| e.path().join("model.onnx").exists())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn piper_factory_is_installed_checks_model_file() {
        let dir = std::env::temp_dir().join("lisca_piper_test_installed");
        let voice_dir = dir.join("test-voice");
        fs::create_dir_all(&voice_dir).unwrap();
        fs::write(voice_dir.join("model.onnx"), "").unwrap();

        let factory = PiperFactory::new(dir.clone());
        assert!(factory.is_installed("test-voice"));
        assert!(!factory.is_installed("nonexistent"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn piper_factory_lists_installed_voices() {
        let dir = std::env::temp_dir().join("lisca_piper_test_list");
        fs::create_dir_all(dir.join("voice-a")).unwrap();
        fs::create_dir_all(dir.join("voice-b")).unwrap();
        fs::create_dir_all(dir.join("voice-c")).unwrap();
        fs::write(dir.join("voice-a").join("model.onnx"), "").unwrap();
        fs::write(dir.join("voice-b").join("model.onnx"), "").unwrap();
        // voice-c has no model.onnx

        let factory = PiperFactory::new(dir.clone());
        let mut voices = factory.installed_voices();
        voices.sort();
        assert_eq!(voices, vec!["voice-a", "voice-b"]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn piper_factory_create_fails_for_missing_model() {
        let dir = std::env::temp_dir().join("lisca_piper_test_missing");
        let factory = PiperFactory::new(dir.clone());
        let result = factory.create("nonexistent");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(dir);
    }
}
