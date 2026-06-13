mod kokoro;
mod piper;
mod session;
pub mod config;
pub mod piper_models;
pub mod queue;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};
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

type SharedPiperModelManager = Arc<tokio::sync::Mutex<piper_models::PiperModelManager>>;

pub trait TtsBackend: Send {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String>;
    fn sample_rate(&self) -> u32;
}

enum AudioChunk {
    Samples(Vec<f32>),
    Eof,
}

const STATE_IDLE: u8 = 0;
const STATE_PLAYING: u8 = 1;
const STATE_PAUSED: u8 = 2;

pub struct TtsManager {
    backend: Arc<std::sync::Mutex<Option<Box<dyn TtsBackend>>>>,
    app_data_dir: PathBuf,
    resource_dir: PathBuf,
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    next_id: Arc<std::sync::Mutex<u32>>,
    app_handle: AppHandle,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    playback_state: Arc<AtomicU8>,
    notify: Arc<tokio::sync::Notify>,
    processor_running: Arc<AtomicBool>,
}

fn split_text(text: &str) -> Vec<String> {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"([.!?;])\s+").unwrap());
    let mut chunks: Vec<String> = Vec::new();
    let mut last = 0;
    for m in re.find_iter(text) {
        let split_at = m.start() + 1;
        let chunk = text[last..split_at].trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        last = m.end();
    }
    if last < text.len() {
        let tail = text[last..].trim().to_string();
        if !tail.is_empty() {
            chunks.push(tail);
        }
    }
    if chunks.is_empty() {
        vec![text.trim().to_string()]
    } else {
        chunks
    }
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

            let chunks = split_text(&text);
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

            if q.len() >= config.max_size {
                return Err(format!("Queue is full (max {})", config.max_size));
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
            self.emit_event(QueueEvent::QueueUpdated { items, auto_read });

            (item, was_empty)
        };

        if notify_needed {
            self.spawn_processor_if_needed();
            self.notify.notify_one();
        }

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

        let item = q.remove(old_pos).unwrap();
        q.insert(new_pos, item);

        let items: Vec<QueueItem> = q.iter().cloned().collect();
        let config = self.queue_config.lock().unwrap().clone();
        queue::save_queue(&self.app_data_dir, &q)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
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
        });
        self.emit_event(QueueEvent::PlaybackStopped);
    }

    pub async fn queue_state(&self) -> QueueSnapshot {
        let items = self.queue.lock().await;
        let config = self.queue_config.lock().unwrap().clone();
        let playback = match self.playback_state.load(Ordering::SeqCst) {
            STATE_PLAYING => PlaybackState::Playing,
            STATE_PAUSED => PlaybackState::Paused,
            _ => PlaybackState::Idle,
        };
        QueueSnapshot {
            items: items.iter().cloned().collect(),
            playback,
            current: None,
            auto_read: config.auto_read,
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

    fn emit_event(&self, event: QueueEvent) {
        self.app_handle.emit("tts-queue-event", &event).ok();
    }

    fn spawn_processor_if_needed(&self) {
        if self.processor_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }

        let queue = self.queue.clone();
        let queue_config = self.queue_config.clone();
        let backend = self.backend.clone();
        let app_data_dir = self.app_data_dir.clone();
        let stop_flag = self.stop_flag.clone();
        let pause_flag = self.pause_flag.clone();
        let playback_state = self.playback_state.clone();
        let notify = self.notify.clone();
        let processor_running = self.processor_running.clone();
        let app_handle = self.app_handle.clone();

        tokio::spawn(async move {
            loop {
                notify.notified().await;

                let should_exit = 'outer: loop {
                    if stop_flag.load(Ordering::SeqCst) {
                        stop_flag.store(false, Ordering::SeqCst);
                        pause_flag.store(false, Ordering::SeqCst);
                        playback_state.store(STATE_IDLE, Ordering::SeqCst);
                        break 'outer false;
                    }

                    if pause_flag.load(Ordering::SeqCst) {
                        playback_state.store(STATE_PAUSED, Ordering::SeqCst);
                        loop {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            if stop_flag.load(Ordering::SeqCst) || !pause_flag.load(Ordering::SeqCst) {
                                break;
                            }
                        }
                        if stop_flag.load(Ordering::SeqCst) {
                            stop_flag.store(false, Ordering::SeqCst);
                            pause_flag.store(false, Ordering::SeqCst);
                            playback_state.store(STATE_IDLE, Ordering::SeqCst);
                            break 'outer false;
                        }
                        pause_flag.store(false, Ordering::SeqCst);
                        playback_state.store(STATE_PLAYING, Ordering::SeqCst);
                    }

                    let item = {
                        let mut q = queue.lock().await;
                        q.pop_front()
                    };

                    let item = match item {
                        Some(i) => i,
                        None => {
                            let config = queue_config.lock().unwrap().clone();
                            let items = queue.lock().await.iter().cloned().collect();
                            app_handle.emit("tts-queue-event", &QueueEvent::QueueUpdated {
                                items,
                                auto_read: config.auto_read,
                            }).ok();
                            break 'outer true;
                        }
                    };

                    {
                        let config = queue_config.lock().unwrap().clone();
                        let items = queue.lock().await.iter().cloned().collect();
                        app_handle.emit("tts-queue-event", &QueueEvent::QueueUpdated {
                            items,
                            auto_read: config.auto_read,
                        }).ok();
                    }

                    app_handle.emit("tts-queue-event", &QueueEvent::PlaybackStarted {
                        item: item.clone(),
                    }).ok();

                    let backend_clone = backend.clone();
                    let text = item.text.clone();
                    let synth_result = tokio::task::spawn_blocking(move || {
                        let mut guard = backend_clone.lock().unwrap();
                        let model = match guard.as_mut() {
                            Some(m) => m,
                            None => return Err("No backend loaded".to_string()),
                        };
                        let chunks = split_text(&text);
                        let mut all_samples = Vec::new();
                        for chunk in &chunks {
                            match model.synthesize(chunk, 1.0) {
                                Ok(samples) => all_samples.extend(samples),
                                Err(e) => return Err(e),
                            }
                        }
                        Ok(all_samples)
                    })
                    .await
                    .unwrap();

                    match synth_result {
                        Ok(samples) => {
                            let sample_rate = {
                                let guard = backend.lock().unwrap();
                                guard.as_ref().map(|b| b.sample_rate()).unwrap_or(DEFAULT_SAMPLE_RATE)
                            };

                            playback_state.store(STATE_PLAYING, Ordering::SeqCst);

                            let play_stop = stop_flag.clone();
                            let play_pause = pause_flag.clone();
                            let play_state = playback_state.clone();
                            let play_result = tokio::task::spawn_blocking(move || {
                                use rodio::{OutputStream, Sink};
                                use rodio::buffer::SamplesBuffer;

                                let (_stream, handle) = OutputStream::try_default()
                                    .expect("Failed to open audio output");
                                let sink = Sink::try_new(&handle)
                                    .expect("Failed to create audio sink");

                                let i16_samples: Vec<i16> = samples
                                    .iter()
                                    .map(|s| (s * I16_SAMPLE_SCALE).clamp(-32768.0, I16_SAMPLE_SCALE) as i16)
                                    .collect();
                                let buffer = SamplesBuffer::new(1, sample_rate, i16_samples);
                                sink.append(buffer);

                                loop {
                                    std::thread::sleep(std::time::Duration::from_millis(50));

                                    if play_stop.load(Ordering::SeqCst) {
                                        play_stop.store(false, Ordering::SeqCst);
                                        play_pause.store(false, Ordering::SeqCst);
                                        play_state.store(STATE_IDLE, Ordering::SeqCst);
                                        drop(sink);
                                        drop(_stream);
                                        return true;
                                    }

                                    if play_pause.load(Ordering::SeqCst) {
                                        sink.pause();
                                        play_state.store(STATE_PAUSED, Ordering::SeqCst);
                                        loop {
                                            std::thread::sleep(std::time::Duration::from_millis(50));
                                            if play_stop.load(Ordering::SeqCst) || !play_pause.load(Ordering::SeqCst) {
                                                break;
                                            }
                                        }
                                        if play_stop.load(Ordering::SeqCst) {
                                            play_stop.store(false, Ordering::SeqCst);
                                            play_pause.store(false, Ordering::SeqCst);
                                            play_state.store(STATE_IDLE, Ordering::SeqCst);
                                            drop(sink);
                                            drop(_stream);
                                            return true;
                                        }
                                        play_pause.store(false, Ordering::SeqCst);
                                        play_state.store(STATE_PLAYING, Ordering::SeqCst);
                                        sink.play();
                                    }

                                    if sink.empty() {
                                        break;
                                    }
                                }

                                sink.sleep_until_end();
                                drop(sink);
                                drop(_stream);
                                false
                            })
                            .await
                            .unwrap();

                            if play_result {
                                app_handle.emit("tts-queue-event", &QueueEvent::PlaybackStopped).ok();
                                continue;
                            }

                            playback_state.store(STATE_IDLE, Ordering::SeqCst);
                            {
                                let config = queue_config.lock().unwrap().clone();
                                let q_ref = queue.lock().await;
                                let items = q_ref.iter().cloned().collect();
                                queue::save_queue(&app_data_dir, &q_ref)
                                    .map_err(|e| eprintln!("Failed to save queue: {}", e))
                                    .ok();
                                drop(q_ref);
                                app_handle.emit("tts-queue-event", &QueueEvent::ItemCompleted {
                                    id: item.id,
                                }).ok();
                                app_handle.emit("tts-queue-event", &QueueEvent::QueueUpdated {
                                    items,
                                    auto_read: config.auto_read,
                                }).ok();
                            }

                            if !queue_config.lock().unwrap().auto_read {
                                break 'outer true;
                            }
                        }
                        Err(e) => {
                            app_handle.emit("tts-queue-event", &QueueEvent::Error {
                                id: Some(item.id),
                                message: e,
                            }).ok();

                            {
                                let mut q = queue.lock().await;
                                q.retain(|i| i.id != item.id);
                                let config = queue_config.lock().unwrap().clone();
                                let items = q.iter().cloned().collect();
                                queue::save_queue(&app_data_dir, &q)
                                    .map_err(|e| eprintln!("Failed to save queue: {}", e))
                                    .ok();
                                app_handle.emit("tts-queue-event", &QueueEvent::QueueUpdated {
                                    items,
                                    auto_read: config.auto_read,
                                }).ok();
                            }
                            continue;
                        }
                    }
                };

                processor_running.store(false, Ordering::SeqCst);

                if should_exit {
                    break;
                }
            }
        });
    }
}

