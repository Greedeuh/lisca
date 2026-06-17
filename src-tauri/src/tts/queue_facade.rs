use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use super::backend::{BackendPool, VoiceResolver};
use super::playback::PlaybackController;
use super::processor;
use super::queue::{QueueConfig, QueueEvent, QueueItem, QueueSnapshot};
use super::queue_manager::QueueManager;

pub(crate) struct QueueFacade {
    queue_mgr: QueueManager,
    pub playback: PlaybackController,
    processor_running: Arc<AtomicBool>,
    pool: Arc<std::sync::Mutex<BackendPool>>,
    resolver: Arc<std::sync::Mutex<VoiceResolver>>,
    app_data_dir: PathBuf,
    app_handle: AppHandle,
}

// TODO: explain why façade, why do we need a facade?
impl QueueFacade {
    pub fn new(
        queue_mgr: QueueManager,
        playback: PlaybackController,
        processor_running: Arc<AtomicBool>,
        pool: Arc<std::sync::Mutex<BackendPool>>,
        resolver: Arc<std::sync::Mutex<VoiceResolver>>,
        app_data_dir: PathBuf,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            queue_mgr,
            playback,
            processor_running,
            pool,
            resolver,
            app_data_dir,
            app_handle,
        }
    }

    pub async fn add(&self, text: String) -> Result<QueueItem, String> {
        let was_empty = self.queue_mgr.is_empty().await;
        let item = self.queue_mgr.add(text).await?;

        let config = self.queue_mgr.get_config();
        let q = self.queue_mgr.queue_arc();
        let items: Vec<QueueItem> = q.lock().await.iter().cloned().collect();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });

        if was_empty {
            self.spawn_processor_if_needed();
            self.playback.notify().notify_one();
        }

        self.sync_overlay(true);

        Ok(item)
    }

    pub async fn remove(&self, id: u32) {
        self.queue_mgr.remove(id).await;
        let config = self.queue_mgr.get_config();
        let q = self.queue_mgr.queue_arc();
        let items: Vec<QueueItem> = q.lock().await.iter().cloned().collect();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    pub async fn move_item(&self, id: u32, new_index: usize) {
        self.queue_mgr.move_item(id, new_index).await;
        let config = self.queue_mgr.get_config();
        let q = self.queue_mgr.queue_arc();
        let items: Vec<QueueItem> = q.lock().await.iter().cloned().collect();
        self.emit_event(QueueEvent::QueueUpdated {
            items,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
    }

    pub async fn clear(&self) {
        self.playback.stop();
        self.queue_mgr.clear().await;
        let config = self.queue_mgr.get_config();
        self.emit_event(QueueEvent::QueueUpdated {
            items: vec![],
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        });
        self.emit_event(QueueEvent::PlaybackStopped);
    }

    pub async fn state(&self) -> QueueSnapshot {
        let mut snap = self.queue_mgr.snapshot().await;
        snap.playback = self.playback.playback_state();
        snap
    }

    pub fn stop(&self) {
        self.playback.stop();
    }

    pub fn pause(&self) {
        self.playback.pause();
        if !self.playback.is_idle() {
            self.emit_event(QueueEvent::PlaybackPaused);
        }
    }

    pub fn resume(&self) {
        if !self.playback.is_idle() {
            self.playback.resume();
            self.emit_event(QueueEvent::PlaybackResumed);
        } else {
            let has_items = !self.queue_mgr.is_empty_sync();
            if has_items {
                self.spawn_processor_if_needed();
                self.playback.notify().notify_one();
            }
        }
    }

    pub fn get_config(&self) -> QueueConfig {
        self.queue_mgr.get_config()
    }

    pub fn set_config(&self, config: QueueConfig) -> Result<(), String> {
        self.queue_mgr.set_config(config)
    }

    // --- Internal ---

    fn is_main_window_visible(&self) -> bool {
        self.app_handle
            .get_webview_window("main")
            .map(|w| w.is_visible().unwrap_or(true))
            .unwrap_or(true)
    }

    // TODO: what's the goal of this function, need explanation, maybe we can rename it because we don't get it directly from the name
    fn sync_overlay(&self, has_items: bool) {
        if self.is_main_window_visible() {
            return;
        }
        let show = self.queue_mgr.get_config().show_overlay;
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
            self.queue_mgr.queue_arc(),
            self.queue_mgr.config_arc(),
            self.pool.clone(),
            self.resolver.clone(),
            self.app_data_dir.clone(),
            self.playback.stop_flag(),
            self.playback.pause_flag(),
            self.playback.state_arc(),
            self.playback.notify(),
            self.processor_running.clone(),
            self.app_handle.clone(),
        );
    }
}
