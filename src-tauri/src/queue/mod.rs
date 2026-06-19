// In-memory queue of TextMessage and Speech items.
// Items are not persisted — only config is saved to disk.
// Exposes consumer-specific traits: QueueControllable, Transcribable, Playable.

mod playable;
mod controllable;
mod transcribable;

pub use controllable::QueueControllable;
pub use playable::Playable;
pub use transcribable::Transcribable;

use crate::persist::{load_json, save_json};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TextMessageStatus {
    Pending,
    Processing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpeechStatus {
    ToPlay,
    Playing,
    Paused,
    Played,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum QueueItem {
    TextMessage {
        id: u64,
        text: String,
        language: Option<String>,
        status: TextMessageStatus,
    },
    Speech {
        id: u64,
        text: String,
        audio_data: Option<Vec<f32>>,
        voice_key: Option<String>,
        language: Option<String>,
        status: SpeechStatus,
    },
}

impl QueueItem {
    pub fn id(&self) -> u64 {
        match self {
            QueueItem::TextMessage { id, .. } => *id,
            QueueItem::Speech { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueEvent {
    ItemAdded,
    ItemRemoved,
    ItemMoved,
    ItemCleared,
    ItemReplaced,
}

pub struct Queue {
    pub(super) items: Vec<QueueItem>,
    pub(super) next_id: u64,
    pub(super) config: QueueConfig,
    on_event: Option<Box<dyn Fn(QueueEvent) + Send + Sync>>,
    config_path: Option<PathBuf>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            config: QueueConfig::default(),
            on_event: None,
            config_path: None,
        }
    }

    pub fn with_config(mut self, config: QueueConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub fn with_event_handler<F: Fn(QueueEvent) + Send + Sync + 'static>(
        mut self,
        handler: F,
    ) -> Self {
        self.on_event = Some(Box::new(handler));
        self
    }

    pub fn save_config(&self) -> Result<(), String> {
        let path = self
            .config_path
            .as_ref()
            .ok_or("no config path configured")?;
        save_json(path, &self.config)
    }

    pub fn load_config(path: &Path) -> QueueConfig {
        load_json(path)
    }

    pub(super) fn emit(&self, event: QueueEvent) {
        if let Some(ref handler) = self.on_event {
            handler(event);
        }
    }
}
