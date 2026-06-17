use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use unicode_normalization::UnicodeNormalization;

use crate::tts::{BackendFactory, TtsBackend};
use super::InstalledModel;

static INSTALLED_LANGUAGES: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static DATA_DIR_SET: std::sync::Once = std::sync::Once::new();

fn ensure_espeak_data(resource_dir: &Path, language: &str) {
    let data_dir = resource_dir.join("espeak-ng-data");

    DATA_DIR_SET.call_once(|| {
        std::env::set_var("ESPEAK_DATA_PATH", &data_dir);
    });

    let lang_code = language.split('-').next().unwrap_or("fr");

    let mut installed = INSTALLED_LANGUAGES.lock().unwrap();
    if !installed.contains(lang_code) {
        if let Err(e) = espeak_ng::install_bundled_languages(&data_dir, &[lang_code]) {
            eprintln!("Failed to install espeak-ng data for {}: {}", lang_code, e);
        } else {
            eprintln!("Installed espeak-ng data for language: {}", lang_code);
            installed.insert(lang_code.to_string());
        }
    }
}

#[derive(serde::Deserialize)]
struct PiperConfig {
    #[serde(default)]
    inference: PiperInference,
    #[serde(default)]
    audio: PiperAudio,
    #[serde(default)]
    espeak: PiperEspeak,
    phoneme_id_map: HashMap<String, Vec<i64>>,
}

#[derive(serde::Deserialize, Default)]
struct PiperEspeak {
    #[serde(default = "default_espeak_voice")]
    voice: String,
}

fn default_espeak_voice() -> String {
    "en-us".into()
}

#[derive(serde::Deserialize)]
struct PiperInference {
    #[serde(default = "default_noise_scale")]
    noise_scale: f32,
    #[serde(default = "default_length_scale")]
    length_scale: f32,
    #[serde(default = "default_noise_w")]
    noise_w: f32,
}

#[derive(serde::Deserialize)]
struct PiperAudio {
    #[serde(default = "default_sample_rate")]
    sample_rate: u32,
}

fn default_noise_scale() -> f32 { 0.667 }
fn default_length_scale() -> f32 { 1.0 }
fn default_noise_w() -> f32 { 0.8 }
fn default_sample_rate() -> u32 { 22050 }

impl Default for PiperInference {
    fn default() -> Self {
        Self {
            noise_scale: default_noise_scale(),
            length_scale: default_length_scale(),
            noise_w: default_noise_w(),
        }
    }
}

impl Default for PiperAudio {
    fn default() -> Self {
        Self {
            sample_rate: default_sample_rate(),
        }
    }
}

pub struct PiperModel {
    session: ort::session::Session,
    phoneme_to_id: HashMap<char, i64>,
    config: PiperConfig,
}

