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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    struct TestValue {
        name: String,
        count: u32,
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        let value = TestValue { name: "hello".into(), count: 42 };
        save_json(&path, &value).unwrap();
        let loaded: TestValue = load_json(&path);
        assert_eq!(loaded, value);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let loaded: TestValue = load_json(&path);
        assert_eq!(loaded, TestValue::default());
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, "not valid json {{{").unwrap();
        let loaded: TestValue = load_json(&path);
        assert_eq!(loaded, TestValue::default());
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deep").join("test.json");
        let value = TestValue { name: "test".into(), count: 1 };
        save_json(&path, &value).unwrap();
        assert!(path.exists());
    }
}
