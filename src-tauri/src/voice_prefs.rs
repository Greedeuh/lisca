// Per-language active voice selection with fallback.
// Resolves a detected language to a voice key for the transcriber.

use crate::persist::{load_json, save_json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
                .or(self.fallback_voice_key.as_deref()),
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

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("voice_mapping.json");

        let mut mapping = VoiceMapping::default();
        mapping
            .language_voice
            .insert("en".to_string(), "en-us-voice".to_string());
        mapping.fallback_voice_key = Some("default-voice".to_string());
        mapping.save(&path).unwrap();

        let loaded = VoiceMapping::load(&path);
        assert_eq!(loaded.language_voice.get("en").unwrap(), "en-us-voice");
        assert_eq!(loaded.fallback_voice_key.as_deref(), Some("default-voice"));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = std::env::temp_dir().join("nonexistent_voice_mapping.json");
        let loaded = VoiceMapping::load(&path);
        assert_eq!(loaded, VoiceMapping::default());
    }
}
