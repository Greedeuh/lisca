// Kokoro voice catalog: install, uninstall.
// Shared ONNX engine downloaded once from HuggingFace, per-voice .bin embeddings downloaded separately.

use std::path::PathBuf;

use super::{InstalledVoice, ModelType, VoiceCatalogOps, VoiceEntry};

pub(super)  struct KokoroCatalog {
    models_dir: PathBuf,
    shared_engine_path: PathBuf,
    entries: Vec<VoiceEntry>,
}

impl KokoroCatalog {
    pub(super)  fn new(models_dir: PathBuf, entries: Vec<VoiceEntry>) -> Self {
        let shared_engine_path = models_dir.join("kokoro_engine.onnx");
        Self {
            models_dir,
            shared_engine_path,
            entries,
        }
    }

    fn find_entry(&self, voice_key: &str) -> Option<&VoiceEntry> {
        self.entries.iter().find(|v| v.voice_key == voice_key)
    }

    fn download_url(path: &str) -> String {
        format!(
            "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/{path}"
        )
    }

     fn is_shared_engine_installed(&self) -> bool {
        self.shared_engine_path.exists()
    }

    pub(super)   async fn install<F>(
        &self,
        voice_key: &str,
        mut on_progress: F,
    ) -> Result<InstalledVoice, String>
    where
        F: FnMut(super::DownloadProgress),
    {
        let entry = self
            .find_entry(voice_key)
            .ok_or_else(|| format!("voice '{}' not found in catalog", voice_key))?;

        std::fs::create_dir_all(&self.models_dir).map_err(|e| e.to_string())?;

        // Download shared engine if not present
        if !self.shared_engine_path.exists() {
            let url = Self::download_url("onnx/model_q4.onnx");
            log::info!("Downloading Kokoro shared engine from {url}");
            super::download::download_file(&url, &self.shared_engine_path, &mut |downloaded, total| {
                on_progress(super::DownloadProgress::Downloading {
                    voice_key: "kokoro_engine".to_string(),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                });
            })
            .await?;
        }

        // Download per-voice embedding
        let voice_path = self.models_dir.join(format!("{voice_key}.bin"));
        let fallback_url = Self::download_url(&format!("voices/{voice_key}.bin"));
        let url = entry
            .download_url
            .as_deref()
            .unwrap_or(&fallback_url);
        log::info!("Downloading Kokoro voice {voice_key} from {url}");
        super::download::download_file(url, &voice_path, &mut |downloaded, total| {
            on_progress(super::DownloadProgress::Downloading {
                voice_key: voice_key.to_string(),
                bytes_downloaded: downloaded,
                total_bytes: total,
            });
        })
        .await?;

        on_progress(super::DownloadProgress::Complete {
            voice_key: voice_key.to_string(),
        });

        Ok(InstalledVoice {
            voice_key: voice_key.to_string(),
            name: entry.name.clone(),
            language: entry.language.clone(),
            quality: entry.quality.clone(),
            model_type: ModelType::Kokoro,
            model_path: voice_path.to_string_lossy().to_string(),
        })
    }
}

impl VoiceCatalogOps for KokoroCatalog {
    fn list_available(&self) -> Vec<VoiceEntry> {
        self.entries.clone()
    }

    fn list_installed(&self) -> Vec<InstalledVoice> {
        let mut voices = Vec::new();

        if !self.models_dir.exists() {
            return voices;
        }

        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.ends_with(".bin") {
                    continue;
                }
                let voice_key = name_str.trim_end_matches(".bin").to_string();
                let meta = self.find_entry(&voice_key);
                voices.push(InstalledVoice {
                    voice_key: voice_key.clone(),
                    name: meta.map(|e| e.name.clone()).unwrap_or_else(|| voice_key.clone()),
                    language: meta.map(|e| e.language.clone()).unwrap_or_else(|| "unknown".into()),
                    quality: meta.map(|e| e.quality.clone()).unwrap_or_else(|| "unknown".into()),
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

    fn test_entries() -> Vec<VoiceEntry> {
        vec![VoiceEntry {
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
        }]
    }

    fn setup_kokoro_catalog() -> (KokoroCatalog, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let models_dir = dir.path().join("kokoro");
        (KokoroCatalog::new(models_dir, test_entries()), dir)
    }

    #[test]
    fn list_available_returns_entries() {
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
    fn shared_engine_not_installed_initially() {
        let (catalog, _dir) = setup_kokoro_catalog();
        assert!(!catalog.is_shared_engine_installed());
    }
}
