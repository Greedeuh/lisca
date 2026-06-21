use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{LazyLock, Once};
use unicode_normalization::UnicodeNormalization;

static INSTALLED_LANGUAGES: LazyLock<std::sync::Mutex<HashSet<String>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));
static DATA_DIR_SET: Once = Once::new();

fn ensure_espeak_data(resource_dir: &Path, lang_code: &str) {
    let data_dir = resource_dir.join("espeak-ng-data");
    DATA_DIR_SET.call_once(|| {
        std::env::set_var("ESPEAK_DATA_PATH", &data_dir);
    });
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
pub struct KokoroTokenizerConfig {
    pub normalizer: NormalizerConfig,
    pub post_processor: PostProcessorConfig,
    pub model: ModelConfig,
}

#[derive(serde::Deserialize)]
pub struct NormalizerConfig {
    pub pattern: PatternConfig,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
pub struct PatternConfig {
    pub Regex: String,
}

#[derive(serde::Deserialize)]
pub struct PostProcessorConfig {
    pub special_tokens: HashMap<String, SpecialToken>,
}

#[derive(serde::Deserialize)]
pub struct SpecialToken {
    #[allow(dead_code)]
    pub id: String,
    pub ids: Vec<i64>,
}

#[derive(serde::Deserialize)]
pub struct ModelConfig {
    pub vocab: HashMap<String, i64>,
}

impl KokoroTokenizerConfig {
    pub fn load(resource_dir: &Path) -> Result<Self, String> {
        let config_path = resource_dir.join("kokoro_tokenizer.json");
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Read tokenizer config: {}", e))?;
        serde_json::from_str(&config_str)
            .map_err(|e| format!("Parse tokenizer config: {}", e))
    }

    pub fn build_tokenizer(&self) -> (HashMap<char, i64>, i64) {
        let mut vocab = HashMap::new();
        for (key, &id) in &self.model.vocab {
            if let Some(ch) = key.chars().next() {
                vocab.insert(ch, id);
            }
        }

        let pad_token_id = self
            .post_processor
            .special_tokens
            .get("$")
            .and_then(|t| t.ids.first())
            .copied()
            .unwrap_or(0);

        (vocab, pad_token_id)
    }
}

pub struct KokoroPhonemizer {
    lang_code: String,
}

impl KokoroPhonemizer {
    pub fn new(resource_dir: &Path, lang_code: &str) -> Self {
        ensure_espeak_data(resource_dir, lang_code);
        Self {
            lang_code: lang_code.to_string(),
        }
    }

    pub fn phonemize(&self, text: &str) -> String {
        match espeak_ng::text_to_ipa(&self.lang_code, text) {
            Ok(ipa) => ipa.nfd().collect(),
            Err(e) => {
                log::warn!("espeak IPA error for {}: {}", self.lang_code, e);
                String::new()
            }
        }
    }
}
