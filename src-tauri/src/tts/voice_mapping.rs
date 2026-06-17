/// TODO: Currently Piper-specific, but could be generalized for other model backends later.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::persist;

const VOICE_MAPPING_FILE: &str = "voice_mapping.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoiceMapping {
    pub language_voice: HashMap<String, String>,
    pub fallback_voice_key: Option<String>,
}

impl VoiceMapping {
    pub fn resolve(&self, language: Option<&str>) -> Option<&str> {
        language
            .and_then(|l| self.language_voice.get(l))
            .map(|s| s.as_str())
            .or(self.fallback_voice_key.as_deref())
    }
}

pub fn voice_mapping_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("lisca").join(VOICE_MAPPING_FILE)
}

pub fn load(app_data_dir: &Path) -> VoiceMapping {
    let path = voice_mapping_path(app_data_dir);
    persist::load_json(&path)
}

pub fn save(app_data_dir: &Path, mapping: &VoiceMapping) -> Result<(), String> {
    let path = voice_mapping_path(app_data_dir);
    persist::save_json(&path, mapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_language() {
        let mut m = VoiceMapping::default();
        m.language_voice.insert("en".into(), "en_US-lessac".into());
        m.language_voice.insert("fr".into(), "fr_FR-siwis".into());
        assert_eq!(m.resolve(Some("en")), Some("en_US-lessac"));
        assert_eq!(m.resolve(Some("fr")), Some("fr_FR-siwis"));
    }

    #[test]
    fn resolve_unknown_language_falls_back_to_default() {
        let mut m = VoiceMapping::default();
        m.language_voice.insert("en".into(), "en_US-lessac".into());
        m.fallback_voice_key = Some("en_US-lessac".into());
        assert_eq!(m.resolve(Some("de")), Some("en_US-lessac"));
    }

    #[test]
    fn resolve_nothing_returns_none() {
        let m = VoiceMapping::default();
        assert_eq!(m.resolve(Some("en")), None);
        assert_eq!(m.resolve(None), None);
    }

    #[test]
    fn resolve_none_language_uses_fallback() {
        let mut m = VoiceMapping::default();
        m.fallback_voice_key = Some("en_US-lessac".into());
        assert_eq!(m.resolve(None), Some("en_US-lessac"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut m = VoiceMapping::default();
        m.language_voice.insert("en".into(), "en_US-lessac".into());
        m.fallback_voice_key = Some("en_US-lessac".into());
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: VoiceMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.language_voice.len(), 1);
        assert_eq!(deserialized.fallback_voice_key, Some("en_US-lessac".into()));
    }

    #[test]
    fn save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = VoiceMapping::default();
        m.language_voice.insert("en".into(), "en_US-lessac".into());
        m.fallback_voice_key = Some("en_US-lessac".into());
        save(dir.path(), &m).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.language_voice.len(), 1);
        assert_eq!(loaded.fallback_voice_key, Some("en_US-lessac".into()));
    }

    #[test]
    fn load_missing_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load(dir.path());
        assert!(loaded.language_voice.is_empty());
        assert!(loaded.fallback_voice_key.is_none());
    }
}
