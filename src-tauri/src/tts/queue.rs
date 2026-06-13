use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;

const QUEUE_FILE: &str = "queue.json";
const QUEUE_CONFIG_FILE: &str = "queue_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_size: usize,
    pub auto_read: bool,
    pub show_overlay: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size: 50,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub items: Vec<QueueItem>,
    pub playback: PlaybackState,
    pub current: Option<QueueItem>,
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
}

// --- File persistence ---

pub fn queue_file_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join("lisca").join(QUEUE_FILE)
}

pub fn load_queue(app_data_dir: &Path) -> VecDeque<QueueItem> {
    let path = queue_file_path(app_data_dir);
    if !path.exists() {
        return VecDeque::new();
    }
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return VecDeque::new(),
    };
    let items: Vec<QueueItem> = serde_json::from_str(&data).unwrap_or_default();
    items.into()
}

pub fn save_queue(app_data_dir: &Path, queue: &VecDeque<QueueItem>) -> Result<(), String> {
    let path = queue_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let items: Vec<&QueueItem> = queue.iter().collect();
    let data = serde_json::to_string_pretty(&items).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn queue_config_file_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join("lisca").join(QUEUE_CONFIG_FILE)
}

pub fn load_queue_config(app_data_dir: &Path) -> QueueConfig {
    let path = queue_config_file_path(app_data_dir);
    if !path.exists() {
        return QueueConfig::default();
    }
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return QueueConfig::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_queue_config(app_data_dir: &Path, config: &QueueConfig) -> Result<(), String> {
    let path = queue_config_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}
