// LRU model cache with configurable max size and auto-unload on idle.
// Models are keyed by voice_key and created on-demand via ModelFactory.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::{Model, ModelFactory};

pub(crate)  enum ModelEvent {
    Loaded { voice_key: String },
    Unloaded { voice_key: String },
}

struct CacheEntry {
    model: Arc<Mutex<dyn Model>>,
    last_used: Instant,
}

pub(crate)  struct ModelPool {
    cache: HashMap<String, CacheEntry>,
    order: VecDeque<String>,
    max_size: usize,
    idle_timeout: Option<Duration>,
    on_event: Option<Box<dyn Fn(ModelEvent) + Send + Sync>>,
}

impl ModelPool {
    pub(crate)  fn new(max_size: usize, idle_timeout: Option<Duration>) -> Self {
        Self {
            cache: HashMap::new(),
            order: VecDeque::new(),
            max_size,
            idle_timeout,
            on_event: None,
        }
    }

     fn with_event_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(ModelEvent) + Send + Sync + 'static,
    {
        self.on_event = Some(Box::new(handler));
        self
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

        if let Some(ref handler) = self.on_event {
            handler(ModelEvent::Loaded {
                voice_key: voice_key.to_string(),
            });
        }

        Ok(model)
    }

     fn clear_cache(&mut self) {
        let keys: Vec<String> = self.cache.keys().cloned().collect();
        self.cache.clear();
        self.order.clear();

        if let Some(ref handler) = self.on_event {
            for key in keys {
                handler(ModelEvent::Unloaded { voice_key: key });
            }
        }
    }

     fn refresh_installed(&mut self, installed: &[String]) {
        let to_evict: Vec<String> = self
            .cache
            .keys()
            .filter(|k| !installed.contains(k))
            .cloned()
            .collect();

        for key in to_evict {
            self.evict(&key);
        }
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

        if let Some(ref handler) = self.on_event {
            handler(ModelEvent::Unloaded {
                voice_key: voice_key.to_string(),
            });
        }
    }

    fn evict_lru(&mut self) {
        if let Some(front) = self.order.front().cloned() {
            self.evict(&front);
        }
    }

     fn cached_keys(&self) -> Vec<&str> {
        self.order.iter().map(|s| s.as_str()).collect()
    }

     fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockModel {
        sample_rate: u32,
    }

    impl Model for MockModel {
        fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
            Ok(vec![0.1, 0.2, 0.3])
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
    }

    struct MockFactory {
        create_count: Arc<AtomicUsize>,
    }

