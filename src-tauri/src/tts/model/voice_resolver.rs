use super::super::piper::InstalledModel;
use super::super::voice_mapping::VoiceMapping;

/// Resolves a language family (e.g. "en", "fr") to a concrete voice key
/// for the active TTS model. First checks the user's explicit voice mapping,
/// then falls back to the first installed model matching that language.
pub(crate) struct VoiceResolver {
    mapping: VoiceMapping,
    installed: Vec<InstalledModel>,
}

impl VoiceResolver {
    pub fn new(mapping: VoiceMapping) -> Self {
        Self {
            mapping,
            installed: Vec::new(),
        }
    }

    pub fn resolve_voice_key(&self, language: Option<&str>) -> Option<String> {
        if let Some(key) = self.mapping.resolve(language) {
            return Some(key.to_string());
        }
        language.and_then(|lang| {
            self.installed
                .iter()
                .find(|m| m.language.family == lang)
                .map(|m| m.voice_key.clone())
        })
    }

    pub fn set_mapping(&mut self, mapping: VoiceMapping) {
        self.mapping = mapping;
    }

    pub fn refresh_installed(&mut self, models: Vec<InstalledModel>) {
        self.installed = models;
    }
}
