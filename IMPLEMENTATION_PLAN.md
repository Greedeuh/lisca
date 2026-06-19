# Lisca: POC → Final App Implementation Plan (v3)

## How to Use This Plan

Each phase lists concrete tasks and **Acceptance Criteria** — testable conditions that prove the phase is done. A phase is complete when ALL acceptance criteria pass.

**Testing approach** (from `docs/testing-strategy.md` & test skill):
- Layer 1: Rust unit tests (`cargo test` from `src-tauri/`)
- Layer 3a: Pure frontend component tests (`bun run vitest run`)
- New code in each phase should include tests where applicable (Layer 1 for Rust logic, Layer 3a for presentational components)

So first think (learn from ./poc when needed, but do not copy), implement, review, improve. 
Then ensure Acceptance Criteria are met, think, improve.
And finally before going to the next phase: ask user review.

Focus on readability, simplicity, DDD, SRP and clean code.

---

## Phase 1 — Project Scaffolding & Structure

**Goal:** Fresh Tauri v2 project with clean module layout, buildable and passing CI.

### Tasks
- [x] Create fresh Tauri v2 project from POC (copy configs, clean up demo code)
- [x] Organize Rust backend into domain modules: `queue/`, `transcriber/`, `speech_player/`, `models/`, `voice_prefs/`
- [x] Organize frontend into feature folders: `components/queue/`, `components/voices/`, `components/settings/`, `overlay/`
- [x] Define shared IPC event channel types (queue, transcription, playback, model, download)
- [x] Set up CI: `.github/workflows/test.yml` running `cargo test`, `bun run build`, `bun run vitest run`

### Acceptance Criteria
- [ ] `bun run tauri build` completes without errors on Linux
- [x] `cargo test --lib` from `src-tauri/` passes (0 failures)
- [x] `bun run vitest run` passes (0 failures)
- [x] `bun run build` (type check) passes
- [x] Rust source tree has top-level modules: `queue`, `transcriber`, `speech_player`, `models`, `voice_prefs`
- [x] Frontend `src/` has directories: `components/queue/`, `components/voices/`, `components/settings/`, `overlay/`
- [x] IPC event channel type definitions exist (TypeScript types + Rust enums)
- [x] No dead/demo code remains (e.g. placeholder TTS commands, unused components)

---

## Phase 2 — Queue System & Core Interfaces ✅ DONE

**Goal:** Queue with two item types (TextMessage, Speech), lifecycle states, cursor-based consumption.

**Design decisions (deviations from original plan):**
- Items are **not persisted** — they live in memory only. Only config is persisted.
- Queue split into traits by consumer: `QueueControllable`, `Transcribable`, `Playable`
- Fine-grained events: `ItemAdded`, `ItemRemoved`, `ItemMoved`, `ItemCleared`, `ItemReplaced` (no payload, consumer queries queue directly)

### Files
- `src-tauri/src/queue/mod.rs` — types, Queue struct, builder, config persistence
- `src-tauri/src/queue/controllable.rs` — QueueControllable trait (frontend API)
- `src-tauri/src/queue/transcribable.rs` — Transcribable trait (transcriber API)
- `src-tauri/src/queue/playable.rs` — Playable trait (speech player API)
- `src-tauri/src/persist.rs` — generic save_json/load_json helpers
- `src-tauri/src/models/mod.rs` — Model trait (already existed)

### Tasks
- [x] Define `TextMessage` type (id, text, language, status: pending/processing)
- [x] Define `Speech` type (id, text, audio_path, voice_key, language, status: to_play/playing/paused/played)
- [x] Unify into a single `QueueItem` enum: `TextMessage { .. }` | `Speech { .. }`
- [x] Implement queue operations: add, replace (text→speech in-place), reorder, remove
- [x] Add transcriber cursor (tracks which TextMessage is being processed)
- [x] Add speech player cursor (tracks which Speech is being played)
- [x] Persist queue config to JSON (`{app_data_dir}/lisca/queue_config.json`)
- [x] ~~Implement startup recovery~~ — N/A (items not persisted)
- [x] Define `Model` trait interface: `fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, String>` + `fn sample_rate(&self) -> u32`
- [x] Emit fine-grained events on every mutation
- [x] Write Rust unit tests (38 tests)
- [x] Split into consumer traits (QueueControllable, Transcribable, Playable)

