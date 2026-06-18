use serde::{Deserialize, Serialize};

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
        audio_path: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_item_text_message_creation() {
        let item = QueueItem::TextMessage {
            id: 1,
            text: "Hello world".to_string(),
            language: Some("en".to_string()),
            status: TextMessageStatus::Pending,
        };
        assert_eq!(item.id(), 1);
    }

    #[test]
    fn queue_item_speech_creation() {
        let item = QueueItem::Speech {
            id: 2,
            text: "Hello world".to_string(),
            audio_path: Some("/tmp/audio.wav".to_string()),
            voice_key: Some("en-us".to_string()),
            language: Some("en".to_string()),
            status: SpeechStatus::ToPlay,
        };
        assert_eq!(item.id(), 2);
    }

    #[test]
    fn queue_item_serialization_roundtrip() {
        let item = QueueItem::TextMessage {
            id: 1,
            text: "Test".to_string(),
            language: None,
            status: TextMessageStatus::Pending,
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: QueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }
}
