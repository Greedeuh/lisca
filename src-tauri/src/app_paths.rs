use std::path::PathBuf;
use tauri::Manager;

pub(super)  struct AppPaths {
    pub(super)  app_data_dir: PathBuf,
    pub(super)  piper_models_dir: PathBuf,
    pub(super)  kokoro_models_dir: PathBuf,
    pub(super)  resource_dir: PathBuf,
}

impl AppPaths {
    pub(super)  fn resolve(app_handle: &tauri::AppHandle) -> Self {
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
}
