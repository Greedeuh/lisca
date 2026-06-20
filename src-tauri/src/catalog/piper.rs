// Piper voice catalog: install, uninstall, download with progress.
// Downloads from HuggingFace rhasspy/piper-voices.

use std::path::PathBuf;

use super::{DownloadProgress, InstalledVoice, ModelType, VoiceCatalogOps, VoiceEntry};

pub struct PiperCatalog {
    models_dir: PathBuf,
    entries: Vec<VoiceEntry>,
}

impl PiperCatalog {
    pub fn new(models_dir: PathBuf, entries: Vec<VoiceEntry>) -> Self {
        Self { models_dir, entries }
    }

    fn find_entry(&self, voice_key: &str) -> Option<&VoiceEntry> {
        self.entries.iter().find(|v| v.voice_key == voice_key)
    }

    pub async fn install<F>(
        &self,
        voice_key: &str,
        mut on_progress: F,
    ) -> Result<InstalledVoice, String>
    where
        F: FnMut(DownloadProgress),
    {
        let entry = self
            .find_entry(voice_key)
            .ok_or_else(|| format!("voice '{}' not found in catalog", voice_key))?;

        let voice_dir = self.models_dir.join(voice_key);
        std::fs::create_dir_all(&voice_dir).map_err(|e| e.to_string())?;

        let model_path = voice_dir.join(format!("{}.onnx", voice_key));
        let config_path = voice_dir.join(format!("{}.onnx.json", voice_key));

        // Download model file
        let url = entry
            .download_url
            .as_deref()
            .ok_or_else(|| format!("no download_url for voice '{voice_key}'"))?;
        log::info!("Downloading Piper model from {url}");
        super::download::download_file(url, &model_path, &mut |downloaded, total| {
            on_progress(DownloadProgress::Downloading {
                voice_key: voice_key.to_string(),
                bytes_downloaded: downloaded,
                total_bytes: total,
            });
        })
        .await?;

        // Download config file
        if let Some(config_url) = &entry.config_url {
            log::info!("Downloading Piper config from {config_url}");
            super::download::download_file(config_url, &config_path, &mut |_dl, _total| {})
                .await?;
        }

        on_progress(DownloadProgress::Complete {
            voice_key: voice_key.to_string(),
        });

        Ok(InstalledVoice {
            voice_key: voice_key.to_string(),
            name: entry.name.clone(),
            language: entry.language.clone(),
            quality: entry.quality.clone(),
            model_type: ModelType::Piper,
            model_path: model_path.to_string_lossy().to_string(),
        })
    }

    pub fn verify_checksum(&self, voice_key: &str, expected: &str) -> Result<bool, String> {
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
        self.entries.clone()
    }

    fn list_installed(&self) -> Vec<InstalledVoice> {
        let mut voices = Vec::new();

        if !self.models_dir.exists() {
            return voices;
        }

        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let voice_key = entry.file_name().to_string_lossy().to_string();
                let model_path = entry.path().join(format!("{}.onnx", voice_key));
                let config_path = entry.path().join(format!("{}.onnx.json", voice_key));

                if model_path.exists() && config_path.exists() {
                    let meta = self.find_entry(&voice_key);
                    voices.push(InstalledVoice {
                        voice_key: voice_key.clone(),
                        name: meta.map(|e| e.name.clone()).unwrap_or_else(|| voice_key.clone()),
                        language: meta.map(|e| e.language.clone()).unwrap_or_else(|| "unknown".into()),
                        quality: meta.map(|e| e.quality.clone()).unwrap_or_else(|| "unknown".into()),
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
