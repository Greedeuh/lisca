// Tauri command handlers for IPC between frontend and backend.
// Each command is a thin wrapper calling into domain modules.

use crate::catalog::{DownloadProgress, InstalledVoice, VoiceCatalog, VoiceCatalogOps};
use crate::hotkey::{ShortcutConfig, load_hotkey, parse_shortcut, save_hotkey};
use crate::queue::{Queue, QueueControllable, QueueItem};
use crate::voice_prefs::VoiceMapping;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub struct AppState {
    pub catalog: VoiceCatalog,
    pub queue: Mutex<Queue>,
    pub voice_mapping: Mutex<VoiceMapping>,
    pub app_data_dir: PathBuf,
}

// ── Catalog commands ──────────────────────────────────────────────

#[tauri::command]
pub fn list_catalog_voices(state: tauri::State<AppState>) -> Vec<VoiceEntryDto> {
    state
        .catalog
        .list_available()
        .into_iter()
        .map(VoiceEntryDto::from)
        .collect()
}

#[tauri::command]
pub fn list_installed_voices(state: tauri::State<AppState>) -> Vec<InstalledVoiceDto> {
    state
        .catalog
        .list_installed()
        .into_iter()
        .map(InstalledVoiceDto::from)
        .collect()
}

#[tauri::command]
pub async fn install_voice(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    voice_key: String,
) -> Result<InstalledVoiceDto, String> {
    let catalog = &state.catalog;
    let result = catalog
        .install(&voice_key, |progress| {
            let event_name = match &progress {
                DownloadProgress::Downloading { .. } => "download_progress",
                DownloadProgress::Complete { .. } => "download_complete",
                DownloadProgress::Error { .. } => "download_error",
            };
            if let Err(e) = app.emit(event_name, &progress) {
                log::warn!("Failed to emit download event: {e}");
            }
        })
        .await?;
    Ok(InstalledVoiceDto::from(result))
}

#[tauri::command]
pub fn uninstall_voice(
    state: tauri::State<AppState>,
    app: AppHandle,
    voice_key: String,
) -> Result<(), String> {
    state.catalog.uninstall(&voice_key)?;
    if let Err(e) = app.emit("voice_uninstalled", &voice_key) {
        log::warn!("Failed to emit voice_uninstalled event: {e}");
    }
    Ok(())
}

// ── Queue commands ────────────────────────────────────────────────

#[tauri::command]
pub fn queue_state(state: tauri::State<AppState>) -> Result<QueueSnapshotDto, String> {
    let queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    Ok(QueueSnapshotDto {
        items: queue.items().iter().map(QueueItemDto::from).collect(),
        auto_read: queue.config().auto_read,
        show_overlay: queue.config().show_overlay,
    })
}

#[tauri::command]
pub fn queue_add(state: tauri::State<AppState>, text: String) -> Result<u64, String> {
    let mut queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    let id = queue.add_text(text)?;
    log::info!("Added item {id} to queue");
    Ok(id)
}

#[tauri::command]
pub fn queue_remove(state: tauri::State<AppState>, id: u64) -> Result<(), String> {
    let mut queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    queue.remove(id)?;
    log::info!("Removed item {id} from queue");
    Ok(())
}

#[tauri::command]
pub fn queue_move(state: tauri::State<AppState>, id: u64, index: usize) -> Result<(), String> {
    let mut queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    queue.reorder(id, index)
}

#[tauri::command]
pub fn queue_clear(state: tauri::State<AppState>) -> Result<(), String> {
    let mut queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    queue.clear()?;
    log::info!("Queue cleared");
    Ok(())
}

#[tauri::command]
pub fn queue_toggle_auto_read(state: tauri::State<AppState>) -> Result<bool, String> {
    let mut queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    queue.config.auto_read = !queue.config.auto_read;
    let val = queue.config.auto_read;
    if let Err(e) = queue.save_config() {
        log::error!("Failed to save queue config: {e}");
    }
    Ok(val)
}

// ── Voice mapping commands ────────────────────────────────────────

#[tauri::command]
pub fn get_voice_preference(state: tauri::State<AppState>) -> Result<VoiceMappingDto, String> {
    let mapping = state
        .voice_mapping
        .lock()
        .map_err(|e| format!("failed to lock voice mapping: {e}"))?;
    Ok(VoiceMappingDto::from(&*mapping))
}

#[tauri::command]
pub fn set_voice_preference(
    state: tauri::State<AppState>,
    language: String,
    voice_key: String,
) -> Result<(), String> {
    let mut mapping = state
        .voice_mapping
        .lock()
        .map_err(|e| format!("failed to lock voice mapping: {e}"))?;
    mapping.language_voice.insert(language, voice_key);
    let path = state.app_data_dir.join("voice_mapping.json");
    mapping.save(&path)
}

