mod language;
pub mod piper;
pub mod kokoro;
pub mod audio;
pub mod commands;
pub mod config;
pub mod playback;
pub mod processor;
pub mod queue;
pub mod queue_manager;
pub mod text;
pub mod voice_mapping;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;

use self::config::BackendConfig;
use self::playback::{PlaybackController, STATE_IDLE, STATE_PAUSED, STATE_PLAYING};
use self::piper::InstalledModel;
use self::queue::{QueueConfig, QueueEvent, QueueItem, QueueSnapshot};
use self::queue_manager::QueueManager;
use self::voice_mapping::VoiceMapping;

const AUDIO_CHANNEL_BUFFER: usize = 8;
const DEFAULT_SAMPLE_RATE: u32 = 24000;
const I16_SAMPLE_SCALE: f32 = 32767.0;
const MAX_CACHED_BACKENDS: usize = 4;

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
    fn load(
        &self,
        model_path: &str,
        config_path: &str,
        resource_dir: &Path,
    ) -> Result<Box<dyn TtsBackend>, String>;
}

enum AudioChunk {
    Samples(Vec<f32>),
    Eof,
}

pub(crate) struct BackendPool {
    primary: Option<Box<dyn TtsBackend>>,
    cache: HashMap<String, Box<dyn TtsBackend>>,
    installed: Vec<InstalledModel>,
    mapping: VoiceMapping,
    resource_dir: PathBuf,
    lru_order: VecDeque<String>,
    factory: Box<dyn BackendFactory>,
    active_backend: ActiveBackend,
}

impl BackendPool {
    fn new(
        resource_dir: PathBuf,
        mapping: VoiceMapping,
        factory: Box<dyn BackendFactory>,
        active_backend: ActiveBackend,
    ) -> Self {
        Self {
            primary: None,
            cache: HashMap::new(),
            installed: Vec::new(),
            mapping,
            resource_dir,
            lru_order: VecDeque::new(),
            factory,
            active_backend,
        }
    }

    fn set_active_backend(&mut self, backend: ActiveBackend) {
        self.active_backend = backend;
        self.cache.clear();
        self.lru_order.clear();
    }

