# Lisca: POC → Final App Implementation Plan (v2)

## Phase 1 — Project Scaffolding & Structure
- [ ] Create fresh Tauri v2 project from poc (copy configs, clean up)
- [ ] Organize Rust backend into domain modules: `queue/`, `transcriber/`, `speech_player/`, `models/`, `voice_prefs/`
- [ ] Organize frontend into feature folders: `components/queue/`, `components/voices/`, `components/settings/`, `overlay/`
- [ ] Define shared IPC event channel types (queue, transcription, playback, model, download)

## Phase 2 — Queue System & Core Interfaces
- [ ] Implement `TextMessage` and `Speech` item types with lifecycle states
- [ ] Decide audio format for Speech items (in-memory PCM vs temp file — affects serialization and download)
- [ ] Implement Queue with cursors (transcriber cursor, speech player cursor)
- [ ] Implement queue operations: add, replace (text→speech in-place), reorder, remove, get-next-by-cursor
- [ ] Decide persistence format (JSON / SQLite / bincode)
- [ ] Persist queue state to disk
- [ ] Implement startup recovery: restore queue, reset in-progress items to pending
- [ ] Define `Model` trait interface (just `synthesize` + `sample_rate` — no implementations yet)
- [ ] Emit `queue_updated` events on every mutation

## Phase 2b — Voice Preferences
- [ ] Implement per-language active voice selection (JSON map)
- [ ] Implement fallback voice configuration
- [ ] Persist to `{app_data_dir}/lisca/voice_preferences.json`
- [ ] Wire get/set preference commands

## Phase 3 — Transcriber (Background Consumer)
- [ ] Implement Transcriber as a concurrent task consuming Text Messages from queue
- [ ] Integrate language detection (whatlang)
- [ ] Resolve active voice via Voice Preferences (Phase 2b)
- [ ] Delegate synthesis to Model Pool (programs against Model trait from Phase 2)
- [ ] Replace Text Message with Speech in queue on completion
- [ ] Emit transcription events (started, completed, error)

## Phase 4 — Audio Backend & SpeechPlayer
- [ ] Port audio output from poc (rodio-based, already working)
- [ ] Implement SpeechPlayer as a concurrent task consuming Speech items from queue
- [ ] Implement playback state machine: to_play → playing → paused → played
- [ ] Implement controls: play, pause, resume, stop, skip
- [ ] Implement auto-play mode (process next item automatically)
- [ ] Emit playback events (started, paused, resumed, stopped, item_completed)

## Phase 4b — Global Hotkey & Clipboard
- [ ] Port global hotkey registration from poc (tauri-plugin-global-shortcut)
- [ ] Port clipboard reading from poc (tauri-plugin-clipboard-manager)
- [ ] Wire hotkey press → clipboard read → queue_add trigger chain
- [ ] Persist hotkey config to `{app_data_dir}/lisca/hotkey.txt`

## Phase 5 — Model Pool & Implementations
- [ ] Implement Model Pool with LRU eviction and max cached limit (configurable)
- [ ] Implement `PiperModel` (ORT binding — library, not subprocess)
- [ ] Implement `KokoroModel` with shared ONNX engine pattern
- [ ] Implement shared engine abstraction (Kokoro shared model, Piper empty placeholder)
- [ ] Implement auto-unload on idle timeout
- [ ] Emit model events (loaded, unloaded)

> **Smoke test checkpoint** — run one hotkey → clipboard → transcribe → play cycle with a single hardcoded voice before proceeding.

## Phase 6 — Voice Catalog & Install Flow
- [ ] Start with minimal hardcoded catalog (1 Piper voice + 1 Kokoro voice) for end-to-end testing
- [ ] Define unified Voice Catalog interface (list, install, uninstall, list_installed)
- [ ] Implement Piper catalog (hardcoded JSON initially — defer HuggingFace API)
- [ ] Implement Kokoro catalog (hardcoded set, shared model + per-voice vectors)
- [ ] Implement download with progress reporting
- [ ] Implement file verification (checksum if available)
- [ ] Wire install/uninstall commands and download progress events
- [ ] Expand catalog to full voice set once install flow is validated

## Phase 7 — Shared Queue UI Component
- [ ] Build `<QueueList>` as a shared component (used by both main window and overlay)
- [ ] Text Message items: text preview, status, remove control
- [ ] Speech items: text preview, status, play/pause/stop/restart/remove/download/reorder controls
- [ ] Shared controls: auto-play toggle, clear all
- [ ] Wire to IPC events for real-time updates

## Phase 8 — Frontend: Main Window
- [ ] Voice Catalog browser (browse available voices, quality/speed/size metadata)
- [ ] Installed Voices list (active/inactive, set active, uninstall, set fallback)
- [ ] Embed shared `<QueueList>` (no frosted glass)
- [ ] Hotkey Configuration (record + persist global hotkey)

## Phase 9 — Frontend: Overlay & Window Config
- [ ] Frosted glass overlay window (top-right, only visible when queue has items)
- [ ] Platform-specific: NSVisualEffectView (macOS), acrylic/mica (Windows), best-effort (Linux)
- [ ] Tauri window config: always-on-top, no taskbar entry, transparent
- [ ] Embed shared `<QueueList>` with overlay styling
- [ ] Overlay auto-show when queue has items and main window is closed

## Phase 10 — System Tray & Window Management
- [ ] Tray icon with menu: Show, Show/Hide Overlay, Quit
- [ ] Close-to-tray behavior (hide window instead of quit)
- [ ] Wire overlay visibility to tray toggle

## Phase 11 — Error Handling & Logging
- [ ] Define structured error types incrementally per module (not as a final sweep)
- [ ] Surface errors to UI via events (no silent failures)
- [ ] Implement logging at different levels for diagnostics
- [ ] Wire error events to frontend notification system

## Phase 12 — Polish & Hardening
- [ ] End-to-end testing of full flow: hotkey → clipboard → transcribe → play
- [ ] Test multi-item queue with mixed languages
- [ ] Test model pool eviction and auto-unload
- [ ] Test overlay visibility logic
- [ ] Test queue recovery on app restart
- [ ] Clean up poc directory
