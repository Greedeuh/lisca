# Settings & Persistence

## Feature
All user preferences and state are persisted to disk and restored on app startup.

## Scenarios

- **As a user**, my backend config (which TTS engine and model paths) is saved to `config.json` and restored on startup.
- **As a user**, my global hotkey is saved to `hotkey.txt` and re-registered on startup.
- **As a user**, my queue items are saved to `queue.json` and restored on startup, so I don't lose queued text.
- **As a user**, my queue settings (auto-read, show overlay, max items) are saved to `queue_config.json` and restored on startup.
- **As a user**, the voice catalog cache is saved locally so the catalog loads fast on subsequent starts without fetching from the network.
- **As a user**, if any config file is corrupted or missing, the app uses sensible defaults instead of crashing.

## Persistence Files

| File | Content |
|------|---------|
| `config.json` | BackendConfig (Kokoro or Piper paths) |
| `queue.json` | Array of QueueItem (id + text) |
| `queue_config.json` | QueueConfig (max_items, auto_read, show_overlay) |
| `hotkey.txt` | Hotkey shortcut string (e.g. `Control+Shift+T`) |
| `piper_voices_cache.json` | Cached HuggingFace voice catalog |

All files are stored under `{app_data_dir}/lisca/`.

## Key Files
- `src-tauri/src/persist.rs` — generic JSON load/save
- `src-tauri/src/tts/config.rs` — backend config persistence
- `src-tauri/src/tts/queue.rs` — queue and queue config persistence
- `src-tauri/src/hotkey.rs` — hotkey persistence
