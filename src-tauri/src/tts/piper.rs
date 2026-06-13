use std::collections::HashMap;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

use super::TtsBackend;

static ESPEAK_INITIALIZED: std::sync::Once = std::sync::Once::new();

fn ensure_espeak_data(resource_dir: &Path) {
    ESPEAK_INITIALIZED.call_once(|| {
        let data_dir = resource_dir.join("espeak-ng-data");
        if !data_dir.exists() {
            if let Err(e) = espeak_ng::install_bundled_languages(&data_dir, &["en"]) {
                eprintln!("Failed to install espeak-ng data: {}", e);
            }
        }
        std::env::set_var("ESPEAK_DATA_PATH", &data_dir);
    });
}

#[derive(serde::Deserialize)]
struct PiperConfig {
    #[serde(default)]
    inference: PiperInference,
    #[serde(default)]
    audio: PiperAudio,
    phoneme_id_map: HashMap<String, Vec<i64>>,
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
        ensure_espeak_data(resource_dir);

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

        let session = super::session::create_session(model_path)
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
        let ipa = match espeak_ng::text_to_ipa("en", text) {
            Ok(ipa) => ipa,
            Err(e) => {
                eprintln!("espeak IPA error: {}", e);
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
