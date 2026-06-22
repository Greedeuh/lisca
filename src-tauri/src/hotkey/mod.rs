// Global hotkey registration, shortcut parsing, and persistence.
// Parses "Control+Shift+K" style strings into modifiers + key.
// Persists the configured hotkey to {app_data_dir}/lisca/hotkey.txt.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super)  struct ShortcutConfig {
     modifiers: Vec<String>,
     key: String,
}

impl ShortcutConfig {
     fn new(modifiers: Vec<String>, key: String) -> Self {
        Self { modifiers, key }
    }

    pub(super)  fn to_string_repr(&self) -> String {
        let mut parts = self.modifiers.clone();
        parts.push(self.key.clone());
        parts.join("+")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super)  enum ParseError {
    EmptyInput,
    NoKey,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "shortcut string is empty"),
            ParseError::NoKey => write!(f, "no key specified (only modifiers)"),
        }
    }
}

const VALID_MODIFIERS: &[&str] = &["Control", "Alt", "Shift", "Super", "Command"];

pub(super)  fn parse_shortcut(input: &str) -> Result<ShortcutConfig, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let parts: Vec<&str> = input.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut modifiers = Vec::new();
    let mut key: Option<String> = None;

    for part in &parts {
        if VALID_MODIFIERS.contains(part) {
            modifiers.push(part.to_string());
        } else if key.is_some() {
            return Err(ParseError::NoKey);
        } else {
            key = Some(part.to_string());
        }
    }

    match key {
        Some(k) => Ok(ShortcutConfig::new(modifiers, k)),
        None => Err(ParseError::NoKey),
    }
}

pub(super)  fn save_hotkey(path: &Path, config: &ShortcutConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, config.to_string_repr()).map_err(|e| e.to_string())
}

pub(super)  fn load_hotkey(path: &Path) -> Option<ShortcutConfig> {
    let data = std::fs::read_to_string(path).ok()?;
    let data = data.trim();
    if data.is_empty() {
        return None;
    }
    parse_shortcut(data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_control_shift_k() {
        let config = parse_shortcut("Control+Shift+K").unwrap();
        assert_eq!(config.modifiers, vec!["Control", "Shift"]);
        assert_eq!(config.key, "K");
    }

    #[test]
    fn parse_alt_a() {
        let config = parse_shortcut("Alt+A").unwrap();
        assert_eq!(config.modifiers, vec!["Alt"]);
        assert_eq!(config.key, "A");
    }

    #[test]
    fn parse_super_space() {
        let config = parse_shortcut("Super+Space").unwrap();
        assert_eq!(config.modifiers, vec!["Super"]);
        assert_eq!(config.key, "Space");
    }

    #[test]
    fn parse_empty_returns_error() {
        assert!(matches!(parse_shortcut(""), Err(ParseError::EmptyInput)));
    }

    #[test]
    fn parse_whitespace_only_returns_error() {
        assert!(matches!(parse_shortcut("   "), Err(ParseError::EmptyInput)));
    }

    #[test]
    fn parse_modifier_only_returns_error() {
        assert!(matches!(parse_shortcut("Control"), Err(ParseError::NoKey)));
    }

    #[test]
    fn parse_two_modifiers_no_key() {
        assert!(matches!(
            parse_shortcut("Control+Shift"),
            Err(ParseError::NoKey)
        ));
    }

    #[test]
    fn parse_invalid_modifier() {
        let result = parse_shortcut("Invalid+K");
        assert!(result.is_err());
    }

    #[test]
    fn parse_case_sensitive() {
        assert!(parse_shortcut("control+K").is_err());
        assert!(parse_shortcut("Control+K").is_ok());
    }

    #[test]
    fn roundtrip_to_string() {
        let config = parse_shortcut("Control+Shift+K").unwrap();
        assert_eq!(config.to_string_repr(), "Control+Shift+K");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("lisca_hotkey_test");
        let path = dir.join("hotkey.txt");
        let _ = std::fs::remove_file(&path);

        let config = ShortcutConfig::new(
            vec!["Control".to_string(), "Shift".to_string()],
            "K".to_string(),
        );
        save_hotkey(&path, &config).unwrap();

        let loaded = load_hotkey(&path).unwrap();
        assert_eq!(loaded, config);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_none() {
        let path = std::env::temp_dir().join("nonexistent_hotkey.txt");
        assert!(load_hotkey(&path).is_none());
    }

    #[test]
    fn load_empty_file_returns_none() {
        let dir = std::env::temp_dir().join("lisca_hotkey_test_empty");
        let path = dir.join("hotkey.txt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "").unwrap();

        assert!(load_hotkey(&path).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = std::env::temp_dir()
            .join("lisca_hotkey_test_dirs")
            .join("nested");
        let path = dir.join("hotkey.txt");

        let config = ShortcutConfig::new(vec!["Alt".to_string()], "X".to_string());
        save_hotkey(&path, &config).unwrap();

        assert!(path.exists());

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("lisca_hotkey_test_dirs"));
    }
}
