use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Emitter;
use futures_util::StreamExt;

const VOICES_JSON_URL: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/voices.json";
const HF_BASE_URL: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCatalog {
    #[serde(flatten)]
    pub voices: HashMap<String, VoiceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceEntry {
    pub key: String,
    pub name: String,
    pub language: VoiceLanguage,
    pub quality: String,
    pub num_speakers: usize,
    #[serde(default)]
    pub speaker_id_map: HashMap<String, usize>,
    pub files: HashMap<String, VoiceFile>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoiceLanguage {
    pub code: String,
    pub family: String,
    pub region: String,
    pub name_native: String,
    pub name_english: String,
    pub country_english: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceFile {
    pub size_bytes: u64,
    pub md5_digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledModel {
    pub voice_key: String,
    pub model_path: String,
    pub config_path: String,
    pub language: VoiceLanguage,
    pub quality: String,
    pub name: String,
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
}

pub struct PiperModelManager {
    models_dir: PathBuf,
    cache_path: PathBuf,
    catalog: Option<VoiceCatalog>,
    http_client: reqwest::Client,
}

impl PiperModelManager {
    pub fn new(app_data_dir: &Path) -> Self {
        let models_dir = app_data_dir.join("lisca").join("piper_models");
        let cache_path = app_data_dir.join("lisca").join("piper_voices_cache.json");
        Self {
            models_dir,
            cache_path,
            catalog: None,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn load_cached_voices(&mut self) -> Option<&VoiceCatalog> {
        if self.catalog.is_some() {
            return self.catalog.as_ref();
        }
        if !self.cache_path.exists() {
            return None;
        }
        let data = std::fs::read_to_string(&self.cache_path).ok()?;
        let catalog: VoiceCatalog = serde_json::from_str(&data).ok()?;
        self.catalog = Some(catalog);
        self.catalog.as_ref()
    }

    pub async fn fetch_voices(&mut self) -> Result<&VoiceCatalog, String> {
        let response = self
            .http_client
            .get(VOICES_JSON_URL)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch voices: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch voices: HTTP {}", response.status()));
        }

        let catalog: VoiceCatalog = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse voices JSON: {}", e))?;

        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?;
        tokio::fs::write(&self.cache_path, data)
            .await
            .map_err(|e| e.to_string())?;

        self.catalog = Some(catalog);
        Ok(self.catalog.as_ref().unwrap())
    }

    pub async fn download_voice(
        &self,
        voice_key: &str,
        app: &AppHandle,
    ) -> Result<InstalledModel, String> {
        let catalog = self.catalog.as_ref().ok_or("Voice catalog not loaded")?;
        let voice = catalog.voices.get(voice_key).ok_or("Voice not found")?;

        let voice_dir = self.models_dir.join(voice_key);
        std::fs::create_dir_all(&voice_dir).map_err(|e| e.to_string())?;

        let mut onnx_path = None;
        let mut json_path = None;

        for (file_key, file_info) in &voice.files {
            if file_key.ends_with(".onnx") && !file_key.ends_with(".onnx.json") {
                onnx_path = Some((file_key.clone(), file_info.size_bytes));
            } else if file_key.ends_with(".onnx.json") {
                json_path = Some(file_key.clone());
            }
        }

        let (onnx_file_key, _onnx_size) = onnx_path.ok_or("No .onnx file found in voice files")?;
        let json_file_key = json_path.ok_or("No .onnx.json file found in voice files")?;

        let onnx_url = format!("{}/{}", HF_BASE_URL, onnx_file_key);
        let onnx_dest = voice_dir.join(format!("{}.onnx", voice_key));
        let onnx_tmp = voice_dir.join(format!("{}.onnx.tmp", voice_key));

        self.download_file(&onnx_url, &onnx_tmp, &onnx_dest, app, voice_key, true)
            .await?;

        let json_url = format!("{}/{}", HF_BASE_URL, json_file_key);
        let json_dest = voice_dir.join(format!("{}.onnx.json", voice_key));
        let json_tmp = voice_dir.join(format!("{}.onnx.json.tmp", voice_key));

        self.download_file(&json_url, &json_tmp, &json_dest, app, voice_key, false)
            .await?;

        let _ = app.emit(
            "piper-download-progress",
            DownloadProgress::Complete {
                voice_key: voice_key.to_string(),
            },
        );

        Ok(InstalledModel {
            voice_key: voice_key.to_string(),
            model_path: onnx_dest.to_string_lossy().to_string(),
            config_path: json_dest.to_string_lossy().to_string(),
            language: voice.language.clone(),
            quality: voice.quality.clone(),
            name: voice.name.clone(),
        })
    }

    async fn download_file(
        &self,
        url: &str,
        tmp_path: &Path,
        dest_path: &Path,
        app: &AppHandle,
        voice_key: &str,
        is_model: bool,
    ) -> Result<(), String> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to download: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download failed: HTTP {}", response.status()));
        }

        let total_bytes = response.content_length().unwrap_or(0);
        let mut bytes_downloaded: u64 = 0;

        let mut file = tokio::fs::File::create(tmp_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;

            bytes_downloaded += chunk.len() as u64;

            if is_model && (bytes_downloaded % (256 * 1024) < chunk.len() as u64) {
                let _ = app.emit(
                    "piper-download-progress",
                    DownloadProgress::Downloading {
                        voice_key: voice_key.to_string(),
                        bytes_downloaded,
                        total_bytes,
                    },
                );
            }
        }

        tokio::fs::rename(tmp_path, dest_path)
            .await
            .map_err(|e| format!("Failed to rename file: {}", e))?;

        Ok(())
    }

    pub fn list_installed(&self) -> Vec<InstalledModel> {
        let mut models = Vec::new();

        if !self.models_dir.exists() {
            return models;
        }

        if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }

                let voice_key = entry.file_name().to_string_lossy().to_string();
                let onnx_path = entry.path().join(format!("{}.onnx", voice_key));
                let json_path = entry.path().join(format!("{}.onnx.json", voice_key));

                if onnx_path.exists() && json_path.exists() {
                    let (language, quality, name) = self
                        .catalog
                        .as_ref()
                        .and_then(|cat| cat.voices.get(&voice_key))
                        .map(|v| (v.language.clone(), v.quality.clone(), v.name.clone()))
                        .unwrap_or_else(|| (VoiceLanguage::default(), "unknown".into(), voice_key.clone()));

                    models.push(InstalledModel {
                        voice_key,
                        model_path: onnx_path.to_string_lossy().to_string(),
                        config_path: json_path.to_string_lossy().to_string(),
                        language,
                        quality,
                        name,
                    });
                }
            }
        }

        models
    }

    pub fn delete_model(&self, voice_key: &str) -> Result<(), String> {
        let voice_dir = self.models_dir.join(voice_key);
        if voice_dir.exists() {
            std::fs::remove_dir_all(&voice_dir).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
