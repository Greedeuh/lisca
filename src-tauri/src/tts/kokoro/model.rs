use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::tts::{BackendFactory, TtsBackend};
use crate::tts::piper::InstalledModel;

const STYLE_DIM: usize = 256;
const MAX_STYLE_INDEX: usize = 509;

pub struct KokoroModel {
    session: ort::session::Session,
    char_to_id: HashMap<String, i64>,
    g2p: misaki_rs::G2P,
    voice_styles: Vec<f32>,
}

impl KokoroModel {
    pub fn load(
        model_path: &Path,
        voice_path: &Path,
        tokenizer_path: &Path,
    ) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Model not found: {}", model_path.display()));
        }
        if !voice_path.exists() {
            return Err(format!("Voice not found: {}", voice_path.display()));
        }
        if !tokenizer_path.exists() {
            return Err(format!("Tokenizer not found: {}", tokenizer_path.display()));
        }

        eprintln!("[kokoro] Loading model: {}", model_path.display());
        let session = crate::tts::onnx_session::create_session(model_path)
            .map_err(|e| format!("Session: {}", e))?;

        // TODO: why do we need to print the model inputs and outputs? what are inputs and outputs?
        eprintln!("[kokoro] Model inputs:");
        for input in session.inputs() {
            eprintln!("  - {} : {:?}", input.name(), input.dtype());
        }
        eprintln!("[kokoro] Model outputs:");
        for output in session.outputs() {
            eprintln!("  - {} : {:?}", output.name(), output.dtype());
        }

        eprintln!("[kokoro] Loading tokenizer: {}", tokenizer_path.display());
        let tok_str = std::fs::read_to_string(tokenizer_path)
            .map_err(|e| format!("Read tokenizer: {}", e))?;
        let tok_val: serde_json::Value = serde_json::from_str(&tok_str)
            .map_err(|e| format!("Parse tokenizer JSON: {}", e))?;
        let vocab = tok_val["model"]["vocab"]
            .as_object()
            .ok_or("Missing model.vocab in tokenizer")?;
        let mut char_to_id = HashMap::new();
        for (k, v) in vocab {
            if let Some(id) = v.as_i64() {
                char_to_id.insert(k.clone(), id);
            }
        }
        eprintln!("[tokoro] Tokenizer vocab: {} entries", char_to_id.len());

        let g2p = misaki_rs::G2P::new(misaki_rs::Language::EnglishUS);

        eprintln!("[kokoro] Loading voice: {}", voice_path.display());
        let voice_bytes = std::fs::read(voice_path)
            .map_err(|e| format!("Read voice: {}", e))?;
        if voice_bytes.len() % 4 != 0 {
            return Err("Voice file size not aligned to 4 bytes".into());
        }
        let voice_styles: Vec<f32> = voice_bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        eprintln!("[kokoro] Voice loaded: {} floats ({} styles)", voice_styles.len(), voice_styles.len() / STYLE_DIM);

        let model = Self {
            session,
            char_to_id,
            g2p,
            voice_styles,
        };

        eprintln!("[kokoro] Model loaded successfully");

        Ok(model)
    }

    fn text_to_tokens(&self, text: &str) -> Result<Vec<i64>, String> {
        let (phonemes, _tokens) = self.g2p.g2p(text)
            .map_err(|e| format!("G2P: {}", e))?;
        let mut ids: Vec<i64> = vec![0];
        for ch in phonemes.chars() {
            let s = ch.to_string();
            if let Some(&id) = self.char_to_id.get(&s) {
                ids.push(id);
            } else if ch == '\u{200d}' { // TODO: explain what this character is and why we need to handle it
                continue;
            } else { // TODO: explain why we need to handle unknown characters and what we do with them
                if let Some(&id) = self.char_to_id.get(" ") {
                    ids.push(id);
                }
            }
        }
        ids.push(0);
        Ok(ids)
    }

    // TODO: explain what this function does and why we need it
    fn get_style(&self, num_tokens: usize) -> &[f32] {
        let idx = num_tokens.min(MAX_STYLE_INDEX);
        let offset = idx * STYLE_DIM;
        if offset + STYLE_DIM <= self.voice_styles.len() {
            &self.voice_styles[offset..offset + STYLE_DIM]
        } else {
            &self.voice_styles[0..STYLE_DIM]
        }
    }
}

impl TtsBackend for KokoroModel {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String> {
        let ids = self.text_to_tokens(text)?;
        if ids.is_empty() {
            return Err("No tokens generated from text".into());
        }

        let num_tokens = ids.len().min(MAX_STYLE_INDEX);
        let style = self.get_style(num_tokens);

        let t_input = ort::value::Tensor::from_array(([1, ids.len()], ids))
            .map_err(|e| format!("Tensor input: {}", e))?;
        let t_style = ort::value::Tensor::from_array(([1, STYLE_DIM], style.to_vec()))
            .map_err(|e| format!("Tensor style: {}", e))?;
        let t_speed = ort::value::Tensor::from_array(([1], vec![speed]))
            .map_err(|e| format!("Tensor speed: {}", e))?;

        let start = std::time::Instant::now();
        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => t_input.into_dyn(),
                "style" => t_style.into_dyn(), // TODO: explain what is style and why we need it
                "speed" => t_speed.into_dyn(), // TODO: explain what is speed and why we need it
            ])
            .map_err(|e| format!("Inference: {}", e))?;
        eprintln!("[kokoro] Inference: {}ms", start.elapsed().as_millis());

        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output: {}", e))?;

        Ok(data.to_vec())
    }

    fn sample_rate(&self) -> u32 {
        // TODO: why this value?
        24000
    }
}

pub struct KokoroBackendFactory {
    pub model_dir: PathBuf,
}

impl BackendFactory for KokoroBackendFactory {
    fn create_from_installed(
        &self,
        _model: &InstalledModel,
    ) -> Result<Box<dyn TtsBackend>, String> {
        let model = self.model_dir.join("model_q8f16.onnx");
        let voice = self.model_dir.join("af.bin");
        let tokenizer = self.model_dir.join("tokenizer.json");
        KokoroModel::load(&model, &voice, &tokenizer).map(|m| Box::new(m) as Box<dyn TtsBackend>)
    }
}
