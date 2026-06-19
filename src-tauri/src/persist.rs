use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
    struct TestData {
        name: String,
        values: Vec<u32>,
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("lisca_persist_test_rt");
        let path = dir.join("test.json");
        let _ = fs::remove_file(&path);
        fs::create_dir_all(&dir).unwrap();

        let data = TestData {
            name: "hello".to_string(),
            values: vec![1, 2, 3],
        };
        save_json(&path, &data).unwrap();
        let loaded: TestData = load_json(&path);
        assert_eq!(data, loaded);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = std::env::temp_dir().join("nonexistent_lisca_file.json");
        let loaded: TestData = load_json(&path);
        assert_eq!(loaded, TestData::default());
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let dir = std::env::temp_dir().join("lisca_persist_test_corrupt");
        let path = dir.join("corrupt.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "not valid json {{{").unwrap();

        let loaded: TestData = load_json(&path);
        assert_eq!(loaded, TestData::default());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = std::env::temp_dir()
            .join("lisca_persist_test_dirs")
            .join("nested")
            .join("dirs");
        let path = dir.join("test.json");

        let data = TestData {
            name: "test".to_string(),
            values: vec![],
        };
        save_json(&path, &data).unwrap();
        assert!(path.exists());

        let _ = fs::remove_dir_all(std::env::temp_dir().join("lisca_persist_test_dirs"));
    }
}
