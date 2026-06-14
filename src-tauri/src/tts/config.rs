use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::persist;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
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
    persist::load_json(&path)
}

pub fn save_config(app_data_dir: &Path, config: &BackendConfig) -> Result<(), String> {
    let path = config_path(app_data_dir);
    persist::save_json(&path, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piper_default() {
        let config = BackendConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"piper\""));
        assert!(json.contains("en_US-lessac-medium.onnx"));
    }

    #[test]
    fn piper_serde_roundtrip() {
        let config = BackendConfig::Piper {
            model_path: "models/en_US-lessac-medium.onnx".into(),
            config_path: "models/en_US-lessac-medium.onnx.json".into(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BackendConfig = serde_json::from_str(&json).unwrap();
        let BackendConfig::Piper { model_path, config_path } = deserialized;
        assert_eq!(model_path, "models/en_US-lessac-medium.onnx");
        assert_eq!(config_path, "models/en_US-lessac-medium.onnx.json");
    }

    #[test]
    fn resolve_path_absolute() {
        let base = PathBuf::from("/base/dir");
        let result = BackendConfig::resolve_path("/absolute/path", &base);
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn resolve_path_relative() {
        let base = PathBuf::from("/base/dir");
        let result = BackendConfig::resolve_path("models/voice.onnx", &base);
        assert_eq!(result, PathBuf::from("/base/dir/models/voice.onnx"));
    }

    #[test]
    fn save_and_load_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = BackendConfig::Piper {
            model_path: "test.onnx".into(),
            config_path: "test.onnx.json".into(),
        };
        save_config(dir.path(), &config).unwrap();
        let loaded = load_config(dir.path());
        let BackendConfig::Piper { model_path, config_path } = loaded;
        assert_eq!(model_path, "test.onnx");
        assert_eq!(config_path, "test.onnx.json");
    }

    #[test]
    fn load_missing_config_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(dir.path());
        assert!(matches!(config, BackendConfig::Piper { .. }));
    }
}
