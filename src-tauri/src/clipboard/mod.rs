// Clipboard reading abstraction.
// Reads text from the system clipboard for the hotkey → queue trigger chain.

pub trait ClipboardReader: Send {
    fn read_text(&self) -> Result<String, String>;
}

pub struct SystemClipboard;

impl ClipboardReader for SystemClipboard {
    fn read_text(&self) -> Result<String, String> {
        Err("clipboard not initialized (requires Tauri AppHandle)".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockClipboard {
        content: Arc<Mutex<String>>,
    }

    impl MockClipboard {
        fn new(content: &str) -> Self {
            Self {
                content: Arc::new(Mutex::new(content.to_string())),
            }
        }
    }

    impl ClipboardReader for MockClipboard {
        fn read_text(&self) -> Result<String, String> {
            Ok(self.content.lock().unwrap().clone())
        }
    }

    #[test]
    fn mock_clipboard_reads_content() {
        let clipboard = MockClipboard::new("hello world");
        assert_eq!(clipboard.read_text().unwrap(), "hello world");
    }

    #[test]
    fn mock_clipboard_reads_empty() {
        let clipboard = MockClipboard::new("");
        assert_eq!(clipboard.read_text().unwrap(), "");
    }

    #[test]
    fn system_clipboard_returns_error() {
        let clipboard = SystemClipboard;
        assert!(clipboard.read_text().is_err());
    }
}
