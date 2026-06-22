// Voice catalog: browse, install, uninstall Piper and Kokoro voices.
// Unified interface over both backends with download progress reporting.

mod piper;
mod kokoro;
 mod download;

 use piper::PiperCatalog;
 use kokoro::KokoroCatalog;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
 struct CatalogFile {
     voices: Vec<VoiceEntry>,
}

 fn load_catalog(resource_dir: &Path) -> Result<Vec<VoiceEntry>, String> {
    let catalog_path = resource_dir.join("catalog.json");
    let data = std::fs::read_to_string(&catalog_path)
        .map_err(|e| format!("failed to read catalog.json: {e}"))?;
    let catalog: CatalogFile =
        serde_json::from_str(&data).map_err(|e| format!("failed to parse catalog.json: {e}"))?;
    Ok(catalog.voices)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super)  enum ModelType {
    #[serde(rename = "piper")]
    Piper,
    #[serde(rename = "kokoro")]
    Kokoro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(super)  struct VoiceEntry {
    pub(super)  voice_key: String,
    pub(super)  name: String,
    pub(super)  language: String,
    pub(super)  quality: String,
    pub(super)  size_bytes: u64,
    pub(super)  speed: Option<String>,
    pub(super)  model_type: ModelType,
     checksum: Option<String>,
     download_url: Option<String>,
     config_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub(super)  enum DownloadProgress {
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
pub(super)  struct InstalledVoice {
    pub(super)  voice_key: String,
    pub(super)  name: String,
    pub(super)  language: String,
    pub(super)  quality: String,
    pub(super)  model_type: ModelType,
    pub(super)  model_path: String,
}

pub(super)  trait VoiceCatalogOps {
    fn list_available(&self) -> Vec<VoiceEntry>;
    fn list_installed(&self) -> Vec<InstalledVoice>;
    fn uninstall(&self, voice_key: &str) -> Result<(), String>;
}

pub(super)  struct VoiceCatalog {
    piper: PiperCatalog,
    kokoro: KokoroCatalog,
    entries: Vec<VoiceEntry>,
}

impl VoiceCatalog {
    pub(super)  fn new(piper_models_dir: PathBuf, kokoro_models_dir: PathBuf, resource_dir: &Path) -> Self {
        let all_entries = load_catalog(resource_dir).unwrap_or_else(|e| {
            log::error!("Failed to load catalog: {e}");
            Vec::new()
        });
        let piper_entries: Vec<VoiceEntry> = all_entries
            .iter()
            .filter(|e| e.model_type == ModelType::Piper)
            .cloned()
            .collect();
        let kokoro_entries: Vec<VoiceEntry> = all_entries
            .iter()
            .filter(|e| e.model_type == ModelType::Kokoro)
            .cloned()
            .collect();
        Self {
            piper: PiperCatalog::new(piper_models_dir, piper_entries),
            kokoro: KokoroCatalog::new(kokoro_models_dir, kokoro_entries),
            entries: all_entries,
        }
    }

    pub(super)  async fn install<F>(
        &self,
        voice_key: &str,
        on_progress: F,
    ) -> Result<InstalledVoice, String>
    where
        F: FnMut(DownloadProgress),
    {
        log::info!("Installing voice: {voice_key}");
        let entry = self.entries.iter().find(|v| v.voice_key == voice_key);
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
        self.entries.clone()
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

        // Create a test catalog.json in the temp dir
        let catalog = CatalogFile {
            voices: vec![
                VoiceEntry {
                    voice_key: "en_US-amy-medium".to_string(),
                    name: "Amy (English, US)".to_string(),
                    language: "en".to_string(),
                    quality: "medium".to_string(),
                    size_bytes: 52_000_000,
                    speed: Some("1.0x".to_string()),
                    model_type: ModelType::Piper,
                    checksum: None,
                    download_url: Some("https://example.com/en_US-amy-medium.onnx".to_string()),
                    config_url: Some("https://example.com/en_US-amy-medium.onnx.json".to_string()),
                },
                VoiceEntry {
                    voice_key: "af_heart".to_string(),
                    name: "Heart (American Female)".to_string(),
                    language: "en".to_string(),
                    quality: "high".to_string(),
                    size_bytes: 15_000_000,
                    speed: Some("1.0x".to_string()),
                    model_type: ModelType::Kokoro,
                    checksum: None,
                    download_url: Some("https://example.com/af_heart.bin".to_string()),
                    config_url: None,
                },
            ],
        };
        let json = serde_json::to_string(&catalog).unwrap();
        std::fs::write(dir.path().join("catalog.json"), json).unwrap();

        (
            VoiceCatalog::new(piper_dir, kokoro_dir, dir.path()),
            dir,
        )
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
            download_url: Some("https://example.com/test.onnx".to_string()),
            config_url: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: VoiceEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }

    #[test]
    fn catalog_file_deserialization() {
        let data = r#"{
            "voices": [
                {
                    "voice_key": "test",
                    "name": "Test",
                    "language": "en",
                    "quality": "high",
                    "size_bytes": 1000,
                    "speed": "1.0x",
                    "model_type": "piper",
                    "checksum": null,
                    "download_url": "https://example.com/test.onnx",
                    "config_url": null
                }
            ]
        }"#;
        let catalog: CatalogFile = serde_json::from_str(data).unwrap();
        assert_eq!(catalog.voices.len(), 1);
        assert_eq!(catalog.voices[0].voice_key, "test");
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
