// Piper voice catalog: hardcoded voice list, install, uninstall, download with progress.
// Downloads from HuggingFace rhasspy/piper-voices.

use std::path::PathBuf;

use super::{DownloadProgress, InstalledVoice, ModelType, VoiceCatalogOps, VoiceEntry};

const REPO: &str = "rhasspy/piper-voices";

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
            checksum: None,
        }]
    }

    fn download_url(voice_key: &str) -> String {
        // Piper voices on HuggingFace follow: en/en_US/amy/medium/{voice_key}.onnx
        // Parse the voice_key to extract components
        let parts: Vec<&str> = voice_key.split('-').collect();
        if parts.len() >= 3 {
            let lang_country = parts[0]; // en_US
            let name = parts[1]; // amy
            let quality = parts[2]; // medium
            let lang_parts: Vec<&str> = lang_country.split('_').collect();
            let lang = lang_parts.first().unwrap_or(&"en");
            let country = lang_parts.get(1).unwrap_or(&"US");
            format!(
                "https://huggingface.co/{REPO}/resolve/main/{lang}/{lang}_{country}/{name}/{quality}/{voice_key}.onnx"
            )
        } else {
            format!(
                "https://huggingface.co/{REPO}/resolve/main/{voice_key}/{voice_key}.onnx"
            )
        }
    }

    fn config_url(voice_key: &str) -> String {
        let parts: Vec<&str> = voice_key.split('-').collect();
        if parts.len() >= 3 {
            let lang_country = parts[0];
            let name = parts[1];
            let quality = parts[2];
            let lang_parts: Vec<&str> = lang_country.split('_').collect();
            let lang = lang_parts.first().unwrap_or(&"en");
            let country = lang_parts.get(1).unwrap_or(&"US");
            format!(
                "https://huggingface.co/{REPO}/resolve/main/{lang}/{lang}_{country}/{name}/{quality}/{voice_key}.onnx.json"
            )
        } else {
            format!(
                "https://huggingface.co/{REPO}/resolve/main/{voice_key}/{voice_key}.onnx.json"
            )
        }
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

        let model_path = voice_dir.join(format!("{}.onnx", voice_key));
        let config_path = voice_dir.join(format!("{}.onnx.json", voice_key));

        // Download model file
        let url = Self::download_url(voice_key);
        log::info!("Downloading Piper model from {url}");
        super::download::download_file(&url, &model_path, &mut |downloaded, total| {
            on_progress(DownloadProgress::Downloading {
                voice_key: voice_key.to_string(),
                bytes_downloaded: downloaded,
                total_bytes: total,
            });
        })
        .await?;

        // Download config file
        let config_url = Self::config_url(voice_key);
        log::info!("Downloading Piper config from {config_url}");
        super::download::download_file(&config_url, &config_path, &mut |_dl, _total| {}).await?;

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
}
