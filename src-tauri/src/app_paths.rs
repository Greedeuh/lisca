use crate::catalog::VoiceCatalog;
use std::path::PathBuf;
use tauri::Manager;

pub struct AppPaths {
    pub app_data_dir: PathBuf,
    pub piper_models_dir: PathBuf,
    pub kokoro_models_dir: PathBuf,
    pub resource_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(app_handle: &tauri::AppHandle) -> Self {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to resolve app data dir");
        let _ = std::fs::create_dir_all(&app_data_dir);

        let piper_models_dir = app_data_dir.join("piper_models");
        let kokoro_models_dir = app_data_dir.join("kokoro");
        let resource_dir = app_handle
            .path()
            .resource_dir()
            .unwrap_or_else(|_| app_data_dir.clone());

        Self {
            app_data_dir,
            piper_models_dir,
            kokoro_models_dir,
            resource_dir,
        }
    }

    pub fn voice_catalog(&self) -> VoiceCatalog {
        VoiceCatalog::new(
            self.piper_models_dir.clone(),
            self.kokoro_models_dir.clone(),
            &self.resource_dir,
        )
    }
}
