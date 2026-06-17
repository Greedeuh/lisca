/// Manages the TTS queue: adding/removing/moving items, persisting to disk,
/// and providing snapshots for the frontend.
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use super::language;
use super::queue::{QueueConfig, QueueItem, QueueSnapshot};

pub(crate) struct QueueStore {
    queue: Arc<tokio::sync::Mutex<VecDeque<QueueItem>>>,
    config: Arc<std::sync::Mutex<QueueConfig>>,
    next_id: Arc<std::sync::Mutex<u32>>,
    app_data_dir: PathBuf,
}

impl QueueStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let queue = super::queue::load_queue(&app_data_dir);
        let config = super::queue::load_queue_config(&app_data_dir);
        let next_id = queue.iter().map(|i| i.id).max().unwrap_or(0) + 1;

        Self {
            queue: Arc::new(tokio::sync::Mutex::new(queue)),
            config: Arc::new(std::sync::Mutex::new(config)),
            next_id: Arc::new(std::sync::Mutex::new(next_id)),
            app_data_dir,
        }
    }

    pub async fn add(&self, text: String) -> Result<QueueItem, String> {
        if text.trim().is_empty() {
            return Err("No text to speak".into());
        }

        let config = self.config.lock().unwrap().clone();
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

        let detected_lang = language::detect_language_family(&text)
            .map(|s| s.to_string());

        let item = QueueItem {
            id,
            text: text.trim().to_string(),
            language: detected_lang,
        };
        q.push_back(item.clone());

        self.save(&q);

        Ok(item)
    }

    pub async fn remove(&self, id: u32) {
        let mut q = self.queue.lock().await;
        q.retain(|i| i.id != id);
        self.save(&q);
    }

    pub async fn move_item(&self, id: u32, new_index: usize) {
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
        self.save(&q);
    }

    pub async fn clear(&self) {
        let mut q = self.queue.lock().await;
        q.clear();
        self.save(&q);
    }

    pub fn is_empty_sync(&self) -> bool {
        self.queue.try_lock().map(|q| q.is_empty()).unwrap_or(false)
    }

    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    pub async fn snapshot(&self) -> QueueSnapshot {
        let items = self.queue.lock().await;
        let config = self.config.lock().unwrap().clone();
        QueueSnapshot {
            items: items.iter().cloned().collect(),
            playback: super::queue::PlaybackState::Idle,
            auto_read: config.auto_read,
            show_overlay: config.show_overlay,
        }
    }

    pub fn get_config(&self) -> QueueConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn set_config(&self, config: QueueConfig) -> Result<(), String> {
        super::queue::save_queue_config(&self.app_data_dir, &config)?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }

    pub fn queue_arc(&self) -> Arc<tokio::sync::Mutex<VecDeque<QueueItem>>> {
        self.queue.clone()
    }

    pub fn config_arc(&self) -> Arc<std::sync::Mutex<QueueConfig>> {
        self.config.clone()
    }

    fn save(&self, queue: &VecDeque<QueueItem>) {
        super::queue::save_queue(&self.app_data_dir, queue)
            .map_err(|e| eprintln!("Failed to save queue: {}", e))
            .ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::queue::PlaybackState;

    fn temp_qm() -> (tempfile::TempDir, QueueStore) {
        let dir = tempfile::tempdir().unwrap();
        let qm = QueueStore::new(dir.path().to_path_buf());
        (dir, qm)
    }

    #[tokio::test]
    async fn add_returns_item_with_incrementing_ids() {
        let (_dir, qm) = temp_qm();
        let a = qm.add("hello".into()).await.unwrap();
        let b = qm.add("world".into()).await.unwrap();
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
    }

    #[tokio::test]
    async fn add_rejects_empty_text() {
        let (_dir, qm) = temp_qm();
        assert!(qm.add("".into()).await.is_err());
        assert!(qm.add("   ".into()).await.is_err());
    }

    #[tokio::test]
    async fn add_rejects_when_queue_full() {
        let (_dir, qm) = temp_qm();
        qm.set_config(QueueConfig { max_items: 2, ..Default::default() }).unwrap();
        qm.add("one".into()).await.unwrap();
        qm.add("two".into()).await.unwrap();
        assert!(qm.add("three".into()).await.is_err());
    }

    #[tokio::test]
    async fn add_detects_language() {
        let (_dir, qm) = temp_qm();
        let item = qm.add("Hello, this is a test sentence in English.".into()).await.unwrap();
        assert_eq!(item.language, Some("en".into()));
    }

    #[tokio::test]
    async fn remove_deletes_item() {
        let (_dir, qm) = temp_qm();
        let item = qm.add("hello".into()).await.unwrap();
        assert!(!qm.is_empty().await);
        qm.remove(item.id).await;
        assert!(qm.is_empty().await);
    }

    #[tokio::test]
    async fn move_item_reorders() {
        let (_dir, qm) = temp_qm();
        let a = qm.add("first".into()).await.unwrap();
        let _b = qm.add("second".into()).await.unwrap();
        let c = qm.add("third".into()).await.unwrap();

        // move first item (id=a.id) to end
        qm.move_item(a.id, 2).await;

        let snap = qm.snapshot().await;
        assert_eq!(snap.items[0].id, _b.id);
        assert_eq!(snap.items[1].id, c.id);
        assert_eq!(snap.items[2].id, a.id);
    }

    #[tokio::test]
    async fn move_item_clamps_index() {
        let (_dir, qm) = temp_qm();
        let a = qm.add("first".into()).await.unwrap();
        qm.add("second".into()).await.unwrap();

        // move first to index 100 — should clamp to last
        qm.move_item(a.id, 100).await;

        let snap = qm.snapshot().await;
        assert_eq!(snap.items[1].id, a.id);
    }

    #[tokio::test]
    async fn move_item_noop_same_position() {
        let (_dir, qm) = temp_qm();
        qm.add("first".into()).await.unwrap();
        let b = qm.add("second".into()).await.unwrap();

        // move second to index 1 (already there)
        qm.move_item(b.id, 1).await;

        let snap = qm.snapshot().await;
        assert_eq!(snap.items[0].text, "first");
        assert_eq!(snap.items[1].text, "second");
    }

    #[tokio::test]
    async fn clear_empties_queue() {
        let (_dir, qm) = temp_qm();
        qm.add("one".into()).await.unwrap();
        qm.add("two".into()).await.unwrap();
        assert!(!qm.is_empty().await);
        qm.clear().await;
        assert!(qm.is_empty().await);
    }

    #[tokio::test]
    async fn snapshot_returns_current_state() {
        let (_dir, qm) = temp_qm();
        qm.add("hello".into()).await.unwrap();
        let snap = qm.snapshot().await;
        assert_eq!(snap.items.len(), 1);
        assert!(matches!(snap.playback, PlaybackState::Idle));
        assert!(snap.auto_read);
        assert!(snap.show_overlay);
    }

    #[tokio::test]
    async fn set_config_persists() {
        let (_dir, qm) = temp_qm();
        let new_cfg = QueueConfig { max_items: 5, auto_read: false, show_overlay: false };
        qm.set_config(new_cfg.clone()).unwrap();
        assert_eq!(qm.get_config().max_items, 5);
        assert!(!qm.get_config().auto_read);
    }

    #[tokio::test]
    async fn new_loads_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        {
            let qm = QueueStore::new(dir.path().to_path_buf());
            qm.add("persisted".into()).await.unwrap();
        }
        // create new QueueStore from same dir
        let qm2 = QueueStore::new(dir.path().to_path_buf());
        let snap = qm2.snapshot().await;
        assert_eq!(snap.items.len(), 1);
        assert_eq!(snap.items[0].text, "persisted");
    }
}