#[tauri::command]
pub fn set_fallback_voice(state: tauri::State<AppState>, voice_key: Option<String>) -> Result<(), String> {
    let mut mapping = state
        .voice_mapping
        .lock()
        .map_err(|e| format!("failed to lock voice mapping: {e}"))?;
    mapping.fallback_voice_key = voice_key;
    let path = state.app_data_dir.join("voice_mapping.json");
    mapping.save(&path)
}

// ── Hotkey commands ───────────────────────────────────────────────

#[tauri::command]
pub fn get_hotkey(state: tauri::State<AppState>) -> Option<ShortcutConfig> {
    let path = state.app_data_dir.join("hotkey.txt");
    load_hotkey(&path)
}

#[tauri::command]
pub fn save_hotkey_cmd(state: tauri::State<AppState>, shortcut: String) -> Result<ShortcutConfig, String> {
    let config = parse_shortcut(&shortcut).map_err(|e| e.to_string())?;
    let path = state.app_data_dir.join("hotkey.txt");
    save_hotkey(&path, &config)?;
    Ok(config)
}

// ── Overlay commands ──────────────────────────────────────────────

#[tauri::command]
pub fn create_overlay_window(app: AppHandle) -> Result<(), String> {
    crate::overlay::create_overlay(&app)
}

#[tauri::command]
pub fn show_overlay_window(app: AppHandle) -> Result<(), String> {
    crate::overlay::show_overlay(&app)
}

#[tauri::command]
pub fn hide_overlay_window(app: AppHandle) -> Result<(), String> {
    crate::overlay::hide_overlay(&app)
}

#[tauri::command]
pub fn toggle_overlay_window(app: AppHandle) -> Result<bool, String> {
    crate::overlay::toggle_overlay(&app)
}

#[tauri::command]
pub fn queue_toggle_overlay(state: tauri::State<AppState>) -> Result<bool, String> {
    let mut queue = state
        .queue
        .lock()
        .map_err(|e| format!("failed to lock queue: {e}"))?;
    queue.config.show_overlay = !queue.config.show_overlay;
    let val = queue.config.show_overlay;
    if let Err(e) = queue.save_config() {
        log::error!("Failed to save queue config: {e}");
    }
    Ok(val)
}

// ── DTO types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceEntryDto {
    pub voice_key: String,
    pub name: String,
    pub language: String,
    pub quality: String,
    pub size_bytes: u64,
    pub speed: Option<String>,
    pub model_type: String,
}

impl From<crate::catalog::VoiceEntry> for VoiceEntryDto {
    fn from(e: crate::catalog::VoiceEntry) -> Self {
        Self {
            voice_key: e.voice_key,
            name: e.name,
            language: e.language,
            quality: e.quality,
            size_bytes: e.size_bytes,
            speed: e.speed,
            model_type: match e.model_type {
                crate::catalog::ModelType::Piper => "piper".to_string(),
                crate::catalog::ModelType::Kokoro => "kokoro".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVoiceDto {
    pub voice_key: String,
    pub name: String,
    pub language: String,
    pub quality: String,
    pub model_type: String,
    pub model_path: String,
}

impl From<InstalledVoice> for InstalledVoiceDto {
    fn from(v: InstalledVoice) -> Self {
        Self {
            voice_key: v.voice_key,
            name: v.name,
            language: v.language,
            quality: v.quality,
            model_type: match v.model_type {
                crate::catalog::ModelType::Piper => "piper".to_string(),
                crate::catalog::ModelType::Kokoro => "kokoro".to_string(),
            },
            model_path: v.model_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QueueItemDto {
    TextMessage {
        id: u64,
        text: String,
        language: Option<String>,
        status: String,
    },
    Speech {
        id: u64,
        text: String,
        language: Option<String>,
        voice_key: Option<String>,
        status: String,
    },
}

impl From<&QueueItem> for QueueItemDto {
    fn from(item: &QueueItem) -> Self {
        match item {
            QueueItem::TextMessage {
                id,
                text,
                language,
                status,
            } => QueueItemDto::TextMessage {
                id: *id,
                text: text.clone(),
                language: language.clone(),
                status: format!("{:?}", status).to_lowercase(),
            },
            QueueItem::Speech {
                id,
                text,
                language,
                voice_key,
                status,
                ..
            } => QueueItemDto::Speech {
                id: *id,
                text: text.clone(),
                language: language.clone(),
                voice_key: voice_key.clone(),
                status: format!("{:?}", status).to_lowercase(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshotDto {
    pub items: Vec<QueueItemDto>,
    pub auto_read: bool,
    pub show_overlay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMappingDto {
    pub language_voice: std::collections::HashMap<String, String>,
    pub fallback_voice_key: Option<String>,
}

impl From<&VoiceMapping> for VoiceMappingDto {
    fn from(m: &VoiceMapping) -> Self {
        Self {
            language_voice: m.language_voice.clone(),
            fallback_voice_key: m.fallback_voice_key.clone(),
        }
    }
}
