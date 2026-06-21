# TTS App — Rebuild Specification

## Overview

A desktop text-to-speech application. User selects text anywhere → presses hotkey → app reads clipboard → synthesizes speech → plays audio. Queue system manages multiple items. Multi-backend support (Piper, Kokoro). Overlay window shows queue status when main window is closed.



### IPC Contract

**Event Channels** (backend → frontend, separate per concern):

| Channel | Purpose |
|---------|---------|
| `queue` | Queue state changes |
| `transcription` | Text→Speech pipeline |
| `playback` | Audio playback lifecycle |
| `model` | Model pool changes |
| `download` | Voice download progress |

**Commands** (frontend → backend via `invoke()`): listed per UI section below.

---

## App & UI

### Main Window

Configuration hub. Contains all settings and model management.

#### Voice Catalog

Browse available voices/models by language. Each entry shows:
- Quality score (1-5)
- Size (MB)
- Memory usage (MB)
- Speed (second to speek)
- Model type (Piper, Kokoro, ...)

Catalog is hardcoded metadata + download URLs. Downloaded items appear in "Installed Voices".

> **📡 Commands:** `fetch_voice_catalog`, `install_voice`, `uninstall_voice`
> **🔔 Events:** `download_progress`, `download_complete`, `voice_installed`, `voice_uninstalled`

#### Installed Voices

List of downloaded voice/model pairs. Each shows:
- Voice name + language
- Model type
- Status: active/inactive for language
- Action: set as active, uninstall, set as fallback

> **📡 Commands:** `list_installed_voices`, `set_active_voice`, `set_fallback_voice`, `uninstall_voice`
> **🔔 Events:** `voice_installed`, `voice_uninstalled`, `active_voice_changed`

#### Queue List

Same as Queue Overlay but embedded in main window. No frosted glass. Shows all queue items with controls.

> **📡 Commands:** `queue_add`, `queue_state`, `queue_remove`, `queue_move`, `queue_clear`
> **🔔 Events:** `queue_updated`, `playback_started`, `item_completed`, `error`, `processor_idle`

#### Hotkey Configuration

Record global hotkey for clipboard read + TTS. Store as text in `{app_data_dir}/hotkey.txt`.

> **📡 Commands:** `hotkey_get`, `hotkey_set`
> **🔔 Events:** (none)

### Tray Icon

When main window is closed, app minimizes to system tray. Tray menu:
- Show main window
- Show/hide overlay
- Quit

### Queue Overlay

Frosted glass window. Top-right corner. Only visible when queue has items with actionable status (Pending, Processing, ToPlay, Playing, Paused). Hides when only Played items remain or queue is empty. Shows:

#### Text Message items
- Text preview (truncated)
- Status: to transcribe, transcribing
- Controls: remove

#### Speech items
- Text preview
- Status: to play, playing, played
- Controls: play/pause, stop, restart, remove, download, reorder

#### Shared controls
- Auto-play toggle (process next item automatically)
- Clear all

> **📡 Commands:** `queue_add`, `queue_state`, `queue_remove`, `queue_move`, `queue_clear`, `playback_pause`, `playback_resume`, `playback_stop`
> **🔔 Events:** `queue_updated`, `playback_started`, `playback_paused`, `playback_resumed`, `playback_stopped`, `item_completed`, `error`, `processor_idle`

---

## Backend

### Queue System

Central data structure. Stores items of two kinds. Supports:
- Enqueue Text Message
- Replace Text Message → Speech (same ID, preserves position)
- Reorder items
- Remove items
- Get next Text Message (for Transcriber)
- Get next Speech (for SpeechPlayer)
- hold transcriber cursor (state: to transcribe, transcribing)
- hold speech player cursor (state: to play, playing, paused, played)

> **📡 Commands:** `queue_add`, `queue_remove`, `queue_move`, `queue_clear`, `queue_state`
> **🔔 Events:** `queue_updated`

#### Text Message

Simple text.

Lifecycle: `Pending → Processing → (replaced by Speech)`

#### Speech

- text
- audio
- voice/model used

### Transcriber

Consumer that converts Text Message → Speech.

- Dequeues next Text Message (state: to transcribe)
- Detects language (whatlang lib)
- Resolves active voice for language via VoicePreferences
- Loads model if not loaded
- Make model speak the text
- Replaces Text Message with Speech

The transcriber runs in it's own context concurrently to others.

> **📡 Commands:** (none — runs in background)
> **🔔 Events:** `transcription_started`, `transcription_completed`, `transcription_error`

### SpeechPlayer

Consumer that plays Speech items.

- Dequeues next Speech (state: to play)
- Plays audio (state: playing)
- Controls: play, pause, resume, stop, skip (state: paused, played)
- End audio (state: played)

If auto_read is enabled, player automatically processes next item.

The SpeechPlayer runs in it's own context concurrently to others.

> **📡 Commands:** `playback_pause`, `playback_resume`, `playback_stop`, `playback_skip`
> **🔔 Events:** `playback_started`, `playback_paused`, `playback_resumed`, `playback_stopped`, `item_completed`

---

## Models

### Voice Catalog

Hardcoded list of available voices with metadata. Per-engine catalog.
Merge each model catalog in one abstract layer.

> **📡 Commands:** `fetch_voice_catalog`, `install_voice`, `uninstall_voice`, `list_installed_voices`
> **🔔 Events:** `download_progress`, `download_complete`

#### Piper Catalog

- Source: HuggingFace API or hardcoded JSON
- Structure: each voice = one ONNX model file + config JSON
- Download: per-voice (no shared files)
- Storage: `{app_data_dir}/lisca/piper_models/`

#### Kokoro Catalog

- Source: hardcoded (limited set)
- Structure: one shared ONNX model + per-voice `.bin` style vectors + shared `tokenizer.json`
- Download: shared model (once) + per-voice `.bin` (each)
- Storage: `{app_data_dir}/lisca/kokoro/`
- Files: `model_q8f16.onnx` (86MB), `tokenizer.json` (3.5KB), `*.bin` (~522KB each)

#### Catalog Operations

- list available
- list_installed
- install
- uninstall

#### Installed Voice

Path to model/voice files.

### Voice Preferences

Per-language active voice selection.

- map of language/voice
- fallback (default voice if no match)

Persisted: `{app_data_dir}/lisca/voice_preferences.json`

Operations:
- get preferred
- set preferred for language

If active voice for detected language → use it
Else if fallback voice set → use it
Else → error (surface to user)

> **📡 Commands:** `get_voice_preference`, `set_voice_preference`
> **🔔 Events:** (none)

### Model 

Abstraction with following capabilities:
- synthesize

Implementations:
- `PiperModel`: Piper
- `KokoroModel`: Kokoro -- holds a reference to Kokoro Model, since kokoro model is the same for each voices

### Model Pool

Cache of loaded models.

For each type of model optionally holds a shared engine. 
Kokoro models in the pool will be given the shared engine, which actually contains the actual kokoto model.
Piper actual model will be in the pool and will be given an empty shared engine to maintain abstraction since they don't need a base shared model.

Capabilities:
- load (add a model into the pool and add potential shared engine)
- unload (remove a model from the pool and remove potential shared engine if there is no more model of this type in the pool)
- auto unload

Hold config
- Max cached: 4 (configurable)
- How long until auto unload (second or infinite)

> **📡 Commands:** `model_load`, `model_unload`
> **🔔 Events:** `model_loaded`, `model_unloaded`

## General Error handling

No silent failures, log + UI notif.

## General Monitoring

Different levels of logs to follow step by step whats happening when needed.

##