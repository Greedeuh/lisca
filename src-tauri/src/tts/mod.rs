mod kokoro;
mod piper;
mod session;
pub mod commands;
pub mod config;
pub mod piper_models;
pub mod processor;
pub mod queue;
pub mod text;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;

use kokoro::KokoroModel;
use piper::PiperModel;

use self::config::BackendConfig;
use self::queue::{QueueConfig, QueueEvent, QueueItem, QueueSnapshot, PlaybackState};

const AUDIO_CHANNEL_BUFFER: usize = 8;
const DEFAULT_SAMPLE_RATE: u32 = 24000;
const I16_SAMPLE_SCALE: f32 = 32767.0;

pub trait TtsBackend: Send {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String>;
    fn sample_rate(&self) -> u32;
}

enum AudioChunk {
    Samples(Vec<f32>),
    Eof,
}

const STATE_IDLE: u8 = PlaybackState::Idle as u8;
const STATE_PLAYING: u8 = PlaybackState::Playing as u8;
const STATE_PAUSED: u8 = PlaybackState::Paused as u8;

pub struct TtsManager {
    backend: Arc<std::sync::Mutex<Option<Box<dyn TtsBackend>>>>,
    pub(crate) app_data_dir: PathBuf,
    pub(crate) resource_dir: PathBuf,
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    next_id: Arc<std::sync::Mutex<u32>>,
    pub(crate) app_handle: AppHandle,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    playback_state: Arc<AtomicU8>,
    notify: Arc<tokio::sync::Notify>,
    processor_running: Arc<AtomicBool>,
}

