use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
    #[serde(rename = "kokoro")]
    Kokoro {
        model_path: String,
        voice_path: String,
    },
    #[serde(rename = "piper")]
    Piper {
        model_path: String,
        config_path: String,
    },
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self::Piper {
            model_path: "models/en_US-lessac-medium.onnx".into(),
            config_path: "models/en_US-lessac-medium.onnx.json".into(),
        }
    }
}

impl BackendConfig {
    pub fn resolve_path(path: &str, base_dir: &Path) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            base_dir.join(p)
        }
    }
}

pub fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("lisca").join("config.json")
}

pub fn load_config(app_data_dir: &Path) -> BackendConfig {
    let path = config_path(app_data_dir);
    if !path.exists() {
        return BackendConfig::default();
    }
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return BackendConfig::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_config(app_data_dir: &Path, config: &BackendConfig) -> Result<(), String> {
    let path = config_path(app_data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}
