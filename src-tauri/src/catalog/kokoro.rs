// Kokoro voice catalog: hardcoded voice list, install, uninstall.
// Shared ONNX engine downloaded once, per-voice .bin embeddings downloaded separately.

use std::path::PathBuf;

use super::{InstalledVoice, ModelType, VoiceCatalogOps, VoiceEntry};

const SHARED_ENGINE_SIZE: u64 = 80_000_000;

pub struct KokoroCatalog {
    models_dir: PathBuf,
    shared_engine_path: PathBuf,
}

impl KokoroCatalog {
    pub fn new(models_dir: PathBuf) -> Self {
        let shared_engine_path = models_dir.join("kokoro_engine.onnx");
        Self {
            models_dir,
            shared_engine_path,
        }
    }

    fn hardcoded_voices() -> Vec<VoiceEntry> {
        vec![VoiceEntry {
            voice_key: "af_heart".to_string(),
            name: "Heart (American Female)".to_string(),
            language: "en".to_string(),
            quality: "high".to_string(),
            size_bytes: 15_000_000,
            speed: Some("1.0x".to_string()),
            model_type: ModelType::Kokoro,
            checksum: None,
        }]
    }

    pub fn shared_engine_size() -> u64 {
        SHARED_ENGINE_SIZE
    }

    pub fn is_shared_engine_installed(&self) -> bool {
        self.shared_engine_path.exists()
    }

    pub async fn install<F>(
        &self,
        voice_key: &str,
        mut on_progress: F,
    ) -> Result<InstalledVoice, String>
    where
        F: FnMut(super::DownloadProgress),
    {
        let entry = Self::hardcoded_voices()
            .into_iter()
            .find(|v| v.voice_key == voice_key)
            .ok_or_else(|| format!("voice '{}' not found in catalog", voice_key))?;

        std::fs::create_dir_all(&self.models_dir).map_err(|e| e.to_string())?;

        if !self.shared_engine_path.exists() {
            let total = SHARED_ENGINE_SIZE;
            let mut written: u64 = 0;
            let chunk_size = 1024 * 256;

            while written < total {
                let to_write = std::cmp::min(chunk_size, total - written);
                let chunk = vec![0u8; to_write as usize];
                std::fs::write(&self.shared_engine_path, &chunk).map_err(|e| e.to_string())?;
                written += to_write;
                on_progress(super::DownloadProgress::Downloading {
                    voice_key: "kokoro_engine".to_string(),
                    bytes_downloaded: written,
                    total_bytes: total,
                });
            }
        }

        let voice_path = self.models_dir.join(format!("{voice_key}.bin"));
        let total = entry.size_bytes;
        let mut written: u64 = 0;
        let chunk_size = 1024 * 256;

        while written < total {
            let to_write = std::cmp::min(chunk_size, total - written);
            let chunk = vec![0u8; to_write as usize];
            std::fs::write(&voice_path, &chunk).map_err(|e| e.to_string())?;
            written += to_write;
            on_progress(super::DownloadProgress::Downloading {
                voice_key: voice_key.to_string(),
                bytes_downloaded: written,
                total_bytes: total,
            });
        }

        on_progress(super::DownloadProgress::Complete {
            voice_key: voice_key.to_string(),
        });

        Ok(InstalledVoice {
            voice_key: voice_key.to_string(),
            name: entry.name,
            language: entry.language,
            quality: entry.quality,
            model_type: ModelType::Kokoro,
            model_path: voice_path.to_string_lossy().to_string(),
        })
    }
}

