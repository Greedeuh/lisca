/// Background processor that dequeues TTS items, synthesizes audio, and plays
/// it through the audio output. Runs as a tokio task, woken by the playback
/// notify when new items are added.
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use super::queue::{QueueConfig, QueueEvent, QueueItem};
use super::text::split_text;
use super::model::{ModelPool, VoiceResolver};
use super::language;
use super::playback::{STATE_IDLE, STATE_PAUSED, STATE_PLAYING};

struct ProcessorState {
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    pool: Arc<std::sync::Mutex<ModelPool>>,
    resolver: Arc<std::sync::Mutex<VoiceResolver>>,
    app_data_dir: PathBuf,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    playback_state: Arc<AtomicU8>,
    notify: Arc<tokio::sync::Notify>,
    processor_running: Arc<AtomicBool>,
    app_handle: AppHandle,
}

impl ProcessorState {
    fn check_stop(&self) -> bool {
        if self.stop_flag.load(Ordering::SeqCst) {
            self.reset_to_idle();
            self.stop_flag.store(false, Ordering::SeqCst);
            return true;
        }
        false
    }

    async fn wait_if_paused(&self) -> bool {
        if !self.pause_flag.load(Ordering::SeqCst) {
            return false;
        }
        self.playback_state.store(STATE_PAUSED.into(), Ordering::SeqCst);
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if self.stop_flag.load(Ordering::SeqCst) || !self.pause_flag.load(Ordering::SeqCst) {
                break;
            }
        }
        if self.stop_flag.load(Ordering::SeqCst) {
            self.reset_to_idle();
            self.stop_flag.store(false, Ordering::SeqCst);
            return true;
        }
        self.pause_flag.store(false, Ordering::SeqCst);
        self.playback_state.store(STATE_PLAYING.into(), Ordering::SeqCst);
        false
    }

    fn reset_to_idle(&self) {
        self.pause_flag.store(false, Ordering::SeqCst);
        self.playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
    }

    fn emit(&self, event: QueueEvent) {
        self.app_handle.emit("tts-queue-event", &event).ok();
    }

    fn emit_queue_updated(&self, queue: &VecDeque<QueueItem>) {
        let config = self.queue_config.lock().unwrap().clone();
        let items: Vec<QueueItem> = queue.iter().cloned().collect();
        self.emit(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    /// Pop the next item from the queue and emit a QueueUpdated event.
    async fn dequeue_item(&self) -> Option<QueueItem> {
        let item = self.queue.lock().await.pop_front();
        let q_ref = self.queue.lock().await;
        self.emit_queue_updated(&q_ref);
        drop(q_ref);
        item
    }

    /// Resolve the voice key and sample rate for a queue item.
    fn resolve_voice_for_item(&self, item: &QueueItem) -> (Option<String>, u32) {
        let voice_key = {
            let resolver_guard = self.resolver.lock().unwrap();
            let lang = item.language.as_deref().or_else(|| language::detect_language_family(&item.text));
            resolver_guard.resolve_voice_key(lang)
        };
        let sample_rate = {
            let guard = self.pool.lock().unwrap();
            guard.sample_rate_for_language(voice_key.as_deref())
        };
        (voice_key, sample_rate)
    }

    /// Synthesize all text chunks for a queue item, returning concatenated PCM samples.
    async fn synthesize_item(&self, item: &QueueItem) -> Result<Vec<f32>, String> {
        let pool_clone = self.pool.clone();
        let resolver_clone = self.resolver.clone();
        let item_lang = item.language.clone();
        let text = item.text.clone();

        tokio::task::spawn_blocking(move || {
            let voice_key = {
                let resolver_guard = resolver_clone.lock().unwrap();
                let lang = item_lang.as_deref().or_else(|| language::detect_language_family(&text));
                resolver_guard.resolve_voice_key(lang)
            };
            let mut pool_guard = pool_clone.lock().unwrap();
            let model = pool_guard.get_model_for_language(voice_key.as_deref());
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
        .map_err(|e| format!("Synthesis task panicked: {}", e))?
    }

    /// Play audio samples with pause/stop support. Returns true if playback
    /// was interrupted (stop requested), false if it completed normally.
    async fn play_item(&self, samples: Vec<f32>, sample_rate: u32) -> bool {
        self.playback_state.store(STATE_PLAYING.into(), Ordering::SeqCst);

        let play_stop = self.stop_flag.clone();
        let play_pause = self.pause_flag.clone();
        let play_state = self.playback_state.clone();

        match tokio::task::spawn_blocking(move || {
            let output = match super::audio::AudioOutput::try_new() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{}", e);
                    play_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                    return true;
                }
            };

            let i16_samples = super::audio::f32_to_i16(&samples);
            output.play_buffer(i16_samples, sample_rate);

            loop {
                std::thread::sleep(std::time::Duration::from_millis(50));

                if play_stop.load(Ordering::SeqCst) {
                    play_stop.store(false, Ordering::SeqCst);
                    play_pause.store(false, Ordering::SeqCst);
                    play_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                    return true;
                }

                if play_pause.load(Ordering::SeqCst) {
                    output.pause();
                    play_state.store(STATE_PAUSED.into(), Ordering::SeqCst);
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if play_stop.load(Ordering::SeqCst) || !play_pause.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                    if play_stop.load(Ordering::SeqCst) {
                        play_stop.store(false, Ordering::SeqCst);
                        play_pause.store(false, Ordering::SeqCst);
                        play_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                        return true;
                    }
                    play_pause.store(false, Ordering::SeqCst);
                    play_state.store(STATE_PLAYING.into(), Ordering::SeqCst);
                    output.play();
                }

                if output.is_empty() {
                    break;
                }
            }

            output.sleep_until_end();
            false
        })
        .await
        {
            Ok(interrupted) => interrupted,
            Err(e) => {
                eprintln!("Playback task panicked: {}", e);
                self.playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                true
            }
        }
    }

    /// Handle successful playback completion: save queue, emit events.
    async fn on_item_completed(&self, item: &QueueItem) {
        self.playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
        let q_ref = self.queue.lock().await;
        super::queue::save_queue(&self.app_data_dir, &q_ref)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
        self.emit_queue_updated(&q_ref);
        self.emit(QueueEvent::ItemCompleted { id: item.id });
    }

    /// Handle synthesis/playback error: emit error, remove item, save queue.
    async fn on_item_error(&self, item: &QueueItem, error: String) {
        self.emit(QueueEvent::Error {
            id: Some(item.id),
            message: error,
        });
        let mut q = self.queue.lock().await;
        q.retain(|i| i.id != item.id);
        self.emit_queue_updated(&q);
        super::queue::save_queue(&self.app_data_dir, &q)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
    }
}

pub fn run_processor(
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    pool: Arc<std::sync::Mutex<ModelPool>>,
    resolver: Arc<std::sync::Mutex<VoiceResolver>>,
    app_data_dir: PathBuf,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    playback_state: Arc<AtomicU8>,
    notify: Arc<tokio::sync::Notify>,
    processor_running: Arc<AtomicBool>,
    app_handle: AppHandle,
) {
    let state = ProcessorState {
        queue,
        queue_config,
        pool,
        resolver,
        app_data_dir,
        stop_flag,
        pause_flag,
        playback_state,
        notify,
        processor_running,
        app_handle,
    };

    tokio::spawn(async move {
        loop {
            state.notify.notified().await;

            let should_exit = 'outer: loop {
                if state.check_stop() {
                    break 'outer false;
                }
                if state.wait_if_paused().await {
                    break 'outer false;
                }

                let item = match state.dequeue_item().await {
                    Some(i) => i,
                    None => {
                        state.emit(QueueEvent::ProcessorIdle);
                        break 'outer true;
                    }
                };

                state.emit(QueueEvent::PlaybackStarted { item: item.clone() });

                match state.synthesize_item(&item).await {
                    Ok(samples) => {
                        let (_, sample_rate) = state.resolve_voice_for_item(&item);

                        if state.play_item(samples, sample_rate).await {
                            state.emit(QueueEvent::PlaybackStopped);
                            continue;
                        }

                        state.on_item_completed(&item).await;

                        if !state.queue_config.lock().unwrap().auto_read {
                            state.emit(QueueEvent::ProcessorIdle);
                            break 'outer true;
                        }
                    }
                    Err(e) => {
                        state.on_item_error(&item, e).await;
                        continue;
                    }
                }
            };

            state.processor_running.store(false, Ordering::SeqCst);

            if should_exit {
                break;
            }
        }
    });
}
