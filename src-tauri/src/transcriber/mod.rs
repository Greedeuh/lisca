// Language detection and unified model factory for the transcriber actor.

mod language;

pub(super) use language::detect_language_family;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::{Model, ModelFactory};

/// Unified factory that delegates to Piper or Kokoro based on which has the voice installed.
pub(super) struct UnifiedFactory {
    piper: Arc<dyn ModelFactory>,
    kokoro: Arc<dyn ModelFactory>,
}

impl UnifiedFactory {
    pub(super) fn new(piper: Arc<dyn ModelFactory>, kokoro: Arc<dyn ModelFactory>) -> Self {
        Self { piper, kokoro }
    }
}

impl ModelFactory for UnifiedFactory {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
        if self.piper.is_installed(voice_key) {
            self.piper.create(voice_key)
        } else if self.kokoro.is_installed(voice_key) {
            self.kokoro.create(voice_key)
        } else {
            Err(format!(
                "voice '{}' not installed in any backend",
                voice_key
            ))
        }
    }

    fn is_installed(&self, voice_key: &str) -> bool {
        self.piper.is_installed(voice_key) || self.kokoro.is_installed(voice_key)
    }
}
