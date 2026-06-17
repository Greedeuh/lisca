mod language;
pub mod piper;
pub mod kokoro;
pub mod audio;
pub mod commands;
pub mod config;
pub mod playback;
pub mod processor;
pub mod queue;
pub mod queue_store;
pub mod text;
pub mod voice_mapping;
mod onnx_session;
pub(crate) mod model;
mod queue_facade;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;

use self::model::{ActiveBackend, ModelFactory, ModelPool, VoiceResolver};
use self::config::ModelSelection;
use self::playback::PlaybackController;
use self::piper::InstalledModel;
use self::queue::{QueueConfig, QueueItem, QueueSnapshot};
use self::queue_facade::QueueFacade;
use self::queue_store::QueueStore;
use self::voice_mapping::VoiceMapping;

pub use self::model::TtsModel;

const I16_SAMPLE_SCALE: f32 = 32767.0;

/// Central orchestrator for TTS. Coordinates model pool, voice resolution,
/// queue management, and audio playback.
pub struct ModelsOrchestrator {
    pool: Arc<std::sync::Mutex<ModelPool>>,
    resolver: Arc<std::sync::Mutex<VoiceResolver>>,
    config: Arc<std::sync::Mutex<ModelSelection>>,
    queue: QueueFacade,
    pub(crate) app_data_dir: PathBuf,
    pub(crate) resource_dir: PathBuf,
}

impl ModelsOrchestrator {
    pub fn new(app_data_dir: PathBuf, resource_dir: PathBuf, app_handle: AppHandle) -> Self {
        let mapping = voice_mapping::load(&app_data_dir);
        let config = config::load_config(&app_data_dir);
        let (factory, active_backend) = match &config {
            ModelSelection::Kokoro => {
                let model_dir = ModelSelection::kokoro_model_dir(&app_data_dir);
                let factory: Box<dyn ModelFactory> = Box::new(kokoro::KokoroBackendFactory { model_dir });
                (factory, ActiveBackend::Kokoro)
            }
            ModelSelection::Piper { .. } => {
                let factory: Box<dyn ModelFactory> = Box::new(piper::PiperBackendFactory { resource_dir: resource_dir.clone() });
                (factory, ActiveBackend::Piper)
            }
        };

        let pool = Arc::new(std::sync::Mutex::new(ModelPool::new(
            factory,
            active_backend,
        )));
        let resolver = Arc::new(std::sync::Mutex::new(VoiceResolver::new(mapping)));
        let processor_running = Arc::new(AtomicBool::new(false));

        Self {
            queue: QueueFacade::new(
                QueueStore::new(app_data_dir.clone()),
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

    fn load_model_from_selection(
        selection: &ModelSelection,
        resource_dir: &std::path::Path,
        app_data_dir: &std::path::Path,
    ) -> Option<Box<dyn TtsModel>> {
        match selection {
            ModelSelection::Piper {
                model_path,
                config_path,
            } => piper::PiperModel::from_config(model_path, config_path, resource_dir, app_data_dir)
                .map(|m| Box::new(m) as Box<dyn TtsModel>),
            ModelSelection::Kokoro => kokoro::KokoroModel::from_config(app_data_dir)
                .map(|m| Box::new(m) as Box<dyn TtsModel>),
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
            let new_backend = Self::load_model_from_selection(&config, &base_dir, &app_data);
            pool.lock().unwrap().primary = new_backend;
        });
    }

    pub fn set_backend(&self, config: ModelSelection) -> Result<(), String> {
        self.stop();
        eprintln!("[tts] set_backend: {:?}", config);

        config::save_config(&self.app_data_dir, &config)?;
        *self.config.lock().unwrap() = config.clone();

        match Self::load_model_from_selection(&config, &self.resource_dir, &self.app_data_dir) {
            Some(new_backend) => {
                eprintln!("[tts] Backend loaded successfully");
                let (factory, active_backend) = match &config {
                    ModelSelection::Kokoro => {
                        let model_dir = ModelSelection::kokoro_model_dir(&self.app_data_dir);
                        let factory: Box<dyn ModelFactory> = Box::new(kokoro::KokoroBackendFactory { model_dir });
                        (factory, ActiveBackend::Kokoro)
                    }
                    ModelSelection::Piper { .. } => {
                        let factory: Box<dyn ModelFactory> = Box::new(piper::PiperBackendFactory { resource_dir: self.resource_dir.clone() });
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

    pub fn get_config(&self) -> ModelSelection {
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

    /// Detect the language of `text` and resolve it to a voice key.
    fn resolve_voice(&self, text: &str) -> Option<String> {
        let lang = language::detect_language_family(text);
        self.resolver.lock().unwrap().resolve_voice_key(lang)
    }

    /// Split `text` into chunks, synthesize each via the model, and return the
    /// concatenated f32 PCM samples.
    async fn synthesize_chunks(&self, text: &str, voice_key: &Option<String>) -> Result<Vec<f32>, String> {
        let pool = self.pool.clone();
        let vk = voice_key.clone();
        let text = text.to_string();

        tokio::task::spawn_blocking(move || {
            let mut pool_guard = pool.lock().unwrap();
            let model = pool_guard.get_model_for_language(vk.as_deref());
            let chunks = text::split_text(&text);

            let mut all_samples = Vec::new();
            for chunk in &chunks {
                match model.synthesize(chunk, 1.0) {
                    Ok(s) => all_samples.extend(s),
                    Err(e) => return Err(e),
                }
            }
            Ok(all_samples)
        })
        .await
        .map_err(|e| format!("Synthesis task panicked: {}", e))?
    }

    /// Play f32 PCM samples through the audio output. Runs on a blocking
    /// thread so the caller can `.await` the returned JoinHandle.
    fn play_samples(samples: Vec<f32>, sample_rate: u32) {
        let output = match audio::AudioOutput::try_new() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        let i16_samples = audio::f32_to_i16(&samples);
        output.play_buffer(i16_samples, sample_rate);
        output.sleep_until_end();
    }

    /// Detect language, synthesize text, and play the result.
    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let voice_key = self.resolve_voice(text);
        let sample_rate = {
            let guard = self.pool.lock().unwrap();
            guard.sample_rate_for_language(voice_key.as_deref())
        };

        let samples = self.synthesize_chunks(text, &voice_key).await?;
        tokio::task::spawn_blocking(move || Self::play_samples(samples, sample_rate))
            .await
            .map_err(|e| format!("Playback task panicked: {}", e))?;

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