    impl MockFactory {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let count = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    create_count: count.clone(),
                },
                count,
            )
        }
    }

    impl ModelFactory for MockFactory {
        fn create(&self, _voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
            self.create_count.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(Mutex::new(MockModel { sample_rate: 22050 })))
        }

        fn is_installed(&self, _voice_key: &str) -> bool {
            true
        }

        fn installed_voices(&self) -> Vec<String> {
            vec![
                "voice-a".to_string(),
                "voice-b".to_string(),
                "voice-c".to_string(),
            ]
        }
    }

    #[tokio::test]
    async fn loads_model_on_first_access() {
        let (factory, create_count) = MockFactory::new();
        let mut pool = ModelPool::new(4, None);

        let model = pool.get("voice-a", &factory).await.unwrap();
        assert_eq!(create_count.load(Ordering::SeqCst), 1);
        assert_eq!(pool.cache_size(), 1);
        assert_eq!(pool.cached_keys(), vec!["voice-a"]);

        let m = model.lock().await;
        assert_eq!(m.sample_rate(), 22050);
    }

    #[tokio::test]
    async fn returns_cached_model_on_second_access() {
        let (factory, create_count) = MockFactory::new();
        let mut pool = ModelPool::new(4, None);

        let _ = pool.get("voice-a", &factory).await.unwrap();
        let _ = pool.get("voice-a", &factory).await.unwrap();

        assert_eq!(create_count.load(Ordering::SeqCst), 1);
        assert_eq!(pool.cache_size(), 1);
    }

    #[tokio::test]
    async fn evicts_lru_when_cache_full() {
        let (factory, _) = MockFactory::new();
        let mut pool = ModelPool::new(3, None);

        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();
        let _ = pool.get("c", &factory).await.unwrap();
        assert_eq!(pool.cache_size(), 3);

        // Adding a 4th should evict "a" (LRU)
        let _ = pool.get("d", &factory).await.unwrap();
        assert_eq!(pool.cache_size(), 3);
        assert_eq!(pool.cached_keys(), vec!["b", "c", "d"]);
    }

    #[tokio::test]
    async fn access_refreshes_lru_order() {
        let (factory, _) = MockFactory::new();
        let mut pool = ModelPool::new(3, None);

        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();
        let _ = pool.get("c", &factory).await.unwrap();

        // Access "a" to refresh it
        let _ = pool.get("a", &factory).await.unwrap();
        assert_eq!(pool.cached_keys(), vec!["b", "c", "a"]);

        // Now adding "d" should evict "b" (oldest unused)
        let _ = pool.get("d", &factory).await.unwrap();
        assert_eq!(pool.cached_keys(), vec!["c", "a", "d"]);
    }

    #[tokio::test]
    async fn evicts_expired_models() {
        let (factory, _) = MockFactory::new();
        let mut pool = ModelPool::new(4, Some(Duration::from_millis(10)));

        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(50)).await;

        pool.evict_expired();
        assert_eq!(pool.cache_size(), 0);
        assert!(pool.cached_keys().is_empty());
    }

    #[tokio::test]
    async fn clear_cache_removes_all() {
        let (factory, _) = MockFactory::new();
        let mut pool = ModelPool::new(4, None);

        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();
        assert_eq!(pool.cache_size(), 2);

        pool.clear_cache();
        assert_eq!(pool.cache_size(), 0);
    }

    #[tokio::test]
    async fn refresh_installed_evicts_uninstalled() {
        let (factory, _) = MockFactory::new();
        let mut pool = ModelPool::new(4, None);

        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();
        let _ = pool.get("c", &factory).await.unwrap();

        // Only "a" and "c" are still installed
        pool.refresh_installed(&["a".to_string(), "c".to_string()]);
        assert_eq!(pool.cache_size(), 2);
        assert_eq!(pool.cached_keys(), vec!["a", "c"]);
    }

    #[tokio::test]
    async fn emit_loaded_event() {
        let (factory, _) = MockFactory::new();
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let pool = ModelPool::new(4, None).with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });

        let mut pool = pool;
        let _ = pool.get("voice-a", &factory).await.unwrap();

        let evts = events.lock().unwrap();
        assert_eq!(evts.len(), 1);
        assert!(
            matches!(&evts[0], ModelEvent::Loaded { voice_key } if voice_key == "voice-a")
        );
    }

    #[tokio::test]
    async fn emit_unloaded_event_on_evict() {
        let (factory, _) = MockFactory::new();
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let pool = ModelPool::new(2, None).with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });

        let mut pool = pool;
        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();

        // This should evict "a"
        let _ = pool.get("c", &factory).await.unwrap();

        let evts = events.lock().unwrap();
        let loaded: Vec<_> = evts
            .iter()
            .filter(|e| matches!(e, ModelEvent::Loaded { .. }))
            .collect();
        let unloaded: Vec<_> = evts
            .iter()
            .filter(|e| matches!(e, ModelEvent::Unloaded { .. }))
            .collect();
        assert_eq!(loaded.len(), 3);
        assert_eq!(unloaded.len(), 1);
        assert!(
            matches!(&unloaded[0], ModelEvent::Unloaded { voice_key } if voice_key == "a")
        );
    }

    #[tokio::test]
    async fn emit_unloaded_on_clear_cache() {
        let (factory, _) = MockFactory::new();
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let pool = ModelPool::new(4, None).with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });

        let mut pool = pool;
        let _ = pool.get("a", &factory).await.unwrap();
        let _ = pool.get("b", &factory).await.unwrap();

        pool.clear_cache();

        let evts = events.lock().unwrap();
        let unloaded: Vec<_> = evts
            .iter()
            .filter(|e| matches!(e, ModelEvent::Unloaded { .. }))
            .collect();
        assert_eq!(unloaded.len(), 2);
    }

    #[tokio::test]
    async fn factory_error_propagates() {
        struct FailingFactory;

        impl ModelFactory for FailingFactory {
            fn create(&self, _voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String> {
                Err("model not found".to_string())
            }

            fn is_installed(&self, _voice_key: &str) -> bool {
                false
            }

            fn installed_voices(&self) -> Vec<String> {
                vec![]
            }
        }

        let mut pool = ModelPool::new(4, None);
        let result = pool.get("missing", &FailingFactory).await;
        match result {
            Err(e) => assert_eq!(e, "model not found"),
            Ok(_) => panic!("expected error"),
        }
        assert_eq!(pool.cache_size(), 0);
    }

    #[tokio::test]
    async fn max_size_one_evicts_previous() {
        let (factory, _) = MockFactory::new();
        let mut pool = ModelPool::new(1, None);

        let _ = pool.get("a", &factory).await.unwrap();
        assert_eq!(pool.cached_keys(), vec!["a"]);

        let _ = pool.get("b", &factory).await.unwrap();
        assert_eq!(pool.cached_keys(), vec!["b"]);
        assert_eq!(pool.cache_size(), 1);
    }
}
