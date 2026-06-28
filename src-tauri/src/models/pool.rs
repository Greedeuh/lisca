// LRU model cache with configurable max size and auto-unload on idle.
// Models are keyed by voice_key and created on-demand via ModelFactory.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::persist::{load_json, save_json};

use super::{Model, ModelFactory};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct PoolConfig {
    idle_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 300,
        }
    }
}

struct CacheEntry {
    model: Arc<Mutex<dyn Model>>,
    last_used: Instant,
}

pub(crate) struct ModelPool {
    cache: HashMap<String, CacheEntry>,
    order: VecDeque<String>,
    max_size: usize,
    idle_timeout: Option<Duration>,
    config_path: Option<PathBuf>,
}

impl ModelPool {
    pub(crate) fn new(max_size: usize, idle_timeout: Option<Duration>) -> Self {
        Self {
            cache: HashMap::new(),
            order: VecDeque::new(),
            max_size,
            idle_timeout,
            config_path: None,
        }
    }

    pub(crate) fn with_config_path(mut self, path: PathBuf) -> Self {
        let config: PoolConfig = load_json(&path);
        self.idle_timeout = Some(Duration::from_secs(config.idle_timeout_secs));
        self.config_path = Some(path);
        self
    }

    pub(crate) fn idle_timeout_secs(&self) -> u64 {
        self.idle_timeout
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub(crate) fn set_idle_timeout(&mut self, timeout: Option<Duration>) {
        self.idle_timeout = timeout;
        if let Some(ref path) = self.config_path {
            let config = PoolConfig {
                idle_timeout_secs: timeout.map(|d| d.as_secs()).unwrap_or(0),
            };
            if let Err(e) = save_json(path, &config) {
                log::error!("Failed to save pool config: {e}");
            }
        }
    }

    pub(crate) async fn get(
        &mut self,
        voice_key: &str,
        factory: &dyn ModelFactory,
    ) -> Result<Arc<Mutex<dyn Model>>, String> {
        if let Some(entry) = self.cache.get_mut(voice_key) {
            entry.last_used = Instant::now();
            self.order.retain(|k| k != voice_key);
            self.order.push_back(voice_key.to_string());
            return Ok(entry.model.clone());
        }

        while self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        log::debug!("Creating model for voice: {voice_key}");
        let model = factory.create(voice_key)?;
        log::info!("Model loaded: {voice_key}");

        self.cache.insert(
            voice_key.to_string(),
            CacheEntry {
                model: model.clone(),
                last_used: Instant::now(),
            },
        );
        self.order.push_back(voice_key.to_string());

        Ok(model)
    }

    pub(crate) fn evict_expired(&mut self) {
        if let Some(timeout) = self.idle_timeout {
            let now = Instant::now();
            let expired: Vec<String> = self
                .order
                .iter()
                .filter(|k| {
                    self.cache
                        .get(*k)
                        .map(|e| now.duration_since(e.last_used) > timeout)
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if !expired.is_empty() {
                log::info!("Evicting {} idle model(s) (timeout: {}s)", expired.len(), timeout.as_secs());
            }
            for key in expired {
                self.evict(&key);
            }
        }
    }

    fn evict(&mut self, voice_key: &str) {
        log::info!("Model unloaded: {voice_key}");
        self.cache.remove(voice_key);
        self.order.retain(|k| k != voice_key);
    }

    fn evict_lru(&mut self) {
        if let Some(front) = self.order.front().cloned() {
            self.evict(&front);
        }
    }
}
