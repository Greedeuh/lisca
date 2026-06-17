mod voice_resolver;

use std::collections::{HashMap, VecDeque};

use super::piper::InstalledModel;

pub(crate) use voice_resolver::VoiceResolver;

pub(crate) const DEFAULT_SAMPLE_RATE: u32 = 24000;
pub(crate) const MAX_CACHED_MODELS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveBackend {
    Piper,
    Kokoro,
}

/// Trait for TTS model backends. Each implementation (Piper, Kokoro) provides
/// text-to-audio synthesis and reports its native sample rate.
pub trait TtsModel: Send {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String>;
    fn sample_rate(&self) -> u32;
}

/// Factory trait for creating model instances from installed model metadata.
pub(crate) trait ModelFactory: Send + Sync {
    fn create_from_installed(
        &self,
        model: &InstalledModel,
    ) -> Result<Box<dyn TtsModel>, String>;
}

/// Manages TTS model instances. Holds a primary (preloaded) model and an LRU
/// cache of additional voice models. When a request targets a specific voice
/// that isn't primary, the pool loads it into cache (evicting the oldest if
/// the cache is full).
pub(crate) struct ModelPool {
    /// The preloaded default model, set at startup or when the user changes
    /// the active backend in settings.
    pub primary: Option<Box<dyn TtsModel>>,
    /// LRU cache of voice-key → model, for Piper voices accessed on demand.
    cache: HashMap<String, Box<dyn TtsModel>>,
    /// All models currently installed on disk (refreshed by PiperCatalog).
    installed_models: Vec<InstalledModel>,
    /// Tracks access order for LRU eviction: most-recently-used at the back.
    lru_order: VecDeque<String>,
    pub factory: Box<dyn ModelFactory>,
    /// Which backend type (Piper or Kokoro) is currently active.
    active_backend: ActiveBackend,
}

impl ModelPool {
    pub fn new(
        factory: Box<dyn ModelFactory>,
        active_backend: ActiveBackend,
    ) -> Self {
        Self {
            primary: None,
            cache: HashMap::new(),
            installed_models: Vec::new(),
            lru_order: VecDeque::new(),
            factory,
            active_backend,
        }
    }

    pub fn set_active_backend(&mut self, backend: ActiveBackend) {
        self.active_backend = backend;
        self.cache.clear();
        self.lru_order.clear();
    }

    /// Drops all cached voice models and resets LRU state. Called when the
    /// user changes voice mapping or switches backends.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
    }

    /// Ensures the given voice_key is loaded into the cache. Returns true if
    /// the cache now contains the model (either it was already there, or we
    /// loaded it from disk).
    fn ensure_cached(&mut self, voice_key: &str) -> bool {
        if self.cache.contains_key(voice_key) {
            self.touch_lru(voice_key);
            return true;
        }
        if self.load_by_voice_key(voice_key) {
            self.touch_lru(voice_key);
            return true;
        }
        false
    }

    /// Returns a mutable reference to the TTS model for the given voice.
    /// For Piper, resolves the voice key and loads from cache if needed.
    /// For other backends, returns the primary model.
    pub fn get_model_for_language(&mut self, voice_key: Option<&str>) -> &mut dyn TtsModel {
        if self.active_backend == ActiveBackend::Piper {
            if let Some(key) = voice_key {
                if self.ensure_cached(key) {
                    return &mut **self.cache.get_mut(key).unwrap();
                }
            }
        }
        &mut **self.primary.as_mut().unwrap()
    }

    /// Returns the sample rate for the given voice's model, or the default
    /// rate if no voice-specific model is cached.
    pub fn sample_rate_for_language(&self, voice_key: Option<&str>) -> u32 {
        if let Some(key) = voice_key {
            if let Some(backend) = self.cache.get(key) {
                return backend.sample_rate();
            }
        }
        self.primary.as_ref().map(|b| b.sample_rate()).unwrap_or(DEFAULT_SAMPLE_RATE)
    }

    fn load_by_voice_key(&mut self, voice_key: &str) -> bool {
        let model = match self.installed_models.iter().find(|m| m.voice_key == voice_key) {
            Some(m) => m.clone(),
            None => return false,
        };

        let mp = std::path::PathBuf::from(&model.model_path);
        let cp = std::path::PathBuf::from(&model.config_path);

        if !mp.exists() || !cp.exists() {
            return false;
        }

        eprintln!("Loading model: {} ({})", voice_key, model.name);
        let start = std::time::Instant::now();

        match self.factory.create_from_installed(&model) {
            Ok(m) => {
                eprintln!(
                    "Model {} loaded in {}ms",
                    voice_key,
                    start.elapsed().as_millis()
                );
                self.evict_if_full();
                self.cache.insert(voice_key.to_string(), m);
                true
            }
            Err(e) => {
                eprintln!("Failed to load model {}: {}", voice_key, e);
                false
            }
        }
    }

    /// Updates the LRU order: moves voice_key to the back (most-recently-used).
    fn touch_lru(&mut self, voice_key: &str) {
        self.lru_order.retain(|k| k != voice_key);
        self.lru_order.push_back(voice_key.to_string());
    }

    /// Evicts the least-recently-used cached model when the cache is at capacity.
    fn evict_if_full(&mut self) {
        while self.cache.len() >= MAX_CACHED_MODELS {
            if let Some(oldest) = self.lru_order.pop_front() {
                self.cache.remove(&oldest);
                eprintln!("Evicted cached model: {}", oldest);
            } else {
                break;
            }
        }
    }

    /// Syncs the installed model list and evicts any cached models that are
    /// no longer installed on disk.
    pub fn refresh_installed(&mut self, models: Vec<InstalledModel>) {
        let valid_keys: std::collections::HashSet<&str> =
            models.iter().map(|m| m.voice_key.as_str()).collect();

        self.cache.retain(|key, _| valid_keys.contains(key.as_str()));
        self.lru_order.retain(|key| valid_keys.contains(key.as_str()));

        self.installed_models = models;
    }
}
