use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use super::queue::{QueueConfig, QueueEvent, QueueItem};
use super::text::split_text;
use super::{TtsBackend, DEFAULT_SAMPLE_RATE, I16_SAMPLE_SCALE, STATE_IDLE, STATE_PAUSED, STATE_PLAYING};

fn emit_queue_updated(app: &AppHandle, queue: &VecDeque<QueueItem>, config: &QueueConfig) {
    let items: Vec<QueueItem> = queue.iter().cloned().collect();
    app.emit("tts-queue-event", &QueueEvent::QueueUpdated {
        items,
        auto_read: config.auto_read,
        show_overlay: config.show_overlay,
    }).ok();
}

pub fn run_processor(
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    queue_config: Arc<std::sync::Mutex<QueueConfig>>,
    backend: Arc<std::sync::Mutex<Option<Box<dyn TtsBackend>>>>,
    app_data_dir: std::path::PathBuf,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    playback_state: Arc<AtomicU8>,
    notify: Arc<tokio::sync::Notify>,
    processor_running: Arc<AtomicBool>,
    app_handle: AppHandle,
) {
    tokio::spawn(async move {
        loop {
            notify.notified().await;

            let should_exit = 'outer: loop {
                if stop_flag.load(Ordering::SeqCst) {
                    stop_flag.store(false, Ordering::SeqCst);
                    pause_flag.store(false, Ordering::SeqCst);
                    playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                    break 'outer false;
                }

                if pause_flag.load(Ordering::SeqCst) {
                    playback_state.store(STATE_PAUSED.into(), Ordering::SeqCst);
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        if stop_flag.load(Ordering::SeqCst) || !pause_flag.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                    if stop_flag.load(Ordering::SeqCst) {
                        stop_flag.store(false, Ordering::SeqCst);
                        pause_flag.store(false, Ordering::SeqCst);
                        playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                        break 'outer false;
                    }
                    pause_flag.store(false, Ordering::SeqCst);
                    playback_state.store(STATE_PLAYING.into(), Ordering::SeqCst);
                }

                let item = {
                    let mut q = queue.lock().await;
                    q.pop_front()
                };

                let item = match item {
                    Some(i) => i,
                    None => {
                        let config = queue_config.lock().unwrap().clone();
                        let q_ref = queue.lock().await;
                        emit_queue_updated(&app_handle, &q_ref, &config);
                        drop(q_ref);
                        if config.show_overlay {
                            let main_visible = app_handle
                                .get_webview_window("main")
                                .map(|w| w.is_visible().unwrap_or(true))
                                .unwrap_or(true);
                            if !main_visible {
                                crate::overlay::hide_overlay(&app_handle);
                            }
                        }
                        break 'outer true;
                    }
                };

                {
                    let config = queue_config.lock().unwrap().clone();
                    let q_ref = queue.lock().await;
                    emit_queue_updated(&app_handle, &q_ref, &config);
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

                        playback_state.store(STATE_PLAYING.into(), Ordering::SeqCst);

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
                                    play_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                                    drop(sink);
                                    drop(_stream);
                                    return true;
                                }

                                if play_pause.load(Ordering::SeqCst) {
                                    sink.pause();
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
                                        drop(sink);
                                        drop(_stream);
                                        return true;
                                    }
                                    play_pause.store(false, Ordering::SeqCst);
                                    play_state.store(STATE_PLAYING.into(), Ordering::SeqCst);
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

                        playback_state.store(STATE_IDLE.into(), Ordering::SeqCst);
                        {
                            let config = queue_config.lock().unwrap().clone();
                            let q_ref = queue.lock().await;
                            super::queue::save_queue(&app_data_dir, &q_ref)
                                .map_err(|e| eprintln!("Failed to save queue: {}", e))
                                .ok();
                            emit_queue_updated(&app_handle, &q_ref, &config);
                        }
                        app_handle.emit("tts-queue-event", &QueueEvent::ItemCompleted {
                            id: item.id,
                        }).ok();

                        if !queue_config.lock().unwrap().auto_read {
                            let cfg = queue_config.lock().unwrap().clone();
                            if cfg.show_overlay {
                                let main_visible = app_handle
                                    .get_webview_window("main")
                                    .map(|w| w.is_visible().unwrap_or(true))
                                    .unwrap_or(true);
                                if !main_visible {
                                    crate::overlay::hide_overlay(&app_handle);
                                }
                            }
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
                            emit_queue_updated(&app_handle, &q, &config);
                            super::queue::save_queue(&app_data_dir, &q)
                                .map_err(|e| eprintln!("Failed to save queue: {}", e))
                                .ok();
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
