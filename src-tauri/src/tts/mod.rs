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
mod onnx_session;
pub(crate) mod backend;
mod queue_facade;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;

use self::backend::{ActiveBackend, AudioChunk, BackendFactory, BackendPool, VoiceResolver};
use self::config::BackendConfig;
use self::playback::PlaybackController;
use self::piper::InstalledModel;
use self::queue::{QueueConfig, QueueItem, QueueSnapshot};
use self::queue_facade::QueueFacade;
use self::queue_manager::QueueManager;
use self::voice_mapping::VoiceMapping;

pub use self::backend::TtsBackend;

const AUDIO_CHANNEL_BUFFER: usize = 8;
const I16_SAMPLE_SCALE: f32 = 32767.0;

pub struct TtsManager {
    pool: Arc<std::sync::Mutex<BackendPool>>,
    resolver: Arc<std::sync::Mutex<VoiceResolver>>,
    config: Arc<std::sync::Mutex<BackendConfig>>,
    queue: QueueFacade,
    pub(crate) app_data_dir: PathBuf,
    pub(crate) resource_dir: PathBuf,
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
                let factory: Box<dyn BackendFactory> = Box::new(piper::PiperBackendFactory { resource_dir: resource_dir.clone() });
                (factory, ActiveBackend::Piper)
            }
        };

        let pool = Arc::new(std::sync::Mutex::new(BackendPool::new(
            factory,
            active_backend,
        )));
        let resolver = Arc::new(std::sync::Mutex::new(VoiceResolver::new(mapping)));
        let processor_running = Arc::new(AtomicBool::new(false));

        Self {
            queue: QueueFacade::new(
                QueueManager::new(app_data_dir.clone()),
                PlaybackController::new(),
                processor_running,
                pool.clone(),
                resolver.clone(),
                app_data_dir.clone(),
                app_handle.clone(),
            ),
            pool,
            resolver,
            config: Arc::new(std::sync::Mutex::new(config)),
            app_data_dir,
            resource_dir,
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
                let mp = BackendConfig::resolve_path(model_path, app_data_dir);
                let cp = BackendConfig::resolve_path(config_path, app_data_dir);

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
        let config = self.config.lock().unwrap().clone();

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
        eprintln!("[tts] set_backend: {:?}", config);

        config::save_config(&self.app_data_dir, &config)?;
        *self.config.lock().unwrap() = config.clone();

        match Self::load_backend_from_config(&config, &self.resource_dir, &self.app_data_dir) {
            Some(new_backend) => {
                eprintln!("[tts] Backend loaded successfully");
                let (factory, active_backend) = match &config {
                    BackendConfig::Kokoro => {
                        let model_dir = BackendConfig::kokoro_model_dir(&self.app_data_dir);
                        let factory: Box<dyn BackendFactory> = Box::new(kokoro::KokoroBackendFactory { model_dir });
                        (factory, ActiveBackend::Kokoro)
                    }
                    BackendConfig::Piper { .. } => {
                        let factory: Box<dyn BackendFactory> = Box::new(piper::PiperBackendFactory { resource_dir: self.resource_dir.clone() });
                        (factory, ActiveBackend::Piper)
                    }
                };
                let mut pool = self.pool.lock().unwrap();
                pool.primary = Some(new_backend);
                pool.factory = factory;
                pool.set_active_backend(active_backend);
                Ok(())
            }
            None => {
                eprintln!("[tts] Backend load failed, config saved for next launch");
                let mut pool = self.pool.lock().unwrap();
                pool.primary = None;
                Ok(())
            }
        }
    }

    pub fn get_config(&self) -> BackendConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn refresh_installed_models(&self, models: Vec<InstalledModel>) {
        self.resolver.lock().unwrap().refresh_installed(models.clone());
        self.pool.lock().unwrap().refresh_installed(models);
    }

    pub fn get_voice_mapping(&self) -> VoiceMapping {
        voice_mapping::load(&self.app_data_dir)
    }

    pub fn set_voice_mapping(&self, mapping: VoiceMapping) -> Result<(), String> {
        voice_mapping::save(&self.app_data_dir, &mapping)?;
        self.resolver.lock().unwrap().set_mapping(mapping);
        self.pool.lock().unwrap().clear_cache();
        Ok(())
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioChunk>(AUDIO_CHANNEL_BUFFER);

        let lang = language::detect_language_family(text);
        let voice_key = self.resolver.lock().unwrap().resolve_voice_key(lang);

        let pool = self.pool.clone();
        let vk = voice_key.clone();
        let text_clone = text.to_string();
        let synth_handle = tokio::task::spawn_blocking(move || {
            let mut pool_guard = pool.lock().unwrap();
            let model = pool_guard.get_for_language(vk.as_deref());

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
            guard.sample_rate_for_language(voice_key.as_deref())
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
        self.queue.stop();
    }

    pub async fn queue_add(&self, text: String) -> Result<QueueItem, String> {
        self.queue.add(text).await
    }

    pub async fn queue_remove(&self, id: u32) {
        self.queue.remove(id).await;
    }

    pub async fn queue_move(&self, id: u32, new_index: usize) {
        self.queue.move_item(id, new_index).await;
    }

    pub async fn queue_clear(&self) {
        self.queue.clear().await;
    }

    pub async fn queue_state(&self) -> QueueSnapshot {
        self.queue.state().await
    }

    pub fn pause(&self) {
        self.queue.pause();
    }

    pub fn resume(&self) {
        self.queue.resume();
    }

    pub fn get_queue_config(&self) -> QueueConfig {
        self.queue.get_config()
    }

    pub fn set_queue_config(&self, config: QueueConfig) -> Result<(), String> {
        self.queue.set_config(config)
    }
}
