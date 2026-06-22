// Kokoro TTS model backend using ORT with shared engine pattern.
// A single ONNX model is shared across all voices; each voice has its own .bin embedding.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use misaki_rs::{G2P, Language};

use super::{Model, ModelFactory};
use super::kokoro_phonemizer::KokoroTokenizerConfig;

pub(crate)  struct KokoroEngine {
    session: std::sync::Mutex<ort::session::Session>,
}

impl KokoroEngine {
     fn new(model_path: &Path) -> Result<Self, String> {
        let session = ort::session::Session::builder()
            .map_err(|e| format!("failed to create ORT session builder: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("failed to load shared model {}: {e}", model_path.display()))?;
        Ok(Self { session: std::sync::Mutex::new(session) })
    }

     fn run_inputs(
        &self,
        input_ids: ort::value::DynValue,
        style: ort::value::DynValue,
        speed: ort::value::DynValue,
    ) -> Result<Vec<f32>, String> {
        let mut session = self.session.lock().map_err(|e| format!("lock error: {e}"))?;
        let outputs = session
            .run(ort::inputs![
                "input_ids" => input_ids,
                "style" => style,
                "speed" => speed,
            ])
            .map_err(|e| format!("ORT inference error: {e}"))?;
        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output extraction: {e}"))?;
        Ok(data.to_vec())
    }
}

pub(crate)  struct KokoroModel {
    engine: Arc<KokoroEngine>,
    vocab: HashMap<char, i64>,
    pad_token_id: i64,
    voices: Vec<Vec<f32>>,
    g2p: G2P,
    sample_rate: u32,
}

impl KokoroModel {
     fn new(
        engine: Arc<KokoroEngine>,
        voice_path: &Path,
        resource_dir: &Path,
        sample_rate: u32,
    ) -> Result<Self, String> {
        let voice_data = std::fs::read(voice_path)
            .map_err(|e| format!("failed to read voice file {}: {e}", voice_path.display()))?;

        let voices = Self::load_voice_data(&voice_data)?;

        let config = KokoroTokenizerConfig::load(resource_dir)?;
        let (vocab, pad_token_id) = config.build_tokenizer();

        let g2p = G2P::new(Language::EnglishUS);

        Ok(Self {
            engine,
            vocab,
            pad_token_id,
            voices,
            g2p,
            sample_rate,
        })
    }

    fn load_voice_data(bytes: &[u8]) -> Result<Vec<Vec<f32>>, String> {
        if !bytes.len().is_multiple_of(4) {
            return Err("Invalid voice file size".into());
        }

        let floats: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        // Reshape to (N, 256)
        if !floats.len().is_multiple_of(256) {
            return Err(format!(
                "Voice file has {} floats, not divisible by 256",
                floats.len()
            ));
        }

        let n = floats.len() / 256;
        let voices = (0..n)
            .map(|i| floats[i * 256..(i + 1) * 256].to_vec())
            .collect();

        Ok(voices)
    }

    fn phonemize(&self, text: &str) -> String {
        let (phonemes, _) = self.g2p.g2p(text).unwrap_or_default();
        phonemes
    }

    fn tokenize(&self, phonemes: &str) -> Vec<i64> {
        let mut tokens = vec![self.pad_token_id];
        for ch in phonemes.chars() {
            if let Some(&id) = self.vocab.get(&ch) {
                tokens.push(id);
            }
        }
        tokens.push(self.pad_token_id);
        tokens
    }
}

impl Model for KokoroModel {
    fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, String> {
        let phonemes = self.phonemize(text);
        log::debug!("Text: {}", text);
        log::debug!("Phonemes: {}", phonemes);

        let tokens = self.tokenize(&phonemes);

        if tokens.is_empty() {
            return Err("No tokens generated from text".into());
        }

        log::debug!("Tokens ({}): {:?}", tokens.len(), &tokens[..tokens.len().min(20)]);

        // Max context length is 510 (512 - 2 for pad tokens)
        let tokens = if tokens.len() > 510 {
            &tokens[..510]
        } else {
            &tokens
        };

        // Select style vector based on token count (exclude BOS/EOS)
        let inner_len = tokens.len().saturating_sub(2).max(1);
        let style_idx = (inner_len - 1).min(self.voices.len() - 1);
        let ref_s = &self.voices[style_idx];

        // Build input: tokenize() already adds pad tokens at start and end
        let input_ids: Vec<i64> = tokens.to_vec();

        // Create tensors
        let t_input_ids = ort::value::Tensor::from_array(([1, input_ids.len()], input_ids))
            .map_err(|e| format!("Tensor input_ids: {}", e))?;

        let t_style = ort::value::Tensor::from_array(([1, 256], ref_s.clone()))
            .map_err(|e| format!("Tensor style: {}", e))?;

        let t_speed = ort::value::Tensor::from_array(([1], vec![1.0f32]))
            .map_err(|e| format!("Tensor speed: {}", e))?;

        // Run inference and extract audio
        let audio = self
            .engine
            .run_inputs(t_input_ids.into_dyn(), t_style.into_dyn(), t_speed.into_dyn())?;

        log::debug!("Kokoro output length: {}", audio.len());
        Ok(audio)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

pub(crate)  struct KokoroFactory {
    models_dir: PathBuf,
    shared_engine_path: PathBuf,
    resource_dir: PathBuf,
    shared_engine: std::sync::Mutex<Option<Arc<KokoroEngine>>>,
    default_sample_rate: u32,
}

impl KokoroFactory {
    pub(crate)  fn new(models_dir: PathBuf, shared_engine_path: PathBuf, resource_dir: PathBuf) -> Self {
        Self {
            models_dir,
            shared_engine_path,
            resource_dir,
            shared_engine: std::sync::Mutex::new(None),
            default_sample_rate: 24000,
        }
    }

     fn with_shared_engine(self, engine: Arc<KokoroEngine>) -> Self {
        *self.shared_engine.lock().unwrap() = Some(engine);
        self
    }

    fn ensure_engine(&self) -> Result<Arc<KokoroEngine>, String> {
        {
            let guard = self.shared_engine.lock().map_err(|e| e.to_string())?;
            if let Some(ref engine) = *guard {
                return Ok(engine.clone());
            }
        }
        // Not loaded yet — try loading from disk
        if self.shared_engine_path.exists() {
            let engine = KokoroEngine::new(&self.shared_engine_path)?;
            let engine = Arc::new(engine);
            let mut guard = self.shared_engine.lock().map_err(|e| e.to_string())?;
            // Double-check in case another thread loaded it
            if guard.is_none() {
                *guard = Some(engine.clone());
            }
            return Ok(guard.as_ref().unwrap().clone());
        }
        Err(format!(
            "Kokoro shared engine not found at {}",
            self.shared_engine_path.display()
        ))
    }
}

impl ModelFactory for KokoroFactory {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
        let engine = self.ensure_engine()?;
        let voice_path = self.models_dir.join(format!("{voice_key}.bin"));
        let model = KokoroModel::new(engine, &voice_path, &self.resource_dir, self.default_sample_rate)?;
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

        let factory = KokoroFactory::new(dir.clone(), PathBuf::from("unused.onnx"), dir.clone());
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

        let factory = KokoroFactory::new(dir.clone(), PathBuf::from("unused.onnx"), dir.clone());
        let mut voices = factory.installed_voices();
        voices.sort();
        assert_eq!(voices, vec!["voice-a", "voice-b"]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn kokoro_factory_create_fails_without_engine() {
        let factory = KokoroFactory::new(PathBuf::from("/tmp"), PathBuf::from("unused.onnx"), PathBuf::from("/tmp"));
        match factory.create("voice-a") {
            Err(e) => assert!(e.contains("shared engine not found")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn kokoro_factory_create_fails_for_missing_voice() {
        let dir = std::env::temp_dir().join("lisca_kokoro_test_missing");
        fs::create_dir_all(&dir).unwrap();

        let factory = KokoroFactory::new(dir.clone(), dir.join("shared.onnx"), dir.clone());
        let result = factory.create("nonexistent");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(dir);
    }
}