impl TtsManager {
    pub fn new(app_data_dir: PathBuf, resource_dir: PathBuf, app_handle: AppHandle) -> Self {
        let queue = queue::load_queue(&app_data_dir);
        let queue_config = queue::load_queue_config(&app_data_dir);
        let next_id = queue.iter().map(|i| i.id).max().unwrap_or(0) + 1;

        Self {
            backend: Arc::new(std::sync::Mutex::new(None)),
            app_data_dir,
            resource_dir,
            queue: Arc::new(tokio::sync::Mutex::new(queue)),
            queue_config: Arc::new(std::sync::Mutex::new(queue_config)),
            next_id: Arc::new(std::sync::Mutex::new(next_id)),
            app_handle,
            stop_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            playback_state: Arc::new(AtomicU8::new(STATE_IDLE)),
            notify: Arc::new(tokio::sync::Notify::new()),
            processor_running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn load_backend_from_config(config: &BackendConfig, base_dir: &std::path::Path) -> Option<Box<dyn TtsBackend>> {
        match config {
            BackendConfig::Kokoro {
                model_path,
                voice_path,
            } => {
                let mp = BackendConfig::resolve_path(model_path, base_dir);
                let vp = BackendConfig::resolve_path(voice_path, base_dir);

                if !mp.exists() || !vp.exists() {
                    eprintln!("Kokoro model files not found: {:?}, {:?}", mp, vp);
                    return None;
                }

                eprintln!("Preloading Kokoro model...");
                let start = std::time::Instant::now();

                match KokoroModel::load(&mp, &vp) {
                    Ok(model) => {
                        eprintln!(
                            "Kokoro model preloaded in {}ms",
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
            BackendConfig::Piper {
                model_path,
                config_path,
            } => {
                let mp = BackendConfig::resolve_path(model_path, base_dir);
                let cp = BackendConfig::resolve_path(config_path, base_dir);

                if !mp.exists() || !cp.exists() {
                    eprintln!("Piper model files not found: {:?}, {:?}", mp, cp);
                    return None;
                }

                eprintln!("Preloading Piper model...");
                let start = std::time::Instant::now();

                match PiperModel::load(&mp, &cp, base_dir) {
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
        }
    }

    pub fn preload(&self) {
        let config = config::load_config(&self.app_data_dir);

        if self.backend.try_lock().map(|b| b.is_some()).unwrap_or(true) {
            return;
        }

        let backend = self.backend.clone();
        let base_dir = self.resource_dir.clone();

        std::thread::spawn(move || {
            let new_backend = Self::load_backend_from_config(&config, &base_dir);
            *backend.lock().unwrap() = new_backend;
        });
    }

    pub fn set_backend(&self, config: BackendConfig) -> Result<(), String> {
        self.stop();

        let new_backend = Self::load_backend_from_config(&config, &self.resource_dir)
            .ok_or("Failed to load backend")?;

        *self.backend.lock().unwrap() = Some(new_backend);
        config::save_config(&self.app_data_dir, &config)?;
        Ok(())
    }

    pub fn get_config(&self) -> BackendConfig {
        config::load_config(&self.app_data_dir)
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioChunk>(AUDIO_CHANNEL_BUFFER);

        let backend = self.backend.clone();
        let text = text.to_string();
        let synth_handle = tokio::task::spawn_blocking(move || {
            let mut backend_guard = backend.lock().unwrap();
            let model = match backend_guard.as_mut() {
                Some(m) => m,
                None => {
                    drop(backend_guard);
                    let _ = audio_tx.blocking_send(AudioChunk::Eof);
                    return;
                }
            };

            let chunks = text::split_text(&text);
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
            drop(backend_guard);
            let _ = audio_tx.blocking_send(AudioChunk::Eof);
        });

        let sample_rate = {
            let guard = self.backend.lock().unwrap();
            guard.as_ref().map(|b| b.sample_rate()).unwrap_or(DEFAULT_SAMPLE_RATE)
        };

        let play_handle = tokio::task::spawn_blocking(move || {
            use rodio::buffer::SamplesBuffer;
            use rodio::{OutputStream, Sink};

            let (_stream, handle) =
                OutputStream::try_default().expect("Failed to open audio output");
            let sink = Sink::try_new(&handle).expect("Failed to create audio sink");

            loop {
                let chunk = audio_rx.blocking_recv();
                match chunk {
                    Some(AudioChunk::Samples(samples)) => {
                        let i16_samples: Vec<i16> = samples
                            .iter()
                            .map(|s| (s * I16_SAMPLE_SCALE).clamp(-32768.0, I16_SAMPLE_SCALE) as i16)
                            .collect();
                        let buffer = SamplesBuffer::new(1, sample_rate, i16_samples);
                        sink.append(buffer);
                    }
                    _ => break,
                }
            }

            sink.sleep_until_end();
            drop(sink);
            drop(_stream);
        });

        let _ = synth_handle.await;
        let _ = play_handle.await;

        Ok(())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.playback_state.store(STATE_IDLE, Ordering::SeqCst);
        self.notify.notify_one();
    }

    // --- Queue methods ---

    pub async fn queue_add(&self, text: String) -> Result<QueueItem, String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let (item, notify_needed) = {
            let config = self.queue_config.lock().unwrap().clone();
            let mut q = self.queue.lock().await;

            if q.len() >= config.max_items {
                return Err(format!("Queue is full (max {})", config.max_items));
            }

            let id = {
                let mut next = self.next_id.lock().unwrap();
                let id = *next;
                *next += 1;
                id
            };

            let item = QueueItem {
                id,
                text: text.trim().to_string(),
            };
            let was_empty = q.is_empty();
            q.push_back(item.clone());

            let items: Vec<QueueItem> = q.iter().cloned().collect();
            queue::save_queue(&self.app_data_dir, &q)
                .map_err(|e| eprintln!("Failed to save queue: {}", e))
                .ok();

            let auto_read = config.auto_read;
            let show_overlay = config.show_overlay;
            self.emit_event(QueueEvent::QueueUpdated { items, auto_read, show_overlay });

            (item, was_empty)
        };

        if notify_needed {
            self.spawn_processor_if_needed();
            self.notify.notify_one();
        }

        self.sync_overlay(true);

        Ok(item)
    }

    pub async fn queue_remove(&self, id: u32) {
        let mut q = self.queue.lock().await;
        q.retain(|i| i.id != id);
        let items: Vec<QueueItem> = q.iter().cloned().collect();
        let config = self.queue_config.lock().unwrap().clone();
        queue::save_queue(&self.app_data_dir, &q)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    pub async fn queue_move(&self, id: u32, new_index: usize) {
        let mut q = self.queue.lock().await;
        let old_pos = match q.iter().position(|i| i.id == id) {
            Some(p) => p,
            None => return,
        };

        let new_pos = new_index.min(q.len().saturating_sub(1));
        if old_pos == new_pos {
            return;
        }

        let item = match q.remove(old_pos) {
            Some(item) => item,
            None => return,
        };
        q.insert(new_pos, item);

        let items: Vec<QueueItem> = q.iter().cloned().collect();
        let config = self.queue_config.lock().unwrap().clone();
        queue::save_queue(&self.app_data_dir, &q)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    pub async fn queue_clear(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.playback_state.store(STATE_IDLE, Ordering::SeqCst);
        self.notify.notify_one();

        let mut q = self.queue.lock().await;
        q.clear();
        let config = self.queue_config.lock().unwrap().clone();
        queue::save_queue(&self.app_data_dir, &q)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
        self.emit_event(QueueEvent::QueueUpdated {
            items: vec![],
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
        self.emit_event(QueueEvent::PlaybackStopped);
    }

    pub async fn queue_state(&self) -> QueueSnapshot {
        let items = self.queue.lock().await;
        let config = self.queue_config.lock().unwrap().clone();
        let playback = PlaybackState::from(self.playback_state.load(Ordering::SeqCst));
        QueueSnapshot {
            items: items.iter().cloned().collect(),
            playback,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        }
    }

    pub fn pause(&self) {
        if self.playback_state.load(Ordering::SeqCst) == STATE_PLAYING {
            self.pause_flag.store(true, Ordering::SeqCst);
            self.playback_state.store(STATE_PAUSED, Ordering::SeqCst);
            self.emit_event(QueueEvent::PlaybackPaused);
        }
    }

    pub fn resume(&self) {
        if self.playback_state.load(Ordering::SeqCst) == STATE_PAUSED {
            self.pause_flag.store(false, Ordering::SeqCst);
            self.playback_state.store(STATE_PLAYING, Ordering::SeqCst);
            self.notify.notify_one();
            self.emit_event(QueueEvent::PlaybackResumed);
        } else if self.playback_state.load(Ordering::SeqCst) == STATE_IDLE {
            let has_items = self.queue.try_lock().map(|q| !q.is_empty()).unwrap_or(false);
            if has_items {
                self.spawn_processor_if_needed();
                self.notify.notify_one();
            }
        }
    }

    pub fn get_queue_config(&self) -> QueueConfig {
        self.queue_config.lock().unwrap().clone()
    }

    pub fn set_queue_config(&self, config: QueueConfig) -> Result<(), String> {
        queue::save_queue_config(&self.app_data_dir, &config)?;
        *self.queue_config.lock().unwrap() = config;
        Ok(())
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
        let show = self.queue_config.lock().unwrap().show_overlay;
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
            self.queue.clone(),
            self.queue_config.clone(),
            self.backend.clone(),
            self.app_data_dir.clone(),
            self.stop_flag.clone(),
            self.pause_flag.clone(),
            self.playback_state.clone(),
            self.notify.clone(),
            self.processor_running.clone(),
            self.app_handle.clone(),
        );
    }
}
