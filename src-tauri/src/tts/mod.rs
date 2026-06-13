mod kokoro;
mod session;

use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;

use kokoro::KokoroModel;

enum AudioChunk {
    Samples(Vec<f32>),
    Done,
}

pub struct TtsManager {
    backend: Arc<std::sync::Mutex<Option<KokoroModel>>>,
    loading: Arc<std::sync::atomic::AtomicBool>,
    system_tts: Mutex<tts::Tts>,
    audio_lock: Arc<std::sync::Mutex<Option<rodio::Sink>>>,
    cancel_tx: Arc<std::sync::Mutex<Option<tokio::sync::mpsc::Sender<AudioChunk>>>>,
    _app_handle: AppHandle,
}

impl TtsManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let engine = tts::Tts::default().map_err(|e| format!("Failed to init system TTS: {}", e))?;

        let manager = Self {
            backend: Arc::new(std::sync::Mutex::new(None)),
            loading: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            system_tts: Mutex::new(engine),
            audio_lock: Arc::new(std::sync::Mutex::new(None)),
            cancel_tx: Arc::new(std::sync::Mutex::new(None)),
            _app_handle: app_handle,
        };

        Ok(manager)
    }

    fn split_text(text: &str) -> Vec<String> {
        let re = regex::Regex::new(r"([.!?;])\s+").unwrap();
        let mut chunks: Vec<String> = Vec::new();
        let mut last = 0;
        for m in re.find_iter(text) {
            let split_at = m.start() + 1; // after the punctuation char
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

    /// Start loading model in background thread (non-blocking).
    pub fn preload(&self) {
        let models_dir = Self::find_project_root().join("models");

        let model_path = models_dir.join("kokoro-q8.onnx");
        let voice_path = models_dir.join("voices/af.bin");

        if !model_path.exists() || !voice_path.exists() {
            eprintln!("Model files not found, skipping preload");
            return;
        }

        if self.loading.load(std::sync::atomic::Ordering::Relaxed)
            || self.is_loaded()
        {
            return;
        }

        self.loading.store(true, std::sync::atomic::Ordering::Relaxed);

        let mp = model_path.display().to_string();
        let vp = voice_path.display().to_string();

        let backend = self.backend.clone();
        let loading = self.loading.clone();

        std::thread::spawn(move || {
            eprintln!("Preloading Kokoro model...");
            let start = std::time::Instant::now();

            match KokoroModel::load(
                std::path::Path::new(&mp),
                std::path::Path::new(&vp),
            ) {
                Ok(model) => {
                    *backend.lock().unwrap() = Some(model);
                    eprintln!(
                        "Kokoro model preloaded in {}ms",
                        start.elapsed().as_millis()
                    );
                }
                Err(e) => eprintln!("Preload failed: {}", e),
            }

            loading.store(false, std::sync::atomic::Ordering::Relaxed);
        });
    }

    fn find_project_root() -> std::path::PathBuf {
        let cwd = std::env::current_dir().unwrap_or_default();

        if cwd.join("models").exists() {
            return cwd;
        }

        let mut path = cwd.clone();
        for _ in 0..5 {
            if path.join("models").exists() {
                return path;
            }
            if !path.pop() {
                break;
            }
        }

        cwd
    }

    fn resolve_path(path: &str, project_root: &std::path::Path) -> String {
        let p = std::path::Path::new(path);
        if p.is_absolute() && p.exists() {
            return path.to_string();
        }
        let resolved = project_root.join(path);
        if resolved.exists() {
            return resolved.display().to_string();
        }
        path.to_string()
    }

    /// Load a Kokoro ONNX model and voice file.
    pub async fn load_model(&self, model_path: &str, voice_path: &str) -> Result<(), String> {
        let project_root = Self::find_project_root();

        let model_path = Self::resolve_path(model_path, &project_root);
        let voice_path = Self::resolve_path(voice_path, &project_root);

        eprintln!("Loading model: {}", model_path);
        eprintln!("Loading voice: {}", voice_path);

        let model = tokio::task::spawn_blocking(move || {
            KokoroModel::load(
                std::path::Path::new(&model_path),
                std::path::Path::new(&voice_path),
            )
        })
        .await
        .map_err(|e| format!("Task: {}", e))?
        .map_err(|e| format!("Load: {}", e))?;

        *self.backend.lock().unwrap() = Some(model);
        Ok(())
    }

    pub async fn speak(&self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioChunk>(8);

        // Store sender so stop() can drop it to close the channel
        {
            let mut tx_guard = self.cancel_tx.lock().unwrap();
            *tx_guard = Some(audio_tx.clone());
        }

        // Spawn synthesis thread (holds backend lock only during per-chunk synthesis)
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

            let chunks = Self::split_text(&text);
            eprintln!("Split into {} chunks", chunks.len());

            for (i, chunk) in chunks.iter().enumerate() {
                eprintln!(
                    "Synthesizing chunk {}/{}: {}",
                    i + 1,
                    chunks.len(),
                    &chunk[..chunk.len().min(60)]
                );
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

        // Spawn playback thread — OutputStream+Sink created locally (no Send issues)
        let audio_lock = self.audio_lock.clone();
        let play_handle = tokio::task::spawn_blocking(move || {
            use rodio::buffer::SamplesBuffer;
            use rodio::{OutputStream, Sink};

            let (_stream, handle) =
                OutputStream::try_default().expect("Failed to open audio output");
            let sink = Sink::try_new(&handle).expect("Failed to create audio sink");

            // Store sink so stop() can drop it
            {
                let mut guard = audio_lock.lock().unwrap();
                *guard = Some(sink);
            }

            loop {
                let chunk = audio_rx.blocking_recv();
                let mut guard = audio_lock.lock().unwrap();
                let sink = match guard.as_mut() {
                    Some(s) => s,
                    None => break, // stop() dropped the sink
                };

                match chunk {
                    Some(AudioChunk::Samples(samples)) => {
                        let i16_samples: Vec<i16> = samples
                            .iter()
                            .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                            .collect();
                        let buffer = SamplesBuffer::new(1, 24000, i16_samples);
                        sink.append(buffer);
                    }
                    _ => break,
                }
                drop(guard);
            }

            // Drain: wait for remaining audio to finish
            let sink = audio_lock.lock().unwrap().take();
            if let Some(sink) = sink {
                sink.sleep_until_end();
            }
        });

        let _ = synth_handle.await;
        let _ = play_handle.await;

        // Clear the cancel sender
        {
            let mut tx_guard = self.cancel_tx.lock().unwrap();
            *tx_guard = None;
        }

        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.backend
            .try_lock()
            .map(|b| b.is_some())
            .unwrap_or(false)
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut engine = self.system_tts.lock().await;
        engine.stop().map_err(|e| format!("Stop: {}", e))?;
        drop(engine);

        // Drop the cancel sender to close the channel → unblocks play task
        {
            let mut tx_guard = self.cancel_tx.lock().unwrap();
            *tx_guard = None;
        }

        // Drop the rodio sink to stop playback immediately
        {
            let mut audio = self.audio_lock.lock().unwrap();
            *audio = None;
        }

        Ok(())
    }
}

#[tauri::command]
pub async fn tts_speak(app: AppHandle, text: String) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.speak(&text).await
}

#[tauri::command]
pub async fn tts_stop(app: AppHandle) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.stop().await
}

#[tauri::command]
pub async fn tts_load_model(
    app: AppHandle,
    model_path: String,
    voice_path: String,
) -> Result<(), String> {
    let tts = app.state::<Arc<TtsManager>>();
    tts.load_model(&model_path, &voice_path).await
}

#[tauri::command]
pub async fn tts_model_loaded(app: AppHandle) -> Result<bool, String> {
    let tts = app.state::<Arc<TtsManager>>();
    let backend = tts.backend.lock().unwrap();
    Ok(backend.is_some())
}
