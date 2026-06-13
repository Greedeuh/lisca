mod kokoro;
mod piper;
mod session;
pub mod config;
pub mod piper_models;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::AppHandle;
use tauri::Manager;

use kokoro::KokoroModel;
use piper::PiperModel;

use self::config::BackendConfig;

type PiperManager = Arc<tokio::sync::Mutex<piper_models::PiperModelManager>>;

pub trait TtsBackend: Send {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String>;
    fn sample_rate(&self) -> u32;
}

enum AudioChunk {
    Samples(Vec<f32>),
    Done,
}

struct AudioState {
    sink: rodio::Sink,
    #[allow(dead_code)]
    cancel_tx: tokio::sync::mpsc::Sender<AudioChunk>,
}

pub struct TtsManager {
    backend: Arc<std::sync::Mutex<Option<Box<dyn TtsBackend>>>>,
    audio: Arc<std::sync::Mutex<Option<AudioState>>>,
    app_data_dir: PathBuf,
    resource_dir: PathBuf,
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
    pub fn new(app_data_dir: PathBuf, resource_dir: PathBuf) -> Self {
        Self {
            backend: Arc::new(std::sync::Mutex::new(None)),
            audio: Arc::new(std::sync::Mutex::new(None)),
            app_data_dir,
            resource_dir,
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

    pub fn switch_backend(&self, config: BackendConfig) -> Result<(), String> {
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

        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioChunk>(8);
        let cancel_tx = audio_tx.clone();

        let backend = self.backend.clone();
        let text = text.to_string();
        let synth_handle = tokio::task::spawn_blocking(move || {
            let mut backend_guard = backend.lock().unwrap();
            let model = match backend_guard.as_mut() {
                Some(m) => m,
                None => {
                    drop(backend_guard);
                    let _ = audio_tx.blocking_send(AudioChunk::Done);
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
            let _ = audio_tx.blocking_send(AudioChunk::Done);
        });

        let sample_rate = {
            let guard = self.backend.lock().unwrap();
            guard.as_ref().map(|b| b.sample_rate()).unwrap_or(24000)
        };

        let audio = self.audio.clone();
        let play_handle = tokio::task::spawn_blocking(move || {
            use rodio::buffer::SamplesBuffer;
            use rodio::{OutputStream, Sink};

            let (_stream, handle) =
                OutputStream::try_default().expect("Failed to open audio output");
            let sink = Sink::try_new(&handle).expect("Failed to create audio sink");

            {
                let mut guard = audio.lock().unwrap();
                *guard = Some(AudioState { sink, cancel_tx });
            }

            loop {
                let chunk = audio_rx.blocking_recv();
                let mut guard = audio.lock().unwrap();
                let state = match guard.as_mut() {
                    Some(s) => s,
                    None => break,
                };

                match chunk {
                    Some(AudioChunk::Samples(samples)) => {
                        let i16_samples: Vec<i16> = samples
                            .iter()
                            .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                            .collect();
                        let buffer = SamplesBuffer::new(1, sample_rate, i16_samples);
                        state.sink.append(buffer);
                    }
                    _ => break,
                }
                drop(guard);
            }

            let state = audio.lock().unwrap().take();
            if let Some(s) = state {
                s.sink.sleep_until_end();
            }
        });

        let _ = synth_handle.await;
        let _ = play_handle.await;

        Ok(())
    }

    pub fn stop(&self) {
        *self.audio.lock().unwrap() = None;
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
    tts.switch_backend(config)
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
    let manager = app.state::<PiperManager>();
    let mut manager = manager.lock().await;
    manager.fetch_voices().await.cloned()
}

#[tauri::command]
pub async fn piper_download_model(
    app: AppHandle,
    voice_key: String,
) -> Result<piper_models::InstalledModel, String> {
    let manager = app.state::<PiperManager>();
    let manager = manager.lock().await;
    manager.download_voice(&voice_key, &app).await
}

#[tauri::command]
pub async fn piper_list_installed(app: AppHandle) -> Result<Vec<piper_models::InstalledModel>, String> {
    let manager = app.state::<PiperManager>();
    let manager = manager.lock().await;
    Ok(manager.list_installed())
}

#[tauri::command]
pub async fn piper_delete_model(app: AppHandle, voice_key: String) -> Result<(), String> {
    let manager = app.state::<PiperManager>();
    let manager = manager.lock().await;
    manager.delete_model(&voice_key)
}
