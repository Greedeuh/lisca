// Per-language active voice selection with fallback.
// Resolves a detected language to a voice key for the transcriber.

use crate::persist::{load_json, save_json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoiceMapping {
    pub language_voice: HashMap<String, String>,
    pub fallback_voice_key: Option<String>,
}

impl VoiceMapping {
    pub fn resolve(&self, language: Option<&str>) -> Option<&str> {
        match language {
            Some(lang) => self
                .language_voice
                .get(lang)
                .map(|s| s.as_str())
                .or_else(|| self.fallback_voice_key.as_deref()),
            None => self.fallback_voice_key.as_deref(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        save_json(path, self)
    }

    pub fn load(path: &Path) -> Self {
        load_json(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_language() {
        let mut mapping = VoiceMapping::default();
        mapping
            .language_voice
            .insert("en".to_string(), "en-us".to_string());
        assert_eq!(mapping.resolve(Some("en")), Some("en-us"));
    }

    #[test]
    fn resolve_unknown_with_fallback() {
        let mut mapping = VoiceMapping::default();
        mapping
            .language_voice
            .insert("en".to_string(), "en-us".to_string());
        mapping.fallback_voice_key = Some("default-voice".to_string());
        assert_eq!(mapping.resolve(Some("de")), Some("default-voice"));
    }

    #[test]
    fn resolve_unknown_no_fallback() {
        let mapping = VoiceMapping::default();
        assert_eq!(mapping.resolve(Some("de")), None);
    }

    #[test]
    fn resolve_none_with_fallback() {
        let mut mapping = VoiceMapping::default();
        mapping.fallback_voice_key = Some("default-voice".to_string());
        assert_eq!(mapping.resolve(None), Some("default-voice"));
    }

    #[test]
    fn resolve_none_no_fallback() {
        let mapping = VoiceMapping::default();
        assert_eq!(mapping.resolve(None), None);
    }
}
