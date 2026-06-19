// Frontend-facing queue operations: add, remove, reorder, clear.
// Enforces max_items limit and emits fine-grained events on mutations.

use super::{Queue, QueueConfig, QueueEvent, QueueItem};

pub trait QueueControllable {
    fn items(&self) -> &[QueueItem];
    fn is_empty(&self) -> bool;
    fn config(&self) -> &QueueConfig;
    fn add_text(&mut self, text: String) -> Result<u64, String>;
    fn remove(&mut self, id: u64) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
    fn reorder(&mut self, id: u64, new_index: usize) -> Result<(), String>;
}

impl QueueControllable for Queue {
    fn items(&self) -> &[QueueItem] {
        &self.items
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn config(&self) -> &QueueConfig {
        &self.config
    }

    fn add_text(&mut self, text: String) -> Result<u64, String> {
        if self.config.max_items > 0 && self.items.len() >= self.config.max_items {
            return Err("queue is full".to_string());
        }
        let id = self.next_id;
        self.next_id += 1;
        self.items
            .push(super::QueueItem::TextMessage {
                id,
                text,
                language: None,
                status: super::TextMessageStatus::Pending,
            });
        self.emit(QueueEvent::ItemAdded);
        Ok(id)
    }

    fn remove(&mut self, id: u64) -> Result<(), String> {
        let before = self.items.len();
        self.items.retain(|item| item.id() != id);
        if self.items.len() == before {
            return Err(format!("item with id {id} not found"));
        }
        self.emit(QueueEvent::ItemRemoved);
        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        self.items.clear();
        self.emit(QueueEvent::ItemCleared);
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
        self.emit(QueueEvent::ItemMoved);
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
        assert!(!q.is_empty());
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
        assert!(!q.is_empty());
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
        assert!(q.is_empty());
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

    #[test]
    fn max_items_enforced() {
        let config = super::QueueConfig {
            max_items: 2,
            ..Default::default()
        };
        let mut q = Queue::new().with_config(config);
        q.add_text("a".to_string()).unwrap();
        q.add_text("b".to_string()).unwrap();
        assert!(q.add_text("c".to_string()).is_err());
        assert!(!q.is_empty());
        assert_eq!(q.items().len(), 2);
    }

    #[test]
    fn max_items_zero_means_unlimited() {
        let config = super::QueueConfig {
            max_items: 0,
            ..Default::default()
        };
        let mut q = Queue::new().with_config(config);
        for i in 0..100 {
            q.add_text(format!("item {i}")).unwrap();
        }
        assert_eq!(q.items().len(), 100);
    }

    #[test]
    fn emit_item_added() {
        use std::sync::{Arc, Mutex};

        let events: Arc<Mutex<Vec<super::super::QueueEvent>>> =
            Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let mut q = Queue::new().with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });
        q.add_text("hello".to_string()).unwrap();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], super::super::QueueEvent::ItemAdded));
    }

    #[test]
    fn emit_item_removed() {
        use std::sync::{Arc, Mutex};

        let events: Arc<Mutex<Vec<super::super::QueueEvent>>> =
            Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let mut q = Queue::new().with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });
        let id = q.add_text("hello".to_string()).unwrap();
        q.remove(id).unwrap();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], super::super::QueueEvent::ItemAdded));
        assert!(matches!(events[1], super::super::QueueEvent::ItemRemoved));
    }

    #[test]
    fn emit_item_cleared() {
        use std::sync::{Arc, Mutex};

        let events: Arc<Mutex<Vec<super::super::QueueEvent>>> =
            Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let mut q = Queue::new().with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });
        q.add_text("hello".to_string()).unwrap();
        q.clear().unwrap();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], super::super::QueueEvent::ItemAdded));
        assert!(matches!(events[1], super::super::QueueEvent::ItemCleared));
    }

    #[test]
    fn emit_item_moved() {
        use std::sync::{Arc, Mutex};

        let events: Arc<Mutex<Vec<super::super::QueueEvent>>> =
            Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let mut q = Queue::new().with_event_handler(move |e| {
            events_clone.lock().unwrap().push(e);
        });
        let id = q.add_text("hello".to_string()).unwrap();
        q.add_text("world".to_string()).unwrap();
        q.reorder(id, 1).unwrap();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], super::super::QueueEvent::ItemAdded));
        assert!(matches!(events[1], super::super::QueueEvent::ItemAdded));
        assert!(matches!(events[2], super::super::QueueEvent::ItemMoved));
    }
}
