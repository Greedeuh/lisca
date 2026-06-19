// Kokoro TTS model backend using ORT with shared engine pattern.
// A single ONNX model is shared across all voices; each voice has its own .bin embedding.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{Model, ModelFactory};

pub struct KokoroEngine {
    session: ort::session::Session,
}

impl KokoroEngine {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let session = ort::session::Session::builder()
            .map_err(|e| format!("failed to create ORT session builder: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("failed to load shared model {}: {e}", model_path.display()))?;
        Ok(Self { session })
    }
}

pub struct KokoroModel {
    engine: Arc<KokoroEngine>,
    voice_data: Vec<u8>,
    sample_rate: u32,
}

impl KokoroModel {
    pub fn new(
        engine: Arc<KokoroEngine>,
        voice_path: &Path,
        sample_rate: u32,
    ) -> Result<Self, String> {
        let voice_data = std::fs::read(voice_path)
            .map_err(|e| format!("failed to read voice file {}: {e}", voice_path.display()))?;
        Ok(Self {
            engine,
            voice_data,
            sample_rate,
        })
    }
}

impl Model for KokoroModel {
    fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
        Err("KokoroModel synthesis not yet implemented".to_string())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

pub struct KokoroFactory {
    models_dir: PathBuf,
    shared_engine_path: PathBuf,
    shared_engine: Option<Arc<KokoroEngine>>,
    default_sample_rate: u32,
}

impl KokoroFactory {
    pub fn new(models_dir: PathBuf, shared_engine_path: PathBuf) -> Self {
        Self {
            models_dir,
            shared_engine_path,
            shared_engine: None,
            default_sample_rate: 24000,
        }
    }

    pub fn with_shared_engine(mut self, engine: Arc<KokoroEngine>) -> Self {
        self.shared_engine = Some(engine);
        self
    }

    pub fn load_shared_engine(&mut self) -> Result<(), String> {
        let engine = KokoroEngine::new(&self.shared_engine_path)?;
        self.shared_engine = Some(Arc::new(engine));
        Ok(())
    }
}

impl ModelFactory for KokoroFactory {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
        let engine = self
            .shared_engine
            .as_ref()
            .ok_or("shared engine not initialized — call load_shared_engine() first")?
            .clone();
        let voice_path = self.models_dir.join(format!("{voice_key}.bin"));
        let model = KokoroModel::new(engine, &voice_path, self.default_sample_rate)?;
        Ok(Arc::new(Mutex::new(model)))
    }

    fn is_installed(&self, voice_key: &str) -> bool {
        self.models_dir.join(format!("{voice_key}.bin")).exists()
    }

    fn installed_voices(&self) -> Vec<String> {
        std::fs::read_dir(&self.models_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "bin")
                            .unwrap_or(false)
                    })
                    .filter_map(|e| e.file_name().into_string().ok())
                    .map(|name| name.trim_end_matches(".bin").to_string())
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
    fn kokoro_factory_is_installed_checks_voice_bin() {
        let dir = std::env::temp_dir().join("lisca_kokoro_test_installed");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("voice-a.bin"), "").unwrap();

        let factory = KokoroFactory::new(dir.clone(), PathBuf::from("unused.onnx"));
        assert!(factory.is_installed("voice-a"));
        assert!(!factory.is_installed("nonexistent"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn kokoro_factory_lists_installed_voices() {
        let dir = std::env::temp_dir().join("lisca_kokoro_test_list");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("voice-a.bin"), "").unwrap();
        fs::write(dir.join("voice-b.bin"), "").unwrap();
        fs::write(dir.join("voice-c.txt"), "").unwrap(); // not a .bin

        let factory = KokoroFactory::new(dir.clone(), PathBuf::from("unused.onnx"));
        let mut voices = factory.installed_voices();
        voices.sort();
        assert_eq!(voices, vec!["voice-a", "voice-b"]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn kokoro_factory_create_fails_without_engine() {
        let factory = KokoroFactory::new(PathBuf::from("/tmp"), PathBuf::from("unused.onnx"));
        match factory.create("voice-a") {
            Err(e) => assert!(e.contains("shared engine not initialized")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn kokoro_factory_create_fails_for_missing_voice() {
        let dir = std::env::temp_dir().join("lisca_kokoro_test_missing");
        fs::create_dir_all(&dir).unwrap();

        let factory = KokoroFactory::new(dir.clone(), dir.join("shared.onnx"));
        let result = factory.create("nonexistent");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(dir);
    }
}