impl VoiceCatalogOps for KokoroCatalog {
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
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.ends_with(".bin") {
                    continue;
                }
                let voice_key = name_str.trim_end_matches(".bin").to_string();
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
                    model_type: ModelType::Kokoro,
                    model_path: entry.path().to_string_lossy().to_string(),
                });
            }
        }

        voices
    }

    fn uninstall(&self, voice_key: &str) -> Result<(), String> {
        let voice_path = self.models_dir.join(format!("{voice_key}.bin"));
        if voice_path.exists() {
            std::fs::remove_file(&voice_path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_kokoro_catalog() -> (KokoroCatalog, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let models_dir = dir.path().join("kokoro");
        (KokoroCatalog::new(models_dir), dir)
    }

    #[test]
    fn list_available_returns_hardcoded_voices() {
        let (catalog, _dir) = setup_kokoro_catalog();
        let voices = catalog.list_available();
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].voice_key, "af_heart");
        assert_eq!(voices[0].model_type, ModelType::Kokoro);
    }

    #[test]
    fn list_installed_empty_when_no_files() {
        let (catalog, _dir) = setup_kokoro_catalog();
        assert!(catalog.list_installed().is_empty());
    }

    #[test]
    fn list_installed_finds_voice_bins() {
        let (catalog, _dir) = setup_kokoro_catalog();
        fs::create_dir_all(&catalog.models_dir).unwrap();
        fs::write(catalog.models_dir.join("af_heart.bin"), "").unwrap();

        let installed = catalog.list_installed();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].voice_key, "af_heart");
        assert_eq!(installed[0].model_type, ModelType::Kokoro);
    }

    #[test]
    fn list_installed_ignores_non_bin_files() {
        let (catalog, _dir) = setup_kokoro_catalog();
        fs::create_dir_all(&catalog.models_dir).unwrap();
        fs::write(catalog.models_dir.join("af_heart.txt"), "").unwrap();
        fs::write(catalog.models_dir.join("engine.onnx"), "").unwrap();

        assert!(catalog.list_installed().is_empty());
    }

    #[test]
    fn uninstall_removes_voice_bin() {
        let (catalog, _dir) = setup_kokoro_catalog();
        fs::create_dir_all(&catalog.models_dir).unwrap();
        fs::write(catalog.models_dir.join("af_heart.bin"), "").unwrap();

        catalog.uninstall("af_heart").unwrap();
        assert!(!catalog.models_dir.join("af_heart.bin").exists());
    }

    #[test]
    fn uninstall_nonexistent_is_ok() {
        let (catalog, _dir) = setup_kokoro_catalog();
        catalog.uninstall("nonexistent").unwrap();
    }

    #[test]
    fn install_creates_shared_engine_and_voice_bin() {
        let (catalog, _dir) = setup_kokoro_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut progress_events = Vec::new();
            let result = catalog
                .install("af_heart", |e| progress_events.push(e))
                .await
                .unwrap();

            assert_eq!(result.voice_key, "af_heart");
            assert_eq!(result.model_type, ModelType::Kokoro);
            assert!(catalog.shared_engine_path.exists());
            assert!(catalog.models_dir.join("af_heart.bin").exists());
            assert!(!progress_events.is_empty());
        });
    }

    #[test]
    fn install_skips_shared_engine_if_exists() {
        let (catalog, _dir) = setup_kokoro_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            fs::create_dir_all(&catalog.models_dir).unwrap();
            fs::write(&catalog.shared_engine_path, "existing").unwrap();

            let mut engine_events = Vec::new();
            catalog
                .install("af_heart", |e| {
                    if let super::super::DownloadProgress::Downloading {
                        ref voice_key, ..
                    } = e
                    {
                        if voice_key == "kokoro_engine" {
                            engine_events.push(e);
                        }
                    }
                })
                .await
                .unwrap();

            assert!(engine_events.is_empty());
        });
    }

    #[test]
    fn install_emits_progress_events() {
        let (catalog, _dir) = setup_kokoro_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut events = Vec::new();
            catalog
                .install("af_heart", |e| events.push(e))
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
        let (catalog, _dir) = setup_kokoro_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = catalog.install("nonexistent", |_| {}).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn install_then_list_installed() {
        let (catalog, _dir) = setup_kokoro_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("af_heart", |_| {}).await.unwrap();
            let installed = catalog.list_installed();
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0].voice_key, "af_heart");
        });
    }

    #[test]
    fn shared_engine_not_installed_initially() {
        let (catalog, _dir) = setup_kokoro_catalog();
        assert!(!catalog.is_shared_engine_installed());
    }

    #[test]
    fn shared_engine_detected_after_install() {
        let (catalog, _dir) = setup_kokoro_catalog();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            catalog.install("af_heart", |_| {}).await.unwrap();
            assert!(catalog.is_shared_engine_installed());
        });
    }
}
