// Voice catalog: browse, install, uninstall Piper and Kokoro voices.
// Unified interface over both backends with download progress reporting.

mod piper;
mod kokoro;

pub use piper::PiperCatalog;
pub use kokoro::KokoroCatalog;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelType {
    #[serde(rename = "piper")]
    Piper,
    #[serde(rename = "kokoro")]
    Kokoro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceEntry {
    pub voice_key: String,
    pub name: String,
    pub language: String,
    pub quality: String,
    pub size_bytes: u64,
    pub speed: Option<String>,
    pub model_type: ModelType,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum DownloadProgress {
    #[serde(rename = "downloading")]
    Downloading {
        voice_key: String,
        bytes_downloaded: u64,
        total_bytes: u64,
    },
    #[serde(rename = "complete")]
    Complete { voice_key: String },
    #[serde(rename = "error")]
    Error { voice_key: String, reason: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledVoice {
    pub voice_key: String,
    pub name: String,
    pub language: String,
    pub quality: String,
    pub model_type: ModelType,
    pub model_path: String,
}

pub trait VoiceCatalogOps {
    fn list_available(&self) -> Vec<VoiceEntry>;
    fn list_installed(&self) -> Vec<InstalledVoice>;
    fn uninstall(&self, voice_key: &str) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_entry_serialization_roundtrip() {
        let entry = VoiceEntry {
            voice_key: "test-voice".to_string(),
            name: "Test Voice".to_string(),
            language: "en".to_string(),
            quality: "medium".to_string(),
            size_bytes: 50_000_000,
            speed: Some("1.0x".to_string()),
            model_type: ModelType::Piper,
            checksum: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: VoiceEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }

    #[test]
    fn download_progress_serialization() {
        let progress = DownloadProgress::Downloading {
            voice_key: "test".to_string(),
            bytes_downloaded: 1024,
            total_bytes: 4096,
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("downloading"));
        assert!(json.contains("1024"));

        let complete = DownloadProgress::Complete {
            voice_key: "test".to_string(),
        };
        let json = serde_json::to_string(&complete).unwrap();
        assert!(json.contains("complete"));
    }
}
