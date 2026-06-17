use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use super::queue::{QueueConfig, QueueEvent, QueueItem};
use super::text::split_text;
use super::backend::{BackendPool, VoiceResolver};
use super::language;
use super::playback::{STATE_IDLE, STATE_PAUSED, STATE_PLAYING};

struct ProcessorState {
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    pool: Arc<std::sync::Mutex<BackendPool>>,
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
}

pub fn run_processor(
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    pool: Arc<std::sync::Mutex<BackendPool>>,
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

            // Inner loop processes items until queue is empty or auto_read is off.
            // should_exit: false = interrupted (stop/pause, re-wait), true = done (idle).
            let should_exit = 'outer: loop {
                if state.check_stop() {
                    break 'outer false;
                }
                if state.wait_if_paused().await {
                    break 'outer false;
                }

                let item = {
                    let mut q = state.queue.lock().await;
                    q.pop_front()
                };

                let item = match item {
                    Some(i) => i,
                    None => {
                        let q_ref = state.queue.lock().await;
                        state.emit_queue_updated(&q_ref);
                        drop(q_ref);
                        state.emit(QueueEvent::ProcessorIdle);
                        break 'outer true;
                    }
                };

                {
                    let q_ref = state.queue.lock().await;
                    state.emit_queue_updated(&q_ref);
                }

                state.emit(QueueEvent::PlaybackStarted {
                    item: item.clone(),
                });

                let pool_clone = state.pool.clone();
                let resolver_clone = state.resolver.clone();
                let item_lang = item.language.clone();
                let text = item.text.clone();
                let synth_result = match tokio::task::spawn_blocking(move || {
                    let voice_key = {
                        let resolver_guard = resolver_clone.lock().unwrap();
                        let lang = item_lang.as_deref().or_else(|| language::detect_language_family(&text));
                        resolver_guard.resolve_voice_key(lang)
                    };
                    let mut pool_guard = pool_clone.lock().unwrap();
                    let model = pool_guard.get_for_language(voice_key.as_deref());
                    let chunks = split_text(&text);
                    eprintln!("[processor] Synthesizing '{}' ({} chunks)", text, chunks.len());
                    let mut all_samples = Vec::new();
                    for (i, chunk) in chunks.iter().enumerate() {
                        eprintln!("[processor] Chunk {}: '{}' ({} chars)", i, chunk, chunk.len());
                        match model.synthesize(chunk, 1.0) {
                            Ok(samples) => {
                                eprintln!("[processor] Chunk {} done: {} samples", i, samples.len());
                                all_samples.extend(samples);
                            }
                            Err(e) => {
                                eprintln!("[processor] Chunk {} failed: {}", i, e);
                                return Err(e);
                            }
                        }
                    }
                    eprintln!("[processor] Total samples: {}", all_samples.len());
                    Ok(all_samples)
                })
                .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        eprintln!("Synthesis task panicked: {}", e);
                        Err(format!("Synthesis failed: {}", e))
                    }
                };

                match synth_result {
                    Ok(samples) => {
                        let sample_rate = {
                            let resolver_guard = state.resolver.lock().unwrap();
                            let lang = item.language.as_deref().or_else(|| language::detect_language_family(&item.text));
                            let voice_key = resolver_guard.resolve_voice_key(lang);
                            let guard = state.pool.lock().unwrap();
                            guard.sample_rate_for_language(voice_key.as_deref())
                        };
                        eprintln!("[processor] Playing {} samples at {}Hz", samples.len(), sample_rate);

                        state.playback_state.store(STATE_PLAYING.into(), Ordering::SeqCst);

                        let play_stop = state.stop_flag.clone();
                        let play_pause = state.pause_flag.clone();
                        let play_state = state.playback_state.clone();
                        let play_result = match tokio::task::spawn_blocking(move || {
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
                            Ok(result) => result,
                            Err(e) => {
                                eprintln!("Playback task panicked: {}", e);
                                state.playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                                continue;
                            }
                        };

                        // play_result: true = playback was interrupted (stop requested),
                        // false = playback completed normally.
                        if play_result {
                            state.emit(QueueEvent::PlaybackStopped);
                            continue;
                        }

                        state.playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                        {
                            let q_ref = state.queue.lock().await;
                            super::queue::save_queue(&state.app_data_dir, &q_ref)
                                .map_err(|e| eprintln!("Failed to save queue: {}", e))
                                .ok();
                            state.emit_queue_updated(&q_ref);
                        }
                        state.emit(QueueEvent::ItemCompleted {
                            id: item.id,
                        });

                        if !state.queue_config.lock().unwrap().auto_read {
                            state.emit(QueueEvent::ProcessorIdle);
                            break 'outer true;
                        }
                    }
                    Err(e) => {
                        state.emit(QueueEvent::Error {
                            id: Some(item.id),
                            message: e,
                        });

                        {
                            let mut q = state.queue.lock().await;
                            q.retain(|i| i.id != item.id);
                            state.emit_queue_updated(&q);
                            super::queue::save_queue(&state.app_data_dir, &q)
                                .map_err(|e| eprintln!("Failed to save queue: {}", e))
                                .ok();
                        }
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
