// In-memory queue of TextMessage and Speech items.
// Items are not persisted — only config is saved to disk.
// Exposes consumer-specific traits: QueueControllable, Transcribable, Playable.

mod playable;
mod controllable;
mod transcribable;

pub(super)  use controllable::QueueControllable;
pub(super)  use playable::Playable;
pub(super)  use transcribable::Transcribable;

use crate::persist::{load_json, save_json};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super)  enum TextMessageStatus {
    Pending,
    Processing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super)  enum SpeechStatus {
    ToPlay,
    Playing,
    Paused,
    Played,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub(super)  enum QueueItem {
    TextMessage {
        id: u64,
        text: String,
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
    pub(super)  fn id(&self) -> u64 {
        match self {
            QueueItem::TextMessage { id, .. } => *id,
            QueueItem::Speech { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super)  struct QueueConfig {
     max_items: usize,
    pub(super)  show_overlay: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_items: 50,
            show_overlay: true,
        }
    }
}

pub(super)  struct Queue {
     items: Vec<QueueItem>,
     next_id: u64,
    pub(super) config: QueueConfig,
    config_path: Option<PathBuf>,
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    pub(super)  fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            config: QueueConfig::default(),
            config_path: None,
        }
    }

    pub(super)  fn with_config(mut self, config: QueueConfig) -> Self {
        self.config = config;
        self
    }

    pub(super)  fn with_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub(super)  fn save_config(&self) -> Result<(), String> {
        let path = self
            .config_path
            .as_ref()
            .ok_or("no config path configured")?;
        save_json(path, &self.config)
    }

    pub(super)  fn load_config(path: &Path) -> QueueConfig {
        load_json(path)
    }

    pub(super)  fn snapshot_dto(&self) -> crate::commands::QueueSnapshotDto {
        crate::commands::QueueSnapshotDto {
            items: self.items.iter().map(crate::commands::QueueItemDto::from).collect(),
            show_overlay: self.config.show_overlay,
        }
    }
}
