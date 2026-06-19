use super::{Queue, QueueEvent, QueueItem, SpeechStatus, TextMessageStatus};

pub trait Transcribable {
    fn next_pending_text_message(&self) -> Option<(usize, u64)>;
    fn set_text_message_status(
        &mut self,
        id: u64,
        status: TextMessageStatus,
    ) -> Result<(), String>;
    fn replace_with_speech(
        &mut self,
        id: u64,
        audio_path: Option<String>,
        voice_key: Option<String>,
        language: Option<String>,
    ) -> Result<(), String>;
}

impl Transcribable for Queue {
    fn next_pending_text_message(&self) -> Option<(usize, u64)> {
        self.items
            .iter()
            .enumerate()
            .find(|(_, item)| {
                matches!(
                    item,
                    QueueItem::TextMessage {
                        status: TextMessageStatus::Pending,
                        ..
                    }
                )
            })
            .map(|(i, item)| (i, item.id()))
    }

    fn set_text_message_status(
        &mut self,
        id: u64,
        status: TextMessageStatus,
    ) -> Result<(), String> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id() == id)
            .ok_or_else(|| format!("item with id {id} not found"))?;
        match item {
            QueueItem::TextMessage {
                status: s, ..
            } => {
                *s = status;
            }
            _ => return Err("item is not a TextMessage".to_string()),
        }
        self.emit(QueueEvent::ItemReplaced);
        Ok(())
    }

    fn replace_with_speech(
        &mut self,
        id: u64,
        audio_path: Option<String>,
        voice_key: Option<String>,
        language: Option<String>,
    ) -> Result<(), String> {
        let index = self
            .items
            .iter()
            .position(|item| item.id() == id)
            .ok_or_else(|| format!("item with id {id} not found"))?;

        let old_text = match &self.items[index] {
            QueueItem::TextMessage { text, .. } => text.clone(),
            _ => return Err("item is not a TextMessage".to_string()),
        };

        self.items[index] = QueueItem::Speech {
            id,
            text: old_text,
            audio_path,
            voice_key,
            language,
            status: SpeechStatus::ToPlay,
        };
        self.emit(QueueEvent::ItemReplaced);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::QueueControllable;

    #[test]
    fn replace_preserves_position() {
        let mut q = Queue::new();
        q.add_text("before".to_string()).unwrap();
        let id = q.add_text("target".to_string()).unwrap();
        q.add_text("after".to_string()).unwrap();

        q.replace_with_speech(
            id,
            Some("/tmp/audio.wav".to_string()),
            Some("en-us".to_string()),
            Some("en".to_string()),
        )
        .unwrap();

        assert_eq!(q.items().len(), 3);
        assert_eq!(q.items()[0].id(), 1);
        assert_eq!(q.items()[1].id(), id);
        assert_eq!(q.items()[2].id(), 3);
        match &q.items()[1] {
            QueueItem::Speech {
                audio_path,
                voice_key,
                language,
                status,
                ..
            } => {
                assert_eq!(audio_path.as_deref(), Some("/tmp/audio.wav"));
                assert_eq!(voice_key.as_deref(), Some("en-us"));
                assert_eq!(language.as_deref(), Some("en"));
                assert_eq!(status, &SpeechStatus::ToPlay);
            }
            _ => panic!("expected Speech"),
        }
    }

    #[test]
    fn replace_wrong_id_fails() {
        let mut q = Queue::new();
        q.add_text("hello".to_string()).unwrap();
        assert!(q
            .replace_with_speech(999, Some("/tmp/a.wav".into()), None, None)
            .is_err());
    }

    #[test]
    fn replace_non_text_message_fails() {
        let mut q = Queue::new();
        q.add_text("hello".to_string()).unwrap();
        q.replace_with_speech(1, Some("/tmp/a.wav".into()), None, None)
            .unwrap();
        assert!(q
            .replace_with_speech(1, Some("/tmp/b.wav".into()), None, None)
            .is_err());
    }

    #[test]
    fn next_pending_finds_first_pending() {
        let mut q = Queue::new();
        let id1 = q.add_text("first".to_string()).unwrap();
        let id2 = q.add_text("second".to_string()).unwrap();
        q.add_text("third".to_string()).unwrap();

        q.set_text_message_status(id2, TextMessageStatus::Processing)
            .unwrap();

        let next = q.next_pending_text_message();
        assert_eq!(next, Some((0, id1)));
    }

    #[test]
    fn next_pending_returns_none_when_empty() {
        let q = Queue::new();
        assert_eq!(q.next_pending_text_message(), None);
    }

    #[test]
    fn next_pending_returns_none_when_all_processing() {
        let mut q = Queue::new();
        let id1 = q.add_text("a".to_string()).unwrap();
        let id2 = q.add_text("b".to_string()).unwrap();
        q.set_text_message_status(id1, TextMessageStatus::Processing)
            .unwrap();
        q.set_text_message_status(id2, TextMessageStatus::Processing)
            .unwrap();
        assert_eq!(q.next_pending_text_message(), None);
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        use std::fs;

        let dir = std::env::temp_dir().join("lisca_queue_test");
        let path = dir.join("config_roundtrip.json");
        let _ = fs::remove_file(&path);
        fs::create_dir_all(&dir).unwrap();

        let config = super::super::QueueConfig {
            max_items: 10,
            auto_read: false,
            show_overlay: false,
        };
        let q = Queue::new()
            .with_config(config.clone())
            .with_config_path(path.clone());
        q.save_config().unwrap();

        let loaded = Queue::load_config(&path);
        assert_eq!(loaded, config);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_config_returns_default() {
        let path = std::env::temp_dir().join("nonexistent_config.json");
        let config = Queue::load_config(&path);
        assert_eq!(config, super::super::QueueConfig::default());
    }

    #[test]
    fn load_corrupt_config_returns_default() {
        use std::fs;

        let dir = std::env::temp_dir().join("lisca_queue_test");
        let path = dir.join("corrupt_config.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "not valid json {{{").unwrap();

        let config = Queue::load_config(&path);
        assert_eq!(config, super::super::QueueConfig::default());
        let _ = fs::remove_file(&path);
    }
}
