// LRU model cache with configurable max size and auto-unload on idle.
// Models are keyed by voice_key and created on-demand via ModelFactory.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::{Model, ModelFactory};

struct CacheEntry {
    model: Arc<Mutex<dyn Model>>,
    last_used: Instant,
}

pub(crate)  struct ModelPool {
    cache: HashMap<String, CacheEntry>,
    order: VecDeque<String>,
    max_size: usize,
    idle_timeout: Option<Duration>,
}

impl ModelPool {
    pub(crate)  fn new(max_size: usize, idle_timeout: Option<Duration>) -> Self {
        Self {
            cache: HashMap::new(),
            order: VecDeque::new(),
            max_size,
            idle_timeout,
        }
    }

    pub(crate)  async fn get(
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

    pub(crate)  fn evict_expired(&mut self) {
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

            for key in expired {
                self.evict(&key);
            }
        }
    }

    fn evict(&mut self, voice_key: &str) {
        self.cache.remove(voice_key);
        self.order.retain(|k| k != voice_key);
    }


    fn evict_lru(&mut self) {
        if let Some(front) = self.order.front().cloned() {
            self.evict(&front);
        }
    }
}