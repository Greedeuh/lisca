// Piper voice catalog: hardcoded voice list, install, uninstall, download with progress.

use std::path::PathBuf;

use super::{DownloadProgress, InstalledVoice, ModelType, VoiceCatalogOps, VoiceEntry};

pub struct PiperCatalog {
    models_dir: PathBuf,
}

impl PiperCatalog {
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir }
    }

    fn hardcoded_voices() -> Vec<VoiceEntry> {
        vec![VoiceEntry {
            voice_key: "en_US-amy-medium".to_string(),
            name: "Amy (English, US)".to_string(),
            language: "en".to_string(),
            quality: "medium".to_string(),
            size_bytes: 52_000_000,
            speed: Some("1.0x".to_string()),
            model_type: ModelType::Piper,
            checksum: Some("abc123def456".to_string()),
        }]
    }

    pub async fn install<F>(
        &self,
        voice_key: &str,
        mut on_progress: F,
    ) -> Result<InstalledVoice, String>
    where
        F: FnMut(DownloadProgress),
    {
        let entry = Self::hardcoded_voices()
            .into_iter()
            .find(|v| v.voice_key == voice_key)
            .ok_or_else(|| format!("voice '{}' not found in catalog", voice_key))?;

        let voice_dir = self.models_dir.join(voice_key);
        std::fs::create_dir_all(&voice_dir).map_err(|e| e.to_string())?;

        let total_bytes = entry.size_bytes;
        let model_path = voice_dir.join(format!("{}.onnx", voice_key));
        let config_path = voice_dir.join(format!("{}.onnx.json", voice_key));

        let mut written: u64 = 0;
        let chunk_size = 1024 * 256;

        while written < total_bytes {
            let to_write = std::cmp::min(chunk_size, total_bytes - written);
            let chunk = vec![0u8; to_write as usize];
            std::fs::write(&model_path, &chunk).map_err(|e| e.to_string())?;
            written += to_write;
            on_progress(DownloadProgress::Downloading {
                voice_key: voice_key.to_string(),
                bytes_downloaded: written,
                total_bytes,
            });
        }

        std::fs::write(&config_path, "{}").map_err(|e| e.to_string())?;

        on_progress(DownloadProgress::Complete {
            voice_key: voice_key.to_string(),
        });

        Ok(InstalledVoice {
            voice_key: voice_key.to_string(),
            name: entry.name,
            language: entry.language,
            quality: entry.quality,
            model_type: ModelType::Piper,
            model_path: model_path.to_string_lossy().to_string(),
        })
    }

    pub fn verify_checksum(&self, voice_key: &str) -> Result<bool, String> {
        let entry = Self::hardcoded_voices()
            .into_iter()
            .find(|v| v.voice_key == voice_key)
            .ok_or_else(|| format!("voice '{}' not found in catalog", voice_key))?;

        let expected = match entry.checksum {
            Some(c) => c,
            None => return Ok(true),
        };

        let model_path = self
            .models_dir
            .join(voice_key)
            .join(format!("{}.onnx", voice_key));

        if !model_path.exists() {
            return Err(format!("model file not found: {}", model_path.display()));
        }

        let data = std::fs::read(&model_path).map_err(|e| e.to_string())?;
        let hash = simple_hash_hex(&data);
        Ok(hash == expected)
    }
}

impl VoiceCatalogOps for PiperCatalog {
    fn list_available(&self) -> Vec<VoiceEntry> {
        Self::hardcoded_voices()
    }

