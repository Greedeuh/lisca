use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    if !path.exists() {
        return T::default();
    }
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return T::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}
