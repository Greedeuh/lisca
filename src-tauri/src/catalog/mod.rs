// Voice catalog: browse, install, uninstall Piper and Kokoro voices.
// Unified interface over both backends with download progress reporting.

mod piper;
mod kokoro;
pub(crate) mod download;

pub use piper::PiperCatalog;
pub use kokoro::KokoroCatalog;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

pub struct VoiceCatalog {
    piper: PiperCatalog,
    kokoro: KokoroCatalog,
}

impl VoiceCatalog {
    pub fn new(piper_models_dir: PathBuf, kokoro_models_dir: PathBuf) -> Self {
        Self {
            piper: PiperCatalog::new(piper_models_dir),
            kokoro: KokoroCatalog::new(kokoro_models_dir),
        }
    }

    pub async fn install<F>(
        &self,
        voice_key: &str,
        on_progress: F,
    ) -> Result<InstalledVoice, String>
    where
        F: FnMut(DownloadProgress),
    {
        log::info!("Installing voice: {voice_key}");
        let all = self.list_available();
        let entry = all.iter().find(|v| v.voice_key == voice_key);
        let result = match entry {
            Some(e) if e.model_type == ModelType::Piper => {
                self.piper.install(voice_key, on_progress).await
            }
            Some(e) if e.model_type == ModelType::Kokoro => {
                self.kokoro.install(voice_key, on_progress).await
            }
            _ => Err(format!("voice '{}' not found in catalog", voice_key)),
        };
        match &result {
            Ok(_) => log::info!("Voice {voice_key} installed successfully"),
            Err(e) => log::error!("Failed to install voice {voice_key}: {e}"),
        }
        result
    }
}

impl VoiceCatalogOps for VoiceCatalog {
    fn list_available(&self) -> Vec<VoiceEntry> {
        let mut voices = self.piper.list_available();
        voices.extend(self.kokoro.list_available());
        voices
    }

    fn list_installed(&self) -> Vec<InstalledVoice> {
        let mut voices = self.piper.list_installed();
        voices.extend(self.kokoro.list_installed());
        voices
    }

    fn uninstall(&self, voice_key: &str) -> Result<(), String> {
        log::info!("Uninstalling voice: {voice_key}");
        let installed = self.list_installed();
        let voice = installed.iter().find(|v| v.voice_key == voice_key);
        let result = match voice {
            Some(v) if v.model_type == ModelType::Piper => self.piper.uninstall(voice_key),
            Some(v) if v.model_type == ModelType::Kokoro => self.kokoro.uninstall(voice_key),
            _ => Ok(()),
        };
        if let Err(e) = &result {
            log::error!("Failed to uninstall voice {voice_key}: {e}");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_voice_catalog() -> (VoiceCatalog, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let piper_dir = dir.path().join("piper_models");
        let kokoro_dir = dir.path().join("kokoro");
        (VoiceCatalog::new(piper_dir, kokoro_dir), dir)
    }

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

    #[test]
    fn unified_catalog_lists_all_voices() {
        let (catalog, _dir) = setup_voice_catalog();
        let voices = catalog.list_available();
        assert_eq!(voices.len(), 2);
        let keys: Vec<&str> = voices.iter().map(|v| v.voice_key.as_str()).collect();
        assert!(keys.contains(&"en_US-amy-medium"));
        assert!(keys.contains(&"af_heart"));
    }

    #[test]
    fn unified_catalog_lists_installed() {
        let (catalog, _dir) = setup_voice_catalog();
        let installed = catalog.list_installed();
        assert!(installed.is_empty());
    }

    #[test]
    fn unified_catalog_install_then_list() {
        let (catalog, _dir) = setup_voice_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("en_US-amy-medium", |_| {}).await.unwrap();
            let installed = catalog.list_installed();
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0].voice_key, "en_US-amy-medium");
        });
    }

    #[test]
    fn unified_catalog_install_kokoro() {
        let (catalog, _dir) = setup_voice_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("af_heart", |_| {}).await.unwrap();
            let installed = catalog.list_installed();
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0].voice_key, "af_heart");
        });
    }

    #[test]
    fn unified_catalog_uninstall() {
        let (catalog, _dir) = setup_voice_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("en_US-amy-medium", |_| {}).await.unwrap();
            catalog.uninstall("en_US-amy-medium").unwrap();
            assert!(catalog.list_installed().is_empty());
        });
    }

    #[test]
    fn unified_catalog_install_unknown_fails() {
        let (catalog, _dir) = setup_voice_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = catalog.install("nonexistent", |_| {}).await;
            assert!(result.is_err());
        });
    }
}