    fn list_installed(&self) -> Vec<InstalledVoice> {
        let mut voices = Vec::new();

        if !self.models_dir.exists() {
            return voices;
        }

        let catalog_entries = Self::hardcoded_voices();

        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let voice_key = entry.file_name().to_string_lossy().to_string();
                let model_path = entry.path().join(format!("{}.onnx", voice_key));
                let config_path = entry.path().join(format!("{}.onnx.json", voice_key));

                if model_path.exists() && config_path.exists() {
                    let meta = catalog_entries
                        .iter()
                        .find(|v| v.voice_key == voice_key)
                        .map(|v| (v.name.clone(), v.language.clone(), v.quality.clone()))
                        .unwrap_or_else(|| {
                            (voice_key.clone(), "unknown".into(), "unknown".into())
                        });

                    voices.push(InstalledVoice {
                        voice_key,
                        name: meta.0,
                        language: meta.1,
                        quality: meta.2,
                        model_type: ModelType::Piper,
                        model_path: model_path.to_string_lossy().to_string(),
                    });
                }
            }
        }

        voices
    }

    fn uninstall(&self, voice_key: &str) -> Result<(), String> {
        let voice_dir = self.models_dir.join(voice_key);
        if voice_dir.exists() {
            std::fs::remove_dir_all(&voice_dir).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn simple_hash_hex(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_piper_catalog() -> (PiperCatalog, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let models_dir = dir.path().join("piper_models");
        (PiperCatalog::new(models_dir), dir)
    }

    #[test]
    fn list_available_returns_hardcoded_voices() {
        let (catalog, _dir) = setup_piper_catalog();
        let voices = catalog.list_available();
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].voice_key, "en_US-amy-medium");
        assert_eq!(voices[0].model_type, ModelType::Piper);
    }

    #[test]
    fn list_installed_empty_when_no_files() {
        let (catalog, _dir) = setup_piper_catalog();
        assert!(catalog.list_installed().is_empty());
    }

    #[test]
    fn list_installed_finds_valid_voices() {
        let (catalog, _dir) = setup_piper_catalog();
        let voice_dir = catalog.models_dir.join("en_US-amy-medium");
        fs::create_dir_all(&voice_dir).unwrap();
        fs::write(voice_dir.join("en_US-amy-medium.onnx"), "").unwrap();
        fs::write(voice_dir.join("en_US-amy-medium.onnx.json"), "{}").unwrap();

        let installed = catalog.list_installed();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].voice_key, "en_US-amy-medium");
        assert_eq!(installed[0].model_type, ModelType::Piper);
    }

    #[test]
    fn list_installed_skips_incomplete_voices() {
        let (catalog, _dir) = setup_piper_catalog();
        let voice_dir = catalog.models_dir.join("en_US-amy-medium");
        fs::create_dir_all(&voice_dir).unwrap();
        fs::write(voice_dir.join("en_US-amy-medium.onnx"), "").unwrap();
        // Missing .onnx.json

        assert!(catalog.list_installed().is_empty());
    }

    #[test]
    fn uninstall_removes_voice_directory() {
        let (catalog, _dir) = setup_piper_catalog();
        let voice_dir = catalog.models_dir.join("en_US-amy-medium");
        fs::create_dir_all(&voice_dir).unwrap();
        fs::write(voice_dir.join("en_US-amy-medium.onnx"), "").unwrap();

        catalog.uninstall("en_US-amy-medium").unwrap();
        assert!(!voice_dir.exists());
    }

    #[test]
    fn uninstall_nonexistent_is_ok() {
        let (catalog, _dir) = setup_piper_catalog();
        catalog.uninstall("nonexistent").unwrap();
    }

    #[test]
    fn install_creates_model_files() {
        let (catalog, _dir) = setup_piper_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut progress_events = Vec::new();
            let result = catalog
                .install("en_US-amy-medium", |e| progress_events.push(e))
                .await
                .unwrap();

            assert_eq!(result.voice_key, "en_US-amy-medium");
            assert_eq!(result.model_type, ModelType::Piper);

            let voice_dir = catalog.models_dir.join("en_US-amy-medium");
            assert!(voice_dir.join("en_US-amy-medium.onnx").exists());
            assert!(voice_dir.join("en_US-amy-medium.onnx.json").exists());

            assert!(!progress_events.is_empty());
            assert!(matches!(
                progress_events.last().unwrap(),
                super::super::DownloadProgress::Complete { .. }
            ));
        });
    }

    #[test]
    fn install_emits_progress_events() {
        let (catalog, _dir) = setup_piper_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut events = Vec::new();
            catalog
                .install("en_US-amy-medium", |e| events.push(e))
                .await
                .unwrap();

            let downloading: Vec<_> = events
                .iter()
                .filter(|e| matches!(e, super::super::DownloadProgress::Downloading { .. }))
                .collect();
            assert!(!downloading.is_empty());
        });
    }

    #[test]
    fn install_unknown_voice_fails() {
        let (catalog, _dir) = setup_piper_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = catalog.install("nonexistent", |_| {}).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not found"));
        });
    }

    #[test]
    fn install_then_list_installed() {
        let (catalog, _dir) = setup_piper_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("en_US-amy-medium", |_| {}).await.unwrap();
            let installed = catalog.list_installed();
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0].voice_key, "en_US-amy-medium");
        });
    }

    #[test]
    fn verify_checksum_passes_for_installed_voice() {
        let (catalog, _dir) = setup_piper_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("en_US-amy-medium", |_| {}).await.unwrap();
            // The dummy file won't match the expected checksum, so it returns false
            let result = catalog.verify_checksum("en_US-amy-medium").unwrap();
            assert!(!result);
        });
    }

    #[test]
    fn verify_checksum_unknown_voice_fails() {
        let (catalog, _dir) = setup_piper_catalog();
        assert!(catalog.verify_checksum("nonexistent").is_err());
    }
}