impl PiperModel {
    pub fn load(model_path: &Path, config_path: &Path, resource_dir: &Path) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Model not found: {}", model_path.display()));
        }
        if !config_path.exists() {
            return Err(format!("Config not found: {}", config_path.display()));
        }

        let config_str = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Read config: {}", e))?;
        let config: PiperConfig = serde_json::from_str(&config_str)
            .map_err(|e| format!("Parse config: {}", e))?;

        // Initialize espeak-ng with the correct language from config
        ensure_espeak_data(resource_dir, &config.espeak.voice);

        let session = crate::tts::onnx_session::create_session(model_path)
            .map_err(|e| format!("Session: {}", e))?;

        let mut phoneme_to_id = HashMap::new();
        for (phoneme, ids) in &config.phoneme_id_map {
            if let Some(&id) = ids.first() {
                for ch in phoneme.chars() {
                    phoneme_to_id.insert(ch, id);
                }
            }
        }

        let mut model = Self {
            session,
            phoneme_to_id,
            config,
        };

        eprintln!("Warming up Piper model...");
        let start = std::time::Instant::now();
        match model.warmup() {
            Ok(()) => eprintln!("Piper model warmed up in {}ms", start.elapsed().as_millis()),
            Err(e) => eprintln!("Piper warmup failed (non-fatal): {}", e),
        }

        Ok(model)
    }

    fn text_to_phoneme_ids(&self, text: &str) -> Vec<i64> {
        // Extract language code from espeak voice (e.g., "en-us" -> "en")
        let lang_code = self.config.espeak.voice.split('-').next().unwrap_or("en");
        let ipa = match espeak_ng::text_to_ipa(lang_code, text) {
            Ok(ipa) => ipa,
            Err(e) => {
                eprintln!("espeak IPA error for {}: {}", lang_code, e);
                return Vec::new();
            }
        };

        let decomposed: String = ipa.nfd().collect();

        let bos = self.phoneme_to_id.get(&'^').copied().unwrap_or(1);
        let pad = self.phoneme_to_id.get(&'_').copied().unwrap_or(0);
        let eos = self.phoneme_to_id.get(&'$').copied().unwrap_or(2);

        let mut ids = Vec::new();
        ids.push(bos);

        for ch in decomposed.chars() {
            if ch == '^' || ch == '_' || ch == '$' {
                continue;
            }
            if ch.is_whitespace() {
                if let Some(&id) = self.phoneme_to_id.get(&' ') {
                    ids.push(id);
                    ids.push(pad);
                }
                continue;
            }
            if let Some(&id) = self.phoneme_to_id.get(&ch) {
                ids.push(id);
                ids.push(pad);
            }
        }

        ids.push(eos);
        ids
    }

    fn warmup(&mut self) -> Result<(), String> {
        let t_input = ort::value::Tensor::from_array(([1, 1], vec![0i64]))
            .map_err(|e| format!("Warmup tensor: {}", e))?;
        let t_lengths = ort::value::Tensor::from_array(([1], vec![1i64]))
            .map_err(|e| format!("Warmup tensor: {}", e))?;
        let t_scales = ort::value::Tensor::from_array(([3], vec![0.667f32, 1.0, 0.8]))
            .map_err(|e| format!("Warmup tensor: {}", e))?;

        let _outputs = self
            .session
            .run(ort::inputs![
                "input" => t_input.into_dyn(),
                "input_lengths" => t_lengths.into_dyn(),
                "scales" => t_scales.into_dyn(),
            ])
            .map_err(|e| format!("Warmup inference: {}", e))?;

        Ok(())
    }
}

impl TtsBackend for PiperModel {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String> {
        let ids = self.text_to_phoneme_ids(text);
        if ids.is_empty() {
            return Err("No phonemes generated from text".into());
        }

        let length_scale = self.config.inference.length_scale / speed;
        let seq_len = ids.len() as i64;

        let t_input = ort::value::Tensor::from_array(([1, ids.len()], ids))
            .map_err(|e| format!("Tensor input: {}", e))?;
        let t_lengths = ort::value::Tensor::from_array(([1], vec![seq_len]))
            .map_err(|e| format!("Tensor lengths: {}", e))?;
        let t_scales = ort::value::Tensor::from_array(([3], vec![
            self.config.inference.noise_scale,
            length_scale,
            self.config.inference.noise_w,
        ]))
            .map_err(|e| format!("Tensor scales: {}", e))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input" => t_input.into_dyn(),
                "input_lengths" => t_lengths.into_dyn(),
                "scales" => t_scales.into_dyn(),
            ])
            .map_err(|e| format!("Inference: {}", e))?;

        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output: {}", e))?;

        Ok(data.to_vec())
    }

    fn sample_rate(&self) -> u32 {
        self.config.audio.sample_rate
    }
}

pub struct PiperBackendFactory {
    pub resource_dir: PathBuf,
}

impl BackendFactory for PiperBackendFactory {
    fn create_from_installed(
        &self,
        model: &InstalledModel,
    ) -> Result<Box<dyn TtsBackend>, String> {
        let mp = PathBuf::from(&model.model_path);
        let cp = PathBuf::from(&model.config_path);
        PiperModel::load(&mp, &cp, &self.resource_dir).map(|m| Box::new(m) as Box<dyn TtsBackend>)
    }
}
