mod kokoro;
mod session;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::AppHandle;
use tauri::Manager;

use kokoro::KokoroModel;

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
    backend: Arc<std::sync::Mutex<Option<KokoroModel>>>,
    audio: Arc<std::sync::Mutex<Option<AudioState>>>,
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

fn find_project_root() -> PathBuf {
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

impl TtsManager {
    pub fn new() -> Self {
        Self {
            backend: Arc::new(std::sync::Mutex::new(None)),
            audio: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn preload(&self) {
        let models_dir = find_project_root().join("models");

        let model_path = models_dir.join("kokoro-q8.onnx");
        let voice_path = models_dir.join("voices/af.bin");

        if !model_path.exists() || !voice_path.exists() {
            eprintln!("Model files not found, skipping preload");
            return;
        }

        if self.backend.try_lock().map(|b| b.is_some()).unwrap_or(true) {
            return;
        }

        let mp = model_path.display().to_string();
        let vp = voice_path.display().to_string();

        let backend = self.backend.clone();

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
        });
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
                        let buffer = SamplesBuffer::new(1, 24000, i16_samples);
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
pub async fn tts_stop(app: AppHandle) {
    let tts = app.state::<Arc<TtsManager>>();
    tts.stop();
}