### Acceptance Criteria
- [x] `QueueItem::TextMessage` and `QueueItem::Speech` types exist with correct fields
- [x] `Model` trait compiles with `synthesize` and `sample_rate` methods
- [x] Queue add/remove/reorder/clear operations work (Rust unit tests pass)
- [x] TextMessage → Speech replacement preserves item position (test)
- [x] Transcriber cursor correctly identifies next pending TextMessage (test)
- [x] Speech player cursor correctly identifies next to_play Speech (test)
- [x] ~~Queue persists to `queue.json` and survives simulated restart~~ — config only, items in-memory
- [x] ~~Startup recovery~~ — N/A (items not persisted)
- [x] Fine-grained events emitted on mutations (test)
- [x] All `cargo test --lib` pass

---

## Phase 2b — Voice Preferences (Voice Mapping) ⏳ PARTIAL

**Goal:** Per-language active voice selection with fallback, persisted to disk, exposed via Tauri commands.

**Done:** VoiceMapping struct, resolve logic, 5 unit tests.
**Deferred:** Persistence (needs Serialize/Deserialize derives), Tauri commands (needs Tauri builder wiring).

### Tasks
- [x] Verify `VoiceMapping` struct: `language_voice: HashMap<String, String>`, `fallback_voice_key: Option<String>`
- [x] Verify resolve logic: known language → mapped voice, unknown → fallback, no fallback → None
- [ ] Persist to `{app_data_dir}/lisca/voice_mapping.json` — needs `#[derive(Serialize, Deserialize)]`
- [ ] Wire Tauri commands: `get_voice_preference`, `set_voice_preference` — deferred to Tauri builder setup
- [x] Write Rust unit tests for resolution logic (5 tests)

### Acceptance Criteria
- [x] `VoiceMapping::resolve(Some("en"))` returns mapped voice for "en" (test)
- [x] `VoiceMapping::resolve(Some("de"))` returns fallback when "de" not in map (test)
- [x] `VoiceMapping::resolve(None)` returns fallback when set, None when not (test)
- [x] `VoiceMapping::resolve(Some("xx"))` returns None when no mapping and no fallback (test)
- [ ] Voice mapping saves to and loads from JSON file (test: save → load → verify)
- [ ] Loading missing file returns empty default (test)
- [ ] Tauri command `get_voice_preference` returns current mapping (integration test)
- [ ] Tauri command `set_voice_preference` persists and updates resolver (integration test)
- [x] All `cargo test --lib` pass

---

## Phase 3 — Transcriber (Background Consumer)

**Goal:** Background task that dequeues TextMessages, detects language, resolves voice, synthesizes via Model, replaces with Speech.

**Note:** The POC `processor.rs` already implements this loop. This phase restructures into a dedicated `transcriber/` module with the two-item-type queue.

### Tasks
- [ ] Implement Transcriber as a tokio task consuming TextMessages from queue
- [ ] Integrate language detection (`whatlang`) on dequeue
- [ ] Resolve active voice via Voice Preferences (Phase 2b)
- [ ] Delegate synthesis to Model trait implementation
- [ ] Replace TextMessage with Speech in queue on completion (preserving position)
- [ ] Emit transcription events: `transcription_started`, `transcription_completed`, `transcription_error`
- [ ] Handle synthesis errors: emit error event, skip item, continue with next
- [ ] Write Rust unit tests for the transcriber logic (with mock Model)

### Acceptance Criteria
- [ ] Transcriber picks up TextMessage from queue when woken (test: add text, verify processing starts)
- [ ] Language detected and stored on TextMessage (test: add English text, verify language = "en")
- [ ] Voice resolved via VoiceMapping for detected language (test: set mapping, verify correct voice key)
- [ ] TextMessage replaced by Speech at same position (test: verify item id and position preserved)
- [ ] Speech contains audio path or in-memory audio data (test: verify Speech is non-empty)
- [ ] `transcription_started` event emitted when processing begins (test)
- [ ] `transcription_completed` event emitted with Speech item (test)
- [ ] `transcription_error` event emitted on synthesis failure (test: use failing mock model)
- [ ] Error item removed from queue, next item processed (test: add 2 items, first fails, second succeeds)
- [ ] Transcriber is a separate tokio task, not blocking the main thread (test: verify concurrent execution)
- [ ] All `cargo test --lib` pass

---

