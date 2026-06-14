use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;

use crate::persist;

const QUEUE_FILE: &str = "queue.json";
const QUEUE_CONFIG_FILE: &str = "queue_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_items: usize,
    pub auto_read: bool,
    pub show_overlay: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_items: 50,
            auto_read: true,
            show_overlay: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
}

impl From<u8> for PlaybackState {
    fn from(v: u8) -> Self {
        match v {
            1 => PlaybackState::Playing,
            2 => PlaybackState::Paused,
            _ => PlaybackState::Idle,
        }
    }
}

impl From<PlaybackState> for u8 {
    fn from(s: PlaybackState) -> Self {
        s as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub items: Vec<QueueItem>,
    pub playback: PlaybackState,
    pub auto_read: bool,
    pub show_overlay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueEvent {
    PlaybackStarted {
        item: QueueItem,
    },
    ItemCompleted {
        id: u32,
    },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped,
    QueueUpdated {
        items: Vec<QueueItem>,
        auto_read: bool,
        show_overlay: bool,
    },
    Error {
        id: Option<u32>,
        message: String,
    },
    ProcessorIdle,
}

// --- File persistence ---

pub fn queue_file_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join("lisca").join(QUEUE_FILE)
}

    pub fn load_queue(app_data_dir: &Path) -> VecDeque<QueueItem> {
    let path = queue_file_path(app_data_dir);
    let items: Vec<QueueItem> = persist::load_json(&path);
    items.into()
}

pub fn save_queue(app_data_dir: &Path, queue: &VecDeque<QueueItem>) -> Result<(), String> {
    let path = queue_file_path(app_data_dir);
    let items: Vec<&QueueItem> = queue.iter().collect();
    persist::save_json(&path, &items)
}

pub fn queue_config_file_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join("lisca").join(QUEUE_CONFIG_FILE)
}

pub fn load_queue_config(app_data_dir: &Path) -> QueueConfig {
    let path = queue_config_file_path(app_data_dir);
    persist::load_json(&path)
}

pub fn save_queue_config(app_data_dir: &Path, config: &QueueConfig) -> Result<(), String> {
    let path = queue_config_file_path(app_data_dir);
    persist::save_json(&path, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_config_defaults() {
        let config = QueueConfig::default();
        assert_eq!(config.max_items, 50);
        assert!(config.auto_read);
        assert!(config.show_overlay);
    }

    #[test]
    fn queue_config_serde_roundtrip() {
        let config = QueueConfig { max_items: 10, auto_read: false, show_overlay: false };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: QueueConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_items, 10);
        assert!(!deserialized.auto_read);
        assert!(!deserialized.show_overlay);
    }

    #[test]
    fn queue_item_serde_roundtrip() {
        let item = QueueItem { id: 1, text: "hello world".into(), language: None };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: QueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.text, "hello world");
    }

    #[test]
    fn playback_state_conversions() {
        assert!(matches!(PlaybackState::from(0), PlaybackState::Idle));
        assert!(matches!(PlaybackState::from(1), PlaybackState::Playing));
        assert!(matches!(PlaybackState::from(2), PlaybackState::Paused));
        assert!(matches!(PlaybackState::from(99), PlaybackState::Idle));
        assert!(matches!(PlaybackState::from(255), PlaybackState::Idle));

        let u: u8 = PlaybackState::Idle.into();
        assert_eq!(u, 0);
        let u: u8 = PlaybackState::Playing.into();
        assert_eq!(u, 1);
        let u: u8 = PlaybackState::Paused.into();
        assert_eq!(u, 2);
    }

    #[test]
    fn queue_event_serde_all_variants() {
        let events = vec![
            QueueEvent::PlaybackStarted { item: QueueItem { id: 1, text: "hi".into(), language: None } },
            QueueEvent::ItemCompleted { id: 1 },
            QueueEvent::PlaybackPaused,
            QueueEvent::PlaybackResumed,
            QueueEvent::PlaybackStopped,
            QueueEvent::QueueUpdated { items: vec![], auto_read: true, show_overlay: true },
            QueueEvent::Error { id: Some(1), message: "fail".into() },
        ];

        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let deserialized: QueueEvent = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(json, json2, "Roundtrip failed for: {}", json);
        }
    }

    #[test]
    fn queue_event_type_tags() {
        let json = serde_json::to_string(&QueueEvent::PlaybackPaused).unwrap();
        assert!(json.contains("\"type\":\"playback_paused\""));

        let json = serde_json::to_string(&QueueEvent::PlaybackStopped).unwrap();
        assert!(json.contains("\"type\":\"playback_stopped\""));

        let json = serde_json::to_string(&QueueEvent::Error { id: None, message: "e".into() }).unwrap();
        assert!(json.contains("\"type\":\"error\""));
    }

    #[test]
    fn save_and_load_queue() {
        let dir = tempfile::tempdir().unwrap();
        let mut queue = VecDeque::new();
        queue.push_back(QueueItem { id: 1, text: "hello".into(), language: None });
        queue.push_back(QueueItem { id: 2, text: "world".into(), language: None });
        save_queue(dir.path(), &queue).unwrap();
        let loaded = load_queue(dir.path());
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].text, "hello");
        assert_eq!(loaded[1].text, "world");
    }

    #[test]
    fn load_empty_queue() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_queue(dir.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn save_and_load_queue_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = QueueConfig { max_items: 10, auto_read: false, show_overlay: true };
        save_queue_config(dir.path(), &config).unwrap();
        let loaded = load_queue_config(dir.path());
        assert_eq!(loaded.max_items, 10);
        assert!(!loaded.auto_read);
        assert!(loaded.show_overlay);
    }

    #[test]
    fn load_missing_queue_config_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_queue_config(dir.path());
        assert_eq!(loaded, QueueConfig::default());
    }
}
