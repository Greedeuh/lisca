use std::collections::HashMap;
use std::path::Path;

#[derive(serde::Deserialize)]
pub struct KokoroTokenizerConfig {
    pub post_processor: PostProcessorConfig,
    pub model: ModelConfig,
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