## Phase 4 — Audio Backend & SpeechPlayer

**Goal:** Background task that plays Speech items with full playback controls.

**Note:** The POC already has `AudioOutput` (rodio-based), `PlaybackController`, and the play loop in `processor.rs`. This phase restructures into a dedicated `speech_player/` module.

### Tasks
- [ ] Port audio output from POC (`AudioOutput` with rodio)
- [ ] Implement SpeechPlayer as a tokio task consuming Speech items from queue
- [ ] Implement playback state machine: to_play → playing → paused → played
- [ ] Implement controls: play, pause, resume, stop, skip
- [ ] Implement auto-play mode (process next item automatically when current finishes)
- [ ] Emit playback events: `playback_started`, `playback_paused`, `playback_resumed`, `playback_stopped`, `item_completed`
- [ ] Write Rust unit tests for PlaybackController state transitions

### Acceptance Criteria
- [ ] AudioOutput correctly converts f32 samples to i16 and plays via rodio (test: f32_to_i16 conversion)
- [ ] SpeechPlayer picks up Speech items when woken (test: add speech, verify playback starts)
- [ ] State transitions: idle → playing → paused → playing → idle (test: verify atomic state changes)
- [ ] Pause only works when playing (test: pause on idle is no-op)
- [ ] Resume only works when paused (test: resume on idle is no-op)
- [ ] Stop clears pause flag and sets idle (test)
- [ ] Auto-play: when current item completes, next item starts automatically (test: add 2 speeches, verify sequential play)
- [ ] Auto-read off: playback stops after current item (test)
- [ ] `playback_started` event emitted with correct item (test)
- [ ] `item_completed` event emitted when playback finishes (test)
- [ ] All `cargo test --lib` pass

---

## Phase 4b — Global Hotkey & Clipboard

**Goal:** System-wide hotkey triggers clipboard read → queue add.

**Note:** The POC already implements this in `hotkey.rs` + `clipboard.rs`. This phase verifies and restructures.

### Tasks
- [ ] Port global hotkey registration from POC (`tauri-plugin-global-shortcut`)
- [ ] Port clipboard reading from POC (`tauri-plugin-clipboard-manager` + enigo)
- [ ] Wire hotkey press → clipboard read → queue_add trigger chain
- [ ] Persist hotkey config to `{app_data_dir}/lisca/hotkey.txt`
- [ ] Register hotkey on app startup if previously saved
- [ ] Write Rust unit tests for shortcut parsing

### Acceptance Criteria
- [ ] `parse_shortcut("Control+Shift+K")` returns correct modifiers and key (test)
- [ ] `parse_shortcut("")` returns error (test)
- [ ] `parse_shortcut("Control")` returns error (no key) (test)
- [ ] Hotkey saves to `hotkey.txt` and loads back correctly (test)
- [ ] Hotkey registration succeeds with valid shortcut (manual test: press hotkey, verify queue_add called)
- [ ] Clipboard read restores original clipboard after auto-copy (manual test)
- [ ] On app startup, previously saved hotkey is re-registered (manual test)
- [ ] All `cargo test --lib` pass

---

## Phase 5 — Model Pool & Implementations

**Goal:** LRU model cache with Piper and Kokoro backends, shared engine pattern, auto-unload.

**Note:** The POC already implements `ModelPool` with LRU cache, `PiperModel`, `KokoroModel`, `KokoroBackendFactory`, `PiperBackendFactory`, and the `TtsModel` trait. This phase restructures and adds auto-unload.

### Tasks
- [ ] Implement Model Pool with LRU eviction and configurable max cached limit (default 4)
- [ ] Implement `PiperModel` (ORT binding — library, not subprocess)
- [ ] Implement `KokoroModel` with shared ONNX engine pattern
- [ ] Implement shared engine abstraction: Kokoro shared model, Piper empty placeholder
- [ ] Implement auto-unload on idle timeout (configurable seconds or infinite)
- [ ] Emit model events: `model_loaded`, `model_unloaded`
- [ ] Write Rust unit tests for ModelPool LRU eviction, factory creation

