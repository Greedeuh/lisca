mod kokoro;
mod session;

use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;

use kokoro::KokoroModel;

pub struct TtsManager {
    backend: Arc<std::sync::Mutex<Option<KokoroModel>>>,
    loading: Arc<std::sync::atomic::AtomicBool>,
    system_tts: Mutex<tts::Tts>,
    _app_handle: AppHandle,
}

impl TtsManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let engine = tts::Tts::default().map_err(|e| format!("Failed to init system TTS: {}", e))?;

        let manager = Self {
            backend: Arc::new(std::sync::Mutex::new(None)),
            loading: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            system_tts: Mutex::new(engine),
            _app_handle: app_handle,
        };

        Ok(manager)
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
        // Start from current working directory (project root in dev mode)
        let cwd = std::env::current_dir().unwrap_or_default();

        // Check if cwd has models/ directory
        if cwd.join("models").exists() {
            return cwd;
        }

        // Check parent directories
        let mut path = cwd.clone();
        for _ in 0..5 {
            if path.join("models").exists() {
                return path;
            }
            if !path.pop() {
                break;
            }
        }

        // Fallback: current directory
        cwd
    }

    fn resolve_path(path: &str, project_root: &std::path::Path) -> String {
        let p = std::path::Path::new(path);
        if p.is_absolute() && p.exists() {
            return path.to_string();
        }
        // Try relative to project root
        let resolved = project_root.join(path);
        if resolved.exists() {
            return resolved.display().to_string();
        }
        // Return as-is and let the loader report the error
        path.to_string()
    }

    /// Load a Kokoro ONNX model and voice file.
    pub async fn load_model(&self, model_path: &str, voice_path: &str) -> Result<(), String> {
        let project_root = Self::find_project_root();

        // Resolve relative paths against project root
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

        let t0 = std::time::Instant::now();

        // Try Kokoro first
        let (audio, sample_rate) = {
            let mut backend = self.backend.lock().unwrap();
            if let Some(ref mut model) = *backend {
                let audio = model.synthesize(text, 1.0)?;
                let sr = model.sample_rate();
                (Some(audio), sr)
            } else {
                (None, 22050)
            }
        };

        let t1 = std::time::Instant::now();

        if let Some(audio) = audio {
            eprintln!("Synthesis: {}ms", t0.elapsed().as_millis());
            self.play_audio(&audio, sample_rate).await?;
            eprintln!("Total (synthesis+playback): {}ms", t1.elapsed().as_millis());
            return Ok(());
        }

        // Fallback to system TTS
        let mut engine = self.system_tts.lock().await;
        engine
            .speak(text, false)
            .map_err(|e| format!("System TTS: {}", e))?;
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.backend.try_lock()
            .map(|b| b.is_some())
            .unwrap_or(false)
    }

    async fn play_audio(&self, samples: &[f32], sample_rate: u32) -> Result<(), String> {
        let samples = samples.to_vec();
        tokio::task::spawn_blocking(move || {
            use rodio::buffer::SamplesBuffer;
            use rodio::{OutputStream, Sink};

            let (_stream, handle) = OutputStream::try_default()
                .map_err(|e| format!("Audio output: {}", e))?;
            let sink = Sink::try_new(&handle)
                .map_err(|e| format!("Audio sink: {}", e))?;

            // Convert f32 samples to i16 for rodio
            let i16_samples: Vec<i16> = samples
                .iter()
                .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();

            let buffer = SamplesBuffer::new(1, sample_rate, i16_samples);
            sink.append(buffer);
            sink.sleep_until_end();

            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Task: {}", e))?
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut engine = self.system_tts.lock().await;
        engine.stop().map_err(|e| format!("Stop: {}", e))?;
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
