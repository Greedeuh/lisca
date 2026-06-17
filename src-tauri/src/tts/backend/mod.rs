mod voice_resolver;

use std::collections::{HashMap, VecDeque};

use super::piper::InstalledModel;

pub(crate) use voice_resolver::VoiceResolver;

pub(crate) const DEFAULT_SAMPLE_RATE: u32 = 24000;
pub(crate) const MAX_CACHED_BACKENDS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveBackend {
    Piper,
    Kokoro,
}

pub trait TtsBackend: Send {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String>;
    fn sample_rate(&self) -> u32;
}

pub(crate) trait BackendFactory: Send + Sync {
    fn create_from_installed(
        &self,
        model: &InstalledModel,
    ) -> Result<Box<dyn TtsBackend>, String>;
}

pub(crate) enum AudioChunk {
    Samples(Vec<f32>),
    Eof,
}

pub(crate) struct BackendPool {
    pub primary: Option<Box<dyn TtsBackend>>,
    cache: HashMap<String, Box<dyn TtsBackend>>,
    installed: Vec<InstalledModel>,
    lru_order: VecDeque<String>,
    pub factory: Box<dyn BackendFactory>,
    active_backend: ActiveBackend,
}

impl BackendPool {
    pub fn new(
        factory: Box<dyn BackendFactory>,
        active_backend: ActiveBackend,
    ) -> Self {
        Self {
            primary: None,
            cache: HashMap::new(),
            installed: Vec::new(),
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

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
    }

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

    pub fn get_for_language(&mut self, voice_key: Option<&str>) -> &mut dyn TtsBackend {
        if self.active_backend == ActiveBackend::Piper {
            if let Some(key) = voice_key {
                if self.ensure_cached(key) {
                    return &mut **self.cache.get_mut(key).unwrap();
                }
            }
        }
        &mut **self.primary.as_mut().unwrap()
    }

    pub fn sample_rate_for_language(&self, voice_key: Option<&str>) -> u32 {
        if let Some(key) = voice_key {
            if let Some(backend) = self.cache.get(key) {
                return backend.sample_rate();
            }
        }
        self.primary.as_ref().map(|b| b.sample_rate()).unwrap_or(DEFAULT_SAMPLE_RATE)
    }

    fn load_by_voice_key(&mut self, voice_key: &str) -> bool {
        let model = match self.installed.iter().find(|m| m.voice_key == voice_key) {
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

    fn touch_lru(&mut self, voice_key: &str) {
        self.lru_order.retain(|k| k != voice_key);
        self.lru_order.push_back(voice_key.to_string());
    }

    fn evict_if_full(&mut self) {
        while self.cache.len() >= MAX_CACHED_BACKENDS {
            if let Some(oldest) = self.lru_order.pop_front() {
                self.cache.remove(&oldest);
                eprintln!("Evicted cached backend: {}", oldest);
            } else {
                break;
            }
        }
    }

    pub fn refresh_installed(&mut self, models: Vec<InstalledModel>) {
        let valid_keys: std::collections::HashSet<&str> =
            models.iter().map(|m| m.voice_key.as_str()).collect();

        self.cache.retain(|key, _| valid_keys.contains(key.as_str()));
        self.lru_order.retain(|key| valid_keys.contains(key.as_str()));

        self.installed = models;
    }
}