### Acceptance Criteria
- [ ] ModelPool loads a model on first access for a voice key (test)
- [ ] ModelPool evicts LRU model when cache is full (test: load 5 models with max 4, verify oldest evicted)
- [ ] ModelPool evicts model when auto-unload timeout expires (test: set short timeout, verify unload)
- [ ] `clear_cache()` removes all cached models (test)
- [ ] `refresh_installed()` syncs installed list and evicts uninstalled models (test)
- [ ] PiperModel creates ONNX session from model file (test with mock file)
- [ ] KokoroModel creates ONNX session and loads voice/tokenizer (test with mock files)
- [ ] `model_loaded` event emitted when model loaded into pool (test)
- [ ] `model_unloaded` event emitted when model evicted (test)
- [ ] All `cargo test --lib` pass

---

## Phase 6 — Voice Catalog & Install Flow

**Goal:** Browse, download, install, and uninstall Piper and Kokoro voices.

**Note:** The POC already implements `PiperCatalog` with HuggingFace fetch, download with progress, install, and delete. This phase adds Kokoro catalog and unified interface.

### Tasks
- [ ] Start with minimal hardcoded catalog (1 Piper voice + 1 Kokoro voice) for end-to-end testing
- [ ] Define unified Voice Catalog interface: `list`, `install`, `uninstall`, `list_installed`
- [ ] Implement Piper catalog (hardcoded JSON initially — defer HuggingFace API)
- [ ] Implement Kokoro catalog (hardcoded set, shared model + per-voice `.bin` vectors)
- [ ] Implement download with progress reporting (emit `download_progress` events)
- [ ] Implement file verification (checksum if available)
- [ ] Wire install/uninstall commands and download progress events
- [ ] Expand catalog to full voice set once install flow is validated
- [ ] Write Rust unit tests for catalog operations

### Acceptance Criteria
- [ ] Catalog returns list of available voices with metadata (name, language, quality, size, speed, type) (test)
- [ ] Install downloads model files to `{app_data_dir}/lisca/piper_models/` or `kokoro/` (test with mock HTTP)
- [ ] `list_installed` returns only voices with files on disk (test)
- [ ] Uninstall removes voice files from disk (test)
- [ ] Download progress events emitted during download (test: verify byte counts)
- [ ] Kokoro shared model downloaded once, per-voice `.bin` downloaded separately (test)
- [ ] Install fails gracefully if download fails (test: simulate network error)
- [ ] Checksum verification fails if file is corrupted (test, if checksums available)
- [ ] All `cargo test --lib` pass

---

## Phase 7 — Shared Queue UI Component

**Goal:** `<QueueList>` component used by both main window and overlay, showing items with full controls.

**Note:** The POC already has `QueueList.tsx` with basic item display, reorder, and remove. This phase expands to match the design spec (TextMessage vs Speech controls, download, restart).

**⚠️ Impacted by Phase 2 changes:**
- Events are now fine-grained (`ItemAdded`, `ItemRemoved`, `ItemMoved`, `ItemCleared`, `ItemReplaced`) — no payload, consumer queries queue directly via `queue_state` Tauri command
- `queue_updated` event no longer exists — frontend must listen to specific variants and fetch queue state on each event

### Tasks
- [ ] Build `<QueueList>` as a shared component (used by both main window and overlay)
- [ ] TextMessage items: text preview (truncated), status badge (pending/processing), remove control
- [ ] Speech items: text preview, status badge (to_play/playing/played), play/pause/stop/restart/remove/download/reorder controls
- [ ] Shared controls: auto-play toggle, clear all
- [ ] Wire to IPC events for real-time updates (listen to `queue_updated`, `playback_started`, etc.)
- [ ] Write frontend component tests for QueueList rendering and interactions

### Acceptance Criteria
- [ ] QueueList renders TextMessage items with text preview and status (component test)
- [ ] QueueList renders Speech items with status and control buttons (component test)
- [ ] "Remove" button calls onRemove with correct item id (component test)
- [ ] "Up"/"Down" buttons call onMove with correct id and index (component test)
- [ ] Auto-play toggle checkbox calls onToggleAutoRead (component test)
- [ ] "Clear" button calls onClear (component test)
- [ ] Currently playing item highlighted with "Playing" or "Paused" badge (component test)
- [ ] Empty state shows "Queue is empty" message (component test)
- [ ] Component updates in real-time when IPC events arrive (component test with mocked events)
- [ ] All `bun run vitest run` pass

---

## Phase 8 — Frontend: Main Window

**Goal:** Main configuration window with voice catalog browser, installed voices, queue list, and hotkey config.

