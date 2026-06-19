use super::{Queue, QueueEvent, QueueItem, SpeechStatus};

pub trait Playable {
    fn next_to_play_speech(&self) -> Option<(usize, u64)>;
    fn set_speech_status(&mut self, id: u64, status: SpeechStatus) -> Result<(), String>;
}

impl Playable for Queue {
    fn next_to_play_speech(&self) -> Option<(usize, u64)> {
        self.items
            .iter()
            .enumerate()
            .find(|(_, item)| {
                matches!(
                    item,
                    QueueItem::Speech {
                        status: SpeechStatus::ToPlay,
                        ..
                    }
                )
            })
            .map(|(i, item)| (i, item.id()))
    }

    fn set_speech_status(&mut self, id: u64, status: SpeechStatus) -> Result<(), String> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id() == id)
            .ok_or_else(|| format!("item with id {id} not found"))?;
        match item {
            QueueItem::Speech {
                status: s, ..
            } => {
                *s = status;
            }
            _ => return Err("item is not a Speech".to_string()),
        }
        self.emit(QueueEvent::ItemReplaced);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{QueueControllable, Transcribable};

    #[test]
    fn next_to_play_finds_first_to_play() {
        let mut q = Queue::new();
        q.add_text("a".to_string()).unwrap();
        q.add_text("b".to_string()).unwrap();

        q.replace_with_speech(1, None, None, None)
            .unwrap();
        q.replace_with_speech(2, None, None, None)
            .unwrap();

        q.set_speech_status(1, SpeechStatus::Playing).unwrap();

        let next = q.next_to_play_speech();
        assert_eq!(next, Some((1, 2)));
    }

    #[test]
    fn next_to_play_returns_none_when_empty() {
        let q = Queue::new();
        assert_eq!(q.next_to_play_speech(), None);
    }

    #[test]
    fn next_to_play_returns_none_when_all_played() {
        let mut q = Queue::new();
        q.add_text("a".to_string()).unwrap();
        q.add_text("b".to_string()).unwrap();
        q.replace_with_speech(1, None, None, None)
            .unwrap();
        q.replace_with_speech(2, None, None, None)
            .unwrap();
        q.set_speech_status(1, SpeechStatus::Played).unwrap();
        q.set_speech_status(2, SpeechStatus::Played).unwrap();
        assert_eq!(q.next_to_play_speech(), None);
    }
}
