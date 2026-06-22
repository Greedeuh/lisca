// Piper TTS model backend using ORT (ONNX Runtime).
// Each voice has its own ONNX model file + config.json with phoneme_id_map.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use unicode_normalization::UnicodeNormalization;

use super::{Model, ModelFactory};

static INSTALLED_LANGUAGES: LazyLock<std::sync::Mutex<HashSet<String>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));
static DATA_DIR_SET: std::sync::Once = std::sync::Once::new();

fn ensure_espeak_data(resource_dir: &Path, language: &str) {
    let data_dir = resource_dir.join("espeak-ng-data");

    DATA_DIR_SET.call_once(|| {
        std::env::set_var("ESPEAK_DATA_PATH", &data_dir);
    });

    let lang_code = language.split('-').next().unwrap_or("en");

    let mut installed = INSTALLED_LANGUAGES.lock().unwrap();
    if !installed.contains(lang_code) {
        if let Err(e) = espeak_ng::install_bundled_languages(&data_dir, &[lang_code]) {
            log::warn!("Failed to install espeak-ng data for {}: {}", lang_code, e);
        } else {
            log::info!("Installed espeak-ng data for language: {}", lang_code);
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

pub(crate)  struct PiperModel {
    session: ort::session::Session,
    phoneme_to_id: HashMap<char, i64>,
    config: PiperConfig,
}

impl PiperModel {
     fn new(model_path: &Path, config_path: &Path, resource_dir: &Path) -> Result<Self, String> {
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

        ensure_espeak_data(resource_dir, &config.espeak.voice);

        let session = ort::session::Session::builder()
            .map_err(|e| format!("failed to create ORT session builder: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("failed to load model {}: {e}", model_path.display()))?;

        let mut phoneme_to_id = HashMap::new();
        for (phoneme, ids) in &config.phoneme_id_map {
            if let Some(&id) = ids.first() {
                for ch in phoneme.chars() {
                    phoneme_to_id.insert(ch, id);
                }
            }
        }

        Ok(Self {
            session,
            phoneme_to_id,
            config,
        })
    }

    fn text_to_phoneme_ids(&self, text: &str) -> Vec<i64> {
        let lang_code = self.config.espeak.voice.split('-').next().unwrap_or("en");
        let ipa = match espeak_ng::text_to_ipa(lang_code, text) {
            Ok(ipa) => ipa,
            Err(e) => {
                log::warn!("espeak IPA error for {}: {}", lang_code, e);
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

    #[allow(dead_code)]
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

impl Model for PiperModel {
    fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, String> {
        let ids = self.text_to_phoneme_ids(text);
        if ids.is_empty() {
            return Err("No phonemes generated from text".into());
        }

        let length_scale = self.config.inference.length_scale;
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

pub(crate)  struct PiperFactory {
    models_dir: PathBuf,
    resource_dir: PathBuf,
}

impl PiperFactory {
    pub(crate)  fn new(models_dir: PathBuf, resource_dir: PathBuf) -> Self {
        Self {
            models_dir,
            resource_dir,
        }
    }
}

impl ModelFactory for PiperFactory {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
        let voice_dir = self.models_dir.join(voice_key);
        let model_path = voice_dir.join(format!("{}.onnx", voice_key));
        let config_path = voice_dir.join(format!("{}.onnx.json", voice_key));
        let model = PiperModel::new(&model_path, &config_path, &self.resource_dir)?;
        Ok(Arc::new(Mutex::new(model)))
    }

    fn is_installed(&self, voice_key: &str) -> bool {
        let voice_dir = self.models_dir.join(voice_key);
        voice_dir.join(format!("{}.onnx", voice_key)).exists()
            && voice_dir.join(format!("{}.onnx.json", voice_key)).exists()
    }

    fn installed_voices(&self) -> Vec<String> {
        std::fs::read_dir(&self.models_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        e.path().join(format!("{}.onnx", name)).exists()
                            && e.path().join(format!("{}.onnx.json", name)).exists()
                    })
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
    fn piper_factory_is_installed_checks_model_and_config() {
        let dir = std::env::temp_dir().join("lisca_piper_test_installed");
        let voice_dir = dir.join("test-voice");
        fs::create_dir_all(&voice_dir).unwrap();
        fs::write(voice_dir.join("test-voice.onnx"), "").unwrap();
        fs::write(voice_dir.join("test-voice.onnx.json"), "{}").unwrap();

        let factory = PiperFactory::new(dir.clone(), PathBuf::from("/tmp"));
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
        fs::write(dir.join("voice-a").join("voice-a.onnx"), "").unwrap();
        fs::write(dir.join("voice-a").join("voice-a.onnx.json"), "{}").unwrap();
        fs::write(dir.join("voice-b").join("voice-b.onnx"), "").unwrap();
        fs::write(dir.join("voice-b").join("voice-b.onnx.json"), "{}").unwrap();
        // voice-c has no config

        let factory = PiperFactory::new(dir.clone(), PathBuf::from("/tmp"));
        let mut voices = factory.installed_voices();
        voices.sort();
        assert_eq!(voices, vec!["voice-a", "voice-b"]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn piper_factory_create_fails_for_missing_model() {
        let dir = std::env::temp_dir().join("lisca_piper_test_missing");
        let factory = PiperFactory::new(dir.clone(), PathBuf::from("/tmp"));
        let result = factory.create("nonexistent");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(dir);
    }
}