**Note:** The POC has basic versions of `HotkeyRecorder`, `ModelConfig`, `PiperModelPicker`, `VoiceBrowser`, `VoiceRow`, `InstalledModels`, `DownloadProgress`, `TtsQueue`. This phase restructures and completes them.

**⚠️ Impacted by Phase 2b deferral:**
- Voice mapping settings (`get_voice_preference`, `set_voice_preference` Tauri commands) must be wired in this phase
- VoiceMapping needs `#[derive(Serialize, Deserialize)]` added before persistence works

### Tasks
- [ ] Voice Catalog browser: browse available voices by language, quality/size/speed/type metadata
- [ ] Installed Voices list: active/inactive per language, set active, uninstall, set fallback
- [ ] Embed shared `<QueueList>` (no frosted glass)
- [ ] Hotkey Configuration: record + persist global hotkey, display current hotkey
- [ ] Voice mapping settings: per-language voice selection, fallback voice selection
- [ ] Write frontend component tests for each settings panel

### Acceptance Criteria
- [ ] Voice catalog lists available voices grouped by language (component test)
- [ ] Download button triggers download, progress bar shows during download (component test)
- [ ] Installed voices list shows model name, language, status badge (component test)
- [ ] "Set Active" button updates voice mapping for language (component test)
- [ ] "Uninstall" button removes voice from installed list (component test)
- [ ] "Set Fallback" button sets fallback voice (component test)
- [ ] HotkeyRecorder captures key combination and displays it (component test)
- [ ] HotkeyRecorder saves via invoke (component test with mock)
- [ ] VoiceMappingSettings shows per-language dropdown and fallback dropdown (component test)
- [ ] QueueList embedded in main window shows queue items (component test)
- [ ] All `bun run vitest run` pass

---

## Phase 9 — Frontend: Overlay & Window Config

**Goal:** Frosted glass overlay window, top-right, transparent, always-on-top, shows queue when main window is closed.

**Note:** The POC already implements overlay creation, positioning, show/hide in `overlay.rs`, and `QueueOverlay.tsx`. This phase restructures for the two-item-type queue.

### Tasks
- [ ] Frosted glass overlay window (top-right, only visible when queue has items)
- [ ] Platform-specific: NSVisualEffectView (macOS), acrylic/mica (Windows), best-effort (Linux)
- [ ] Tauri window config: always-on-top, no taskbar entry, transparent
- [ ] Embed shared `<QueueList>` with overlay styling
- [ ] Overlay auto-show when queue has items and main window is closed
- [ ] Overlay auto-hide when queue becomes empty
- [ ] Drag region on overlay header for repositioning

### Acceptance Criteria
- [ ] Overlay window is created with correct Tauri config: `decorations: false`, `transparent: true`, `always_on_top: true`, `skip_taskbar: true` (test: verify window properties)
- [ ] Overlay positioned at top-right of monitor on first show (test: verify position calculation)
- [ ] Overlay shows when main window closes and queue has items (manual test)
- [ ] Overlay hides when queue becomes empty (manual test)
- [ ] Overlay is draggable by header (manual test)
- [ ] Overlay shows TextMessage items with status and remove control (manual test)
- [ ] Overlay shows Speech items with status and playback controls (manual test)
- [ ] Auto-play toggle and Clear button functional in overlay (manual test)
- [ ] Close (✕) button hides overlay and disables show_overlay setting (manual test)
- [ ] On Windows: overlay stays above all other windows (manual test)

---

## Phase 10 — System Tray & Window Management

**Goal:** System tray icon with menu, close-to-tray behavior.

**Note:** The POC already implements tray icon with Show/Quit menu and close-to-tray in `lib.rs`. This phase adds Show/Hide Overlay toggle.

### Tasks
- [ ] Tray icon with menu: Show, Show/Hide Overlay, Quit
- [ ] Close-to-tray behavior (hide window instead of quit)
- [ ] Wire overlay visibility to tray toggle
- [ ] Left-click tray icon shows main window
- [ ] Right-click tray icon shows context menu

### Acceptance Criteria
- [ ] Tray icon appears in system tray with "Lisca" tooltip (manual test)
- [ ] Left-click tray icon shows main window and hides overlay (manual test)
- [ ] "Show" menu item shows main window (manual test)
- [ ] "Show/Hide Overlay" menu item toggles overlay visibility (manual test)
- [ ] "Quit" menu item exits the application (manual test)
- [ ] Closing main window hides to tray (app keeps running) (manual test)
- [ ] Closing main window shows overlay if queue has items and show_overlay is enabled (manual test)
- [ ] Closing main window does NOT show overlay if queue is empty (manual test)

