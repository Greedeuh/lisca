# TTS Backend

## Feature
A Piper ONNX-based TTS engine that synthesizes text to speech using local neural models, with automatic language-based routing.

## Scenarios

### Piper Backend
- **As a user**, I can download Piper voice models from a catalog of 20+ bundled espeak-ng languages, so I can get TTS in French, German, Spanish, and many more.
- **As a user**, I can see download progress as a bar with byte counts, so I know how long the download will take.
- **As a user**, I can delete installed models I no longer need.
- **As a user**, the backend loads automatically on app startup from my saved config, so I don't have to re-select it each time.
- **As a user**, if the selected model files are missing from disk, the app falls back gracefully with a log message instead of crashing.
- **As a user**, I can open the resource folder in my OS file manager to inspect or manage model files directly.

### Language Routing
- **As a user**, when a queue item is processed, the system automatically loads the correct voice model for its detected language.
- **As a user**, I can configure which installed voice to use per language family (e.g. French → fr_FR-siwis), so I have fine-grained control over voice selection.
- **As a user**, I can set a fallback voice for languages without a specific mapping.
- **As a user**, if a language isn't configured in the mapping but has an installed model, the system automatically uses that model.
- **As a user**, installed models are cached with LRU eviction (max 4), so switching between languages is fast without excessive memory use.

## Key Files
- `src-tauri/src/tts/mod.rs` — TtsManager, BackendPool, backend trait
- `src-tauri/src/tts/piper.rs` — Piper ONNX inference
- `src-tauri/src/tts/piper_models.rs` — Voice catalog & downloads
- `src-tauri/src/tts/session.rs` — ONNX session with XNNPACK/CPU fallback
- `src-tauri/src/tts/config.rs` — BackendConfig persistence
- `src-tauri/src/tts/language.rs` — language detection via whatlang
- `src-tauri/src/tts/voice_mapping.rs` — language-to-voice resolution config
- `src/components/ModelConfig.tsx` — frontend model settings
- `src/components/VoiceMappingSettings.tsx` — language routing config UI