    fn resolve_voice_key(&self, language: Option<&str>) -> Option<String> {
        if let Some(key) = self.mapping.resolve(language) {
            return Some(key.to_string());
        }
        language.and_then(|lang| {
            self.installed.iter()
                .find(|m| m.language.family == lang)
                .map(|m| m.voice_key.clone())
        })
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

    fn get_for_language(&mut self, language: Option<&str>) -> &mut dyn TtsBackend {
        if self.active_backend == ActiveBackend::Piper {
            if let Some(key) = self.resolve_voice_key(language) {
                if self.ensure_cached(&key) {
                    return &mut **self.cache.get_mut(&key).unwrap();
                }
            }
        }
        &mut **self.primary.as_mut().unwrap()
    }

    fn get_for_text(&mut self, text: &str) -> &mut dyn TtsBackend {
        let lang = language::detect_language_family(text);
        self.get_for_language(lang)
    }

    fn sample_rate_for_text(&self, text: &str) -> u32 {
        let lang = language::detect_language_family(text);
        self.sample_rate_for_language(lang)
    }

    fn sample_rate_for_language(&self, language: Option<&str>) -> u32 {
        if let Some(key) = self.resolve_voice_key(language) {
            if let Some(backend) = self.cache.get(&key) {
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

        match self
            .factory
            .load(&model.model_path, &model.config_path, &self.resource_dir)
        {
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

    fn set_mapping(&mut self, mapping: VoiceMapping) {
        self.mapping = mapping;
        self.cache.clear();
        self.lru_order.clear();
    }

    fn refresh_installed(&mut self, models: Vec<InstalledModel>) {
        let valid_keys: std::collections::HashSet<&str> =
            models.iter().map(|m| m.voice_key.as_str()).collect();

        self.cache.retain(|key, _| valid_keys.contains(key.as_str()));
        self.lru_order.retain(|key| valid_keys.contains(key.as_str()));

        self.installed = models;
    }
}

pub struct TtsManager {
    pool: Arc<std::sync::Mutex<BackendPool>>,
    pub(crate) app_data_dir: PathBuf,
    pub(crate) resource_dir: PathBuf,
    queue_mgr: QueueManager,
    pub(crate) app_handle: AppHandle,
    playback: PlaybackController,
    processor_running: Arc<AtomicBool>,
}

impl TtsManager {
    pub fn new(app_data_dir: PathBuf, resource_dir: PathBuf, app_handle: AppHandle) -> Self {
        let mapping = voice_mapping::load(&app_data_dir);
        let config = config::load_config(&app_data_dir);
        let (factory, active_backend) = match &config {
            BackendConfig::Kokoro => {
                let model_dir = BackendConfig::kokoro_model_dir(&app_data_dir);
                let factory: Box<dyn BackendFactory> = Box::new(kokoro::KokoroBackendFactory { model_dir });
                (factory, ActiveBackend::Kokoro)
            }
            BackendConfig::Piper { .. } => {
                let factory: Box<dyn BackendFactory> = Box::new(piper::PiperBackendFactory);
                (factory, ActiveBackend::Piper)
            }
        };

        Self {
            pool: Arc::new(std::sync::Mutex::new(BackendPool::new(
                resource_dir.clone(),
                mapping,
                factory,
                active_backend,
            ))),
            app_data_dir: app_data_dir.clone(),
            resource_dir,
            queue_mgr: QueueManager::new(app_data_dir),
            app_handle,
            playback: PlaybackController::new(),
            processor_running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn load_backend_from_config(
        config: &BackendConfig,
        resource_dir: &std::path::Path,
        app_data_dir: &std::path::Path,
    ) -> Option<Box<dyn TtsBackend>> {
        match config {
            BackendConfig::Piper {
                model_path,
                config_path,
            } => {
                let mp = BackendConfig::resolve_path(model_path, resource_dir);
                let cp = BackendConfig::resolve_path(config_path, resource_dir);

                if !mp.exists() || !cp.exists() {
                    eprintln!("Piper model files not found: {:?}, {:?}", mp, cp);
                    return None;
                }

                eprintln!("Preloading Piper model...");
                let start = std::time::Instant::now();

                match piper::PiperModel::load(&mp, &cp, resource_dir) {
                    Ok(model) => {
                        eprintln!(
                            "Piper model preloaded in {}ms",
                            start.elapsed().as_millis()
                        );
                        Some(Box::new(model))
                    }
                    Err(e) => {
                        eprintln!("Preload failed: {}", e);
                        None
                    }
                }
            }
            BackendConfig::Kokoro => {
                let model_dir = BackendConfig::kokoro_model_dir(app_data_dir);
                let model = model_dir.join("model_q8f16.onnx");
                let voice = model_dir.join("af.bin");
                let tokenizer = model_dir.join("tokenizer.json");

                if !model.exists() || !voice.exists() || !tokenizer.exists() {
                    eprintln!("Kokoro files not found: {:?}", model_dir);
                    return None;
                }

                eprintln!("Preloading Kokoro model...");
                let start = std::time::Instant::now();

                match kokoro::KokoroModel::load(&model, &voice, &tokenizer) {
                    Ok(m) => {
                        eprintln!(
                            "Kokoro model preloaded in {}ms",
                            start.elapsed().as_millis()
                        );
                        Some(Box::new(m))
                    }
                    Err(e) => {
                        eprintln!("Preload failed: {}", e);
                        None
                    }
                }
            }
        }
    }

    pub fn preload(&self) {
        let config = config::load_config(&self.app_data_dir);

        if self.pool.try_lock().map(|p| p.primary.is_some()).unwrap_or(true) {
            return;
        }

        let pool = self.pool.clone();
        let base_dir = self.resource_dir.clone();
        let app_data = self.app_data_dir.clone();

        std::thread::spawn(move || {
            let new_backend = Self::load_backend_from_config(&config, &base_dir, &app_data);
            pool.lock().unwrap().primary = new_backend;
        });
    }

    pub fn set_backend(&self, config: BackendConfig) -> Result<(), String> {
        self.stop();

        let new_backend = Self::load_backend_from_config(&config, &self.resource_dir, &self.app_data_dir)
            .ok_or("Failed to load backend")?;

        let (factory, active_backend) = match &config {
            BackendConfig::Kokoro => {
                let model_dir = BackendConfig::kokoro_model_dir(&self.app_data_dir);
                let factory: Box<dyn BackendFactory> = Box::new(kokoro::KokoroBackendFactory { model_dir });
                (factory, ActiveBackend::Kokoro)
            }
            BackendConfig::Piper { .. } => {
                let factory: Box<dyn BackendFactory> = Box::new(piper::PiperBackendFactory);
                (factory, ActiveBackend::Piper)
            }
        };

        let mut pool = self.pool.lock().unwrap();
        pool.primary = Some(new_backend);
        pool.factory = factory;
        pool.set_active_backend(active_backend);
        config::save_config(&self.app_data_dir, &config)?;
        Ok(())
    }

    pub fn get_config(&self) -> BackendConfig {
        config::load_config(&self.app_data_dir)
    }

    pub fn refresh_installed_models(&self, models: Vec<InstalledModel>) {
        self.pool.lock().unwrap().refresh_installed(models);
    }

    pub fn get_voice_mapping(&self) -> VoiceMapping {
        voice_mapping::load(&self.app_data_dir)
    }

    pub fn set_voice_mapping(&self, mapping: VoiceMapping) -> Result<(), String> {
        voice_mapping::save(&self.app_data_dir, &mapping)?;
        self.pool.lock().unwrap().set_mapping(mapping);
        Ok(())
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioChunk>(AUDIO_CHANNEL_BUFFER);

        let pool = self.pool.clone();
        let text_clone = text.to_string();
        let synth_handle = tokio::task::spawn_blocking(move || {
            let mut pool_guard = pool.lock().unwrap();
            let model = pool_guard.get_for_text(&text_clone);

            let chunks = text::split_text(&text_clone);
            for chunk in &chunks {
                match model.synthesize(chunk, 1.0) {
                    Ok(audio) => {
                        if audio_tx.blocking_send(AudioChunk::Samples(audio)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Synthesis error: {}", e);
                        break;
                    }
                }
            }
            drop(pool_guard);
            let _ = audio_tx.blocking_send(AudioChunk::Eof);
        });

        let sample_rate = {
            let guard = self.pool.lock().unwrap();
            guard.sample_rate_for_text(text)
        };

        let play_handle = tokio::task::spawn_blocking(move || {
            let output = match audio::AudioOutput::try_new() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };

            loop {
                let chunk = audio_rx.blocking_recv();
                match chunk {
                    Some(AudioChunk::Samples(samples)) => {
                        let i16_samples = audio::f32_to_i16(&samples);
                        output.play_buffer(i16_samples, sample_rate);
                    }
                    _ => break,
                }
            }

            output.sleep_until_end();
        });

        let _ = synth_handle.await;
        let _ = play_handle.await;

        Ok(())
    }

    pub fn stop(&self) {
        self.playback.stop();
    }

    // --- Queue methods ---

    pub async fn queue_add(&self, text: String) -> Result<QueueItem, String> {
        let was_empty = self.queue_mgr.is_empty().await;
        let item = self.queue_mgr.add(text).await?;

        let config = self.queue_mgr.get_config();
        let q = self.queue_mgr.queue_arc();
        let items: Vec<QueueItem> = q.lock().await.iter().cloned().collect();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });

        if was_empty {
            self.spawn_processor_if_needed();
            self.playback.notify().notify_one();
        }

        self.sync_overlay(true);

        Ok(item)
    }

    pub async fn queue_remove(&self, id: u32) {
        self.queue_mgr.remove(id).await;
        let config = self.queue_mgr.get_config();
        let q = self.queue_mgr.queue_arc();
        let items: Vec<QueueItem> = q.lock().await.iter().cloned().collect();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    pub async fn queue_move(&self, id: u32, new_index: usize) {
        self.queue_mgr.move_item(id, new_index).await;
        let config = self.queue_mgr.get_config();
        let q = self.queue_mgr.queue_arc();
        let items: Vec<QueueItem> = q.lock().await.iter().cloned().collect();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    pub async fn queue_clear(&self) {
        self.playback.stop();
        self.queue_mgr.clear().await;
        let config = self.queue_mgr.get_config();
        self.emit_event(QueueEvent::QueueUpdated {
            items: vec![],
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
        self.emit_event(QueueEvent::PlaybackStopped);
    }

    pub async fn queue_state(&self) -> QueueSnapshot {
        let mut snap = self.queue_mgr.snapshot().await;
        snap.playback = self.playback.playback_state();
        snap
    }

    pub fn pause(&self) {
        self.playback.pause();
        if !self.playback.is_idle() {
            self.emit_event(QueueEvent::PlaybackPaused);
        }
    }

    pub fn resume(&self) {
        if !self.playback.is_idle() {
            self.playback.resume();
            self.emit_event(QueueEvent::PlaybackResumed);
        } else {
            let has_items = !self.queue_mgr.is_empty_sync();
            if has_items {
                self.spawn_processor_if_needed();
                self.playback.notify().notify_one();
            }
        }
    }

    pub fn get_queue_config(&self) -> QueueConfig {
        self.queue_mgr.get_config()
    }

    pub fn set_queue_config(&self, config: QueueConfig) -> Result<(), String> {
        self.queue_mgr.set_config(config)
    }

    // --- Internal ---

    fn is_main_window_visible(&self) -> bool {
        self.app_handle
            .get_webview_window("main")
            .map(|w| w.is_visible().unwrap_or(true))
            .unwrap_or(true)
    }

    fn sync_overlay(&self, has_items: bool) {
        if self.is_main_window_visible() {
            return;
        }
        let show = self.queue_mgr.get_config().show_overlay;
        if !show {
            crate::overlay::hide_overlay(&self.app_handle);
            return;
        }
        if has_items {
            crate::overlay::show_overlay(&self.app_handle);
        } else {
            crate::overlay::hide_overlay(&self.app_handle);
        }
    }

    fn emit_event(&self, event: QueueEvent) {
        self.app_handle.emit("tts-queue-event", &event).ok();
    }

    fn spawn_processor_if_needed(&self) {
        if self.processor_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }

        processor::run_processor(
            self.queue_mgr.queue_arc(),
            self.queue_mgr.config_arc(),
            self.pool.clone(),
            self.app_data_dir.clone(),
            self.playback.stop_flag(),
            self.playback.pause_flag(),
            self.playback.state_arc(),
            self.playback.notify(),
            self.processor_running.clone(),
            self.app_handle.clone(),
        );
    }
}
