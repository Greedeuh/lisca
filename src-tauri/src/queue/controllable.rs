// Frontend-facing queue operations: add, remove, reorder, clear.
// Enforces max_items limit and emits fine-grained events on mutations.

use super::{Queue, QueueItem};

pub(crate) trait QueueControllable {
    fn items(&self) -> &[QueueItem];
    fn add_text(&mut self, text: String) -> Result<u64, String>;
    fn remove(&mut self, id: u64) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
    fn reorder(&mut self, id: u64, new_index: usize) -> Result<(), String>;
}

impl QueueControllable for Queue {
    fn items(&self) -> &[QueueItem] {
        &self.items
    }

    fn add_text(&mut self, text: String) -> Result<u64, String> {
        if self.config.max_items > 0 && self.items.len() >= self.config.max_items {
            return Err("queue is full".to_string());
        }
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(super::QueueItem::TextMessage {
            id,
            text,
            status: super::TextMessageStatus::Pending,
        });
        Ok(id)
    }

    fn remove(&mut self, id: u64) -> Result<(), String> {
        let before = self.items.len();
        self.items.retain(|item| item.id() != id);
        if self.items.len() == before {
            return Err(format!("item with id {id} not found"));
        }
        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        self.items.clear();
        Ok(())
    }

    fn reorder(&mut self, id: u64, new_index: usize) -> Result<(), String> {
        let old_index = self
            .items
            .iter()
            .position(|item| item.id() == id)
            .ok_or_else(|| format!("item with id {id} not found"))?;

        let clamped = new_index.min(self.items.len() - 1);
        if old_index == clamped {
            return Ok(());
        }
        let item = self.items.remove(old_index);
        self.items.insert(clamped, item);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::TextMessageStatus;

    #[test]
    fn add_text_creates_item() {
        let mut q = Queue::new();
        let id = q.add_text("hello".to_string()).unwrap();
        assert_eq!(id, 1);
        match &q.items()[0] {
            QueueItem::TextMessage { text, status, .. } => {
                assert_eq!(text, "hello");
                assert_eq!(*status, TextMessageStatus::Pending);
            }
            _ => panic!("expected TextMessage"),
        }
    }

    #[test]
    fn add_multiple_items_in_order() {
        let mut q = Queue::new();
        let id1 = q.add_text("first".to_string()).unwrap();
        let id2 = q.add_text("second".to_string()).unwrap();
        let id3 = q.add_text("third".to_string()).unwrap();
        assert_eq!(q.items().len(), 3);
        assert_eq!(q.items()[0].id(), id1);
        assert_eq!(q.items()[1].id(), id2);
        assert_eq!(q.items()[2].id(), id3);
    }

    #[test]
    fn remove_item() {
        let mut q = Queue::new();
        let id1 = q.add_text("first".to_string()).unwrap();
        let id2 = q.add_text("second".to_string()).unwrap();
        q.remove(id1).unwrap();
        assert_eq!(q.items().len(), 1);
        assert_eq!(q.items()[0].id(), id2);
    }

    #[test]
    fn remove_nonexistent_fails() {
        let mut q = Queue::new();
        assert!(q.remove(999).is_err());
    }

    #[test]
    fn clear_removes_all() {
        let mut q = Queue::new();
        q.add_text("first".to_string()).unwrap();
        q.add_text("second".to_string()).unwrap();
        q.clear().unwrap();
        assert!(q.items().is_empty());
    }

    #[test]
    fn reorder_moves_item() {
        let mut q = Queue::new();
        let id1 = q.add_text("first".to_string()).unwrap();
        let id2 = q.add_text("second".to_string()).unwrap();
        let id3 = q.add_text("third".to_string()).unwrap();
        q.reorder(id1, 2).unwrap();
        assert_eq!(q.items()[0].id(), id2);
        assert_eq!(q.items()[1].id(), id3);
        assert_eq!(q.items()[2].id(), id1);
    }

    #[test]
    fn reorder_clamps_index() {
        let mut q = Queue::new();
        let id1 = q.add_text("first".to_string()).unwrap();
        let _id2 = q.add_text("second".to_string()).unwrap();
        q.reorder(id1, 100).unwrap();
        assert_eq!(q.items()[1].id(), id1);
    }

    #[test]
    fn reorder_nonexistent_fails() {
        let mut q = Queue::new();
        q.add_text("first".to_string()).unwrap();
        assert!(q.reorder(999, 0).is_err());
    }
}