#[tauri::command]
pub async fn tts_speak(app: AppHandle, text: String) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.speak(&text).await
}

#[tauri::command]
pub fn tts_stop(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.stop();
}

#[tauri::command]
pub fn tts_get_config(app: AppHandle) -> Result<BackendConfig, String> {
    let tts = app.state::<Arc<TtsManager>>();
    Ok(tts.get_config())
}

#[tauri::command]
pub fn tts_set_config(app: AppHandle, config: BackendConfig) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.set_backend(config)
}

#[tauri::command]
pub fn tts_open_resource_dir(app: AppHandle) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    let dir = &tts.resource_dir;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn piper_fetch_voices(app: AppHandle) -> Result<piper_models::VoiceCatalog, String> {
    let manager = app.state::<SharedPiperModelManager>();
    let mut manager = manager.lock().await;
    manager.fetch_voices().await.cloned()
}

#[tauri::command]
pub async fn piper_download_model(
    app: AppHandle,
    voice_key: String,
) -> Result<piper_models::InstalledModel, String> {
    let manager = app.state::<SharedPiperModelManager>();
    let manager = manager.lock().await;
    manager.download_voice(&voice_key, &app).await
}

#[tauri::command]
pub async fn piper_list_installed(app: AppHandle) -> Result<Vec<piper_models::InstalledModel>, String> {
    let manager = app.state::<SharedPiperModelManager>();
    let manager = manager.lock().await;
    Ok(manager.list_installed())
}

