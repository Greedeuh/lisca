# WIP — Session Summary (2026-06-19)

## Context

All 12 phases of `IMPLEMENTATION_PLAN.md` were marked DONE, but 4 critical gaps prevented the app from working end-to-end. This session closed those gaps.

**Current status:** 153 Rust tests pass, 59 frontend tests pass, clippy clean, TypeScript builds.

---

## What Was Done

### 1. Model Synthesis — Real ORT Inference (was stubs)

Both `PiperModel::synthesize()` and `KokoroModel::synthesize()` returned `Err("not yet implemented")`. Recovered working inference code from git history (commits `e53600e`, `eb7c4d8`) and adapted it.

**Piper** (`src-tauri/src/models/piper.rs`):
- Added `PiperConfig` struct (deserializes `{voice_key}.onnx.json`)
- Added `ensure_espeak_data()` to install bundled espeak-ng language data
- Implemented `text_to_phoneme_ids()`: espeak-ng text → IPA → NFD decomposition → phoneme_id_map lookup
- `synthesize()`: tokenize → ORT tensors (`input`, `input_lengths`, `scales`) → f32 audio
- Updated `PiperFactory` to load config.json alongside model.onnx

**Kokoro** (`src-tauri/src/models/kokoro.rs`):
- Added `load_vocab()` — hardcoded IPA→token ID map from Kokoro-82M
- Added `load_voice_data()` — reads .bin embedding, reshapes to (N, 256)
- Created `src-tauri/src/models/kokoro_phonemizer.rs` — basic English phonemizer (word→IPA map + letter fallback)
- `synthesize()`: phonemize → tokenize → ORT tensors (`input_ids`, `style`, `speed`) → f32 audio
- Fixed `KokoroEngine` to use `std::sync::Mutex<Session>` for ORT's `&mut self` requirement

**New dependencies** (`src-tauri/Cargo.toml`):
- `rodio = "0.20"` — audio output
- `espeak-ng = "0.1.2"` with 20 bundled languages
- `unicode-normalization = "0.1.24"` — IPA decomposition for Piper

### 2. Audio Output — RodioAudioOutput

Created `src-tauri/src/speech_player/rodio_output.rs` implementing the `AudioOutput` trait.

**Key challenge:** `rodio::OutputStream` is not `Send` (cpal constraint). Refactored `spawn_speech_player` and `play_with_controls` to accept `Arc<dyn Fn() -> ...>` factory and create audio output on the blocking thread inside `spawn_blocking`.

Changed `audio_factory` from `Box<dyn Fn>` to `Arc<dyn Fn>` to allow cloning into the blocking closure.

### 3. Transcriber — ModelPool Integration

Refactored `src-tauri/src/transcriber/mod.rs`:
- Changed `spawn_transcriber` signature: replaced `Arc<Mutex<dyn Model>>` with `Arc<Mutex<ModelPool>>` + `Arc<UnifiedFactory>`
- Created `UnifiedFactory` — wraps Piper + Kokoro factories, delegates based on `is_installed()`
- `run_loop` now resolves voice_key → determines factory → `pool.get()` → `model.synthesize()`
- Updated all tests to use `UnifiedFactory` with `MockFactory`

### 4. App Startup Wiring (`src-tauri/src/lib.rs`)

Previously, `spawn_transcriber()` and `spawn_speech_player()` were never called. Now:
- Creates `PiperFactory` + `KokoroFactory` from `app_data_dir`
- Creates `ModelPool` (max 4, idle timeout 300s)
- Spawns transcriber with event callback emitting `transcription_started`/`completed`/`error` via `app.emit()`
- Spawns periodic model pool eviction task (every 60s)
- Hotkey callback now wakes transcriber after adding text to queue
- Fixed `tokio::spawn` → `tauri::async_runtime::spawn` (Tauri setup runs outside Tokio runtime)

### 5. Frontend — Live Queue Updates

Created `src/hooks/useTtsQueue.ts`:
- Subscribes to `queue_updated`, `transcription_started`, `transcription_completed`, `playback_started`, `playback_stopped`, `item_completed`
- Re-fetches queue state on any event
- Returns `{ items, autoRead, showOverlay, refresh }`

Refactored `src/App.tsx` to use the hook instead of inline `useState` + manual event listeners.

### 6. Hotkey Re-registration

Updated `save_hotkey_cmd` in `src-tauri/src/commands.rs`:
- Unregisters all old shortcuts via `global_shortcut().unregister_all()`
- Registers new shortcut with same callback (clipboard → queue → wake transcriber)
- Saves to disk

