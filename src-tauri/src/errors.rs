// Structured error types for each domain module.
// Each error implements Display for human-readable messages and
// converts to String at the IPC boundary (Tauri requires Result<T, String>).

use std::fmt;

// ── Queue errors ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    Full,
    NotFound(u64),
    WrongItemType,
    NoConfigPath,
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::Full => write!(f, "queue is full"),
            QueueError::NotFound(id) => write!(f, "item with id {id} not found"),
            QueueError::WrongItemType => write!(f, "item is not the expected type"),
            QueueError::NoConfigPath => write!(f, "no config path configured"),
        }
    }
}

impl From<QueueError> for String {
    fn from(e: QueueError) -> String {
        e.to_string()
    }
}

// ── Transcriber errors ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriberError {
    SynthesisFailed(String),
    QueueError(QueueError),
}

impl fmt::Display for TranscriberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranscriberError::SynthesisFailed(msg) => write!(f, "synthesis failed: {msg}"),
            TranscriberError::QueueError(e) => write!(f, "queue error: {e}"),
        }
    }
}

impl From<QueueError> for TranscriberError {
    fn from(e: QueueError) -> Self {
        TranscriberError::QueueError(e)
    }
}

// ── Model errors ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    NotFound(String),
    LoadFailed(String),
    NotInitialized(String),
    InferenceFailed(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::NotFound(key) => write!(f, "model not found: {key}"),
            ModelError::LoadFailed(msg) => write!(f, "failed to load model: {msg}"),
            ModelError::NotInitialized(msg) => write!(f, "model not initialized: {msg}"),
            ModelError::InferenceFailed(msg) => write!(f, "inference failed: {msg}"),
        }
    }
}

// ── Catalog errors ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    VoiceNotFound(String),
    DownloadFailed(String),
    InstallFailed(String),
    UninstallFailed(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::VoiceNotFound(key) => write!(f, "voice '{}' not found in catalog", key),
            CatalogError::DownloadFailed(msg) => write!(f, "download failed: {msg}"),
            CatalogError::InstallFailed(msg) => write!(f, "install failed: {msg}"),
            CatalogError::UninstallFailed(msg) => write!(f, "uninstall failed: {msg}"),
        }
    }
}

// ── Persist errors ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistError {
    IoError(String),
    SerializationError(String),
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistError::IoError(msg) => write!(f, "I/O error: {msg}"),
            PersistError::SerializationError(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

// ── Overlay errors ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayError {
    WindowNotFound,
    WindowCreationFailed(String),
    OperationFailed(String),
}

impl fmt::Display for OverlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverlayError::WindowNotFound => write!(f, "overlay window not found"),
            OverlayError::WindowCreationFailed(msg) => {
                write!(f, "failed to create overlay window: {msg}")
            }
            OverlayError::OperationFailed(msg) => write!(f, "overlay operation failed: {msg}"),
        }
    }
}

// ── Hotkey errors ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    ParseError(String),
    RegistrationFailed(String),
    IoError(String),
}

impl fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HotkeyError::ParseError(msg) => write!(f, "shortcut parse error: {msg}"),
            HotkeyError::RegistrationFailed(msg) => write!(f, "registration failed: {msg}"),
            HotkeyError::IoError(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

// ── Clipboard errors ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    ReadFailed(String),
    NotInitialized,
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::ReadFailed(msg) => write!(f, "clipboard read failed: {msg}"),
            ClipboardError::NotInitialized => write!(f, "clipboard not initialized"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_error_display() {
        assert_eq!(QueueError::Full.to_string(), "queue is full");
        assert_eq!(QueueError::NotFound(42).to_string(), "item with id 42 not found");
        assert_eq!(QueueError::WrongItemType.to_string(), "item is not the expected type");
        assert_eq!(QueueError::NoConfigPath.to_string(), "no config path configured");
    }

    #[test]
    fn queue_error_into_string() {
        let err: String = QueueError::Full.into();
        assert_eq!(err, "queue is full");
    }

    #[test]
    fn model_error_display() {
        assert_eq!(ModelError::NotFound("voice-a".to_string()).to_string(), "model not found: voice-a");
        assert_eq!(
            ModelError::LoadFailed("file missing".to_string()).to_string(),
            "failed to load model: file missing"
        );
    }

    #[test]
    fn catalog_error_display() {
        assert_eq!(
            CatalogError::VoiceNotFound("x".to_string()).to_string(),
            "voice 'x' not found in catalog"
        );
    }

    #[test]
    fn overlay_error_display() {
        assert_eq!(OverlayError::WindowNotFound.to_string(), "overlay window not found");
    }

    #[test]
    fn persist_error_display() {
        assert_eq!(
            PersistError::IoError("perm denied".to_string()).to_string(),
            "I/O error: perm denied"
        );
    }

    #[test]
    fn hotkey_error_display() {
        assert_eq!(
            HotkeyError::ParseError("empty".to_string()).to_string(),
            "shortcut parse error: empty"
        );
    }

    #[test]
    fn clipboard_error_display() {
        assert_eq!(ClipboardError::NotInitialized.to_string(), "clipboard not initialized");
    }

    #[test]
    fn transcriber_error_display() {
        assert_eq!(
            TranscriberError::SynthesisFailed("timeout".to_string()).to_string(),
            "synthesis failed: timeout"
        );
    }
}