#[tauri::command]
pub async fn piper_delete_model(app: AppHandle, voice_key: String) -> Result<(), String> {
    let manager = app.state::<SharedPiperModelManager>();
    let manager = manager.lock().await;
    manager.delete_model(&voice_key)
}

// --- Queue commands ---

#[tauri::command]
pub async fn tts_queue_add(app: AppHandle, text: String) -> Result<QueueItem, String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_add(text).await
}

#[tauri::command]
pub async fn tts_queue_remove(app: AppHandle, id: u32) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_remove(id).await;
}

#[tauri::command]
pub async fn tts_queue_move(app: AppHandle, id: u32, index: usize) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_move(id, index).await;
}

#[tauri::command]
pub async fn tts_queue_clear(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_clear().await;
}

#[tauri::command]
pub async fn tts_queue_state(app: AppHandle) -> QueueSnapshot {
    let tts = app.state::<Arc<TtsManager>>();
    tts.queue_state().await
}

#[tauri::command]
pub fn tts_pause(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.pause();
}

#[tauri::command]
pub async fn tts_resume(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.resume();
}

#[tauri::command]
pub fn tts_set_queue_config(app: AppHandle, config: QueueConfig) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.set_queue_config(config)
}

#[tauri::command]
pub fn tts_get_queue_config(app: AppHandle) -> QueueConfig {
    let tts = app.state::<Arc<TtsManager>>();
    tts.get_queue_config()
}