---

## Phase 11 — Error Handling & Logging

**Goal:** Structured error types, no silent failures, errors surfaced to UI, diagnostic logging.

### Tasks
- [ ] Define structured error types incrementally per module (not as a final sweep)
- [ ] Surface errors to UI via events (no silent failures)
- [ ] Implement logging at different levels for diagnostics
- [ ] Wire error events to frontend notification system
- [ ] Replace `.unwrap()` calls in hot paths with `?` or `.unwrap_or_else()` (per `testing-strategy.md`)

### Acceptance Criteria
- [ ] Every module has a structured error type (e.g. `QueueError`, `TranscriberError`, `ModelError`) (code review)
- [ ] No `.unwrap()` calls in IPC handlers or file I/O paths (`cargo clippy -- -W clippy::unwrap_used` passes)
- [ ] Errors emitted via events are visible in frontend (test: trigger error, verify event received)
- [ ] Logging at `error`, `warn`, `info`, `debug` levels for key operations (code review)
- [ ] Corrupted config files don't crash app — defaults used instead (test: write corrupt JSON, verify app loads)
- [ ] Missing model files produce error message, not panic (test: remove model file, verify graceful failure)

---

## Phase 12 — Polish & Hardening

**Goal:** End-to-end validation, edge case testing, cleanup.

**⚠️ Impacted by Phase 2 changes:**
- "Test queue recovery on app restart" — N/A, items are not persisted (in-memory only)
- "App restart recovers queue" — N/A, only config persists

### Tasks
- [ ] End-to-end testing of full flow: hotkey → clipboard → transcribe → play
- [ ] Test multi-item queue with mixed languages
- [ ] Test model pool eviction and auto-unload
- [ ] Test overlay visibility logic
- [ ] Test queue recovery on app restart
- [ ] Test concurrent access: multiple hotkey presses, rapid queue operations
- [ ] Clean up POC directory
- [ ] Final pass: remove unused code, verify all tests pass

### Acceptance Criteria
- [ ] Full flow works: press hotkey with selected text → queue item appears → audio plays (manual test)
- [ ] Mixed language queue: English → French → German items process with correct voices (manual test)
- [ ] Model pool evicts LRU when limit reached, auto-unloads after timeout (manual test with logs)
- [ ] Overlay shows/hides correctly based on queue state and main window visibility (manual test)
- [ ] App restart recovers queue, re-registers hotkey, restores settings (manual test)
- [ ] Rapid hotkey presses don't crash or duplicate items (manual test)
- [ ] `cargo test --lib` passes (0 failures)
- [ ] `bun run vitest run` passes (0 failures)
- [ ] `bun run build` (type check) passes
- [ ] `cargo clippy` passes with no warnings
- [ ] POC directory removed or clearly marked as reference-only

---

## Summary of Acceptance Criteria Counts

| Phase | Criteria | Testing Layer |
|-------|----------|---------------|
| Phase 1 — Scaffolding | 8 | Build + CI verification |
| Phase 2 — Queue System | 10 (8 done, 2 N/A) | Layer 1 (Rust unit tests) |
| Phase 2b — Voice Preferences | 9 | Layer 1 (Rust unit tests) |
| Phase 3 — Transcriber | 11 | Layer 1 (Rust unit tests) |
| Phase 4 — SpeechPlayer | 10 | Layer 1 (Rust unit tests) |
| Phase 4b — Hotkey & Clipboard | 8 | Layer 1 + manual |
| Phase 5 — Model Pool | 9 | Layer 1 (Rust unit tests) |
| Phase 6 — Voice Catalog | 9 | Layer 1 (Rust unit tests) |
| Phase 7 — Queue UI | 10 | Layer 3a (Frontend tests) |
| Phase 8 — Main Window | 11 | Layer 3a (Frontend tests) |
| Phase 9 — Overlay | 10 | Manual + build verification |
| Phase 10 — System Tray | 8 | Manual testing |
| Phase 11 — Error Handling | 6 | Layer 1 + clippy |
| Phase 12 — Polish | 11 | All layers + manual |
| **Total** | **130** | |