### 7. Dead Code Cleanup

- Added `#[allow(dead_code)]` on intentionally unused fields (`warmup`, `handle`, `audio_factory`)
- Fixed clippy warnings: `is_multiple_of()`, `is_none_or()`
- All warnings resolved

### 8. Shared Queue Fix (2026-06-19 23:25)

**Problem:** AppState and transcriber used separate `Queue` instances — items added via hotkey never reached the transcriber.

**Fix:**
- Changed `AppState.queue` from `std::sync::Mutex<Queue>` to `Arc<tokio::sync::Mutex<Queue>>`
- Same for `voice_mapping` and `model_pool`
- Created single queue in `lib.rs`, cloned `Arc` for both AppState and transcriber
- Made all queue/voice_mapping Tauri commands `async` with `.lock().await`
- Synchronous contexts (close handler, hotkey callback) use `tokio::runtime::Handle::block_on()` to await the mutex
- Added `rt-multi-thread` to tokio features

---

## What Still Needs To Be Done

### Minor: Platform-Specific Frosted Glass

Phase 9 deferred native frosted glass (NSVisualEffectView on macOS, acrylic/mica on Windows). CSS `backdrop-filter: blur()` is the current fallback.

### Minor: Full Catalog

Only 1 Piper voice + 1 Kokoro voice hardcoded. HuggingFace API fetch not implemented.

### Minor: Overlay Live Updates

Overlay window doesn't use `useTtsQueue` hook yet — still shows static data.

### Minor: Deadlock Risk in Transcription Callback

The transcriber's `on_event` callback (in lib.rs) tries to wake the speech player via `state.speech_player_handle.try_lock()`. If this fails (lock held), the speech player won't be woken. Consider using `block_on` with async spawn or a dedicated channel.

---

## Fixes Applied This Session (chronological)

1. **Real ORT inference** — PiperModel + KokoroModel synthesize() with espeak-ng phonemization
2. **RodioAudioOutput** — AudioOutput trait implementation for rodio
3. **Transcriber refactor** — ModelPool + UnifiedFactory for per-voice model resolution
4. **App startup wiring** — Transcriber + speech player spawned, model pool eviction task
5. **useTtsQueue hook** — Live queue updates via IPC events
6. **Hotkey re-registration** — Unregister old + register new at runtime
7. **Shared queue fix** — Single `Arc<tokio::sync::Mutex<Queue>>` shared between AppState and transcriber
8. **Speech player wiring** — Spawned in lib.rs, woke on transcription completion
9. **Deadlock fix** — Drop queue lock before calling on_event in transcriber and speech player
10. **Kokoro engine lazy loading** — `ensure_engine()` loads from disk on first `create()` call
11. **Correct HuggingFace repos** — Kokoro: `onnx-community/Kokoro-82M-v1.0-ONNX`, Piper: `rhasspy/piper-voices`
12. **Real model downloads** — Both catalogs download from HuggingFace instead of writing zeros
13. **Queue pipeline fix** — `queue_add` wakes transcriber, hotkey uses async spawn, auto_read synced

---

## Files Modified This Session

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | +rt-multi-thread, +rodio, +espeak-ng, +unicode-normalization, +reqwest, +futures-util |
| `src-tauri/src/models/piper.rs` | Real synthesize() with espeak-ng + ORT |
| `src-tauri/src/models/kokoro.rs` | Real synthesize() with phonemizer + ORT, lazy engine |
| `src-tauri/src/models/kokoro_phonemizer.rs` | New: English phonemizer (word→IPA) |
| `src-tauri/src/models/mod.rs` | Re-export KokoroFactory |
| `src-tauri/src/speech_player/rodio_output.rs` | New: RodioAudioOutput |
| `src-tauri/src/speech_player/mod.rs` | rodio_output module, Arc factory, deadlock fix |
| `src-tauri/src/transcriber/mod.rs` | ModelPool + UnifiedFactory, deadlock fix |
| `src-tauri/src/commands.rs` | Arc<tokio::Mutex<Queue>>, async commands, hotkey async spawn, auto_read sync |
| `src-tauri/src/lib.rs` | Shared queue, speech player spawn, async hotkey |
| `src-tauri/src/catalog/kokoro.rs` | Real HuggingFace downloads |
| `src-tauri/src/catalog/piper.rs` | Real HuggingFace downloads |
| `src/hooks/useTtsQueue.ts` | New: live queue hook |
| `src/hooks/index.ts` | New: barrel export |
| `src/App.tsx` | Refactored to use useTtsQueue |
