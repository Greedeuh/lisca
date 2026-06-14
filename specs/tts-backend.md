# TTS Backends

## Feature
Two switchable ONNX-based TTS engines (Kokoro and Piper) that synthesize text to speech using local neural models, with automatic language-based routing.

## Scenarios

### Backend Selection
- **As a user**, I can choose between two TTS backends (Piper or Kokoro) from a dropdown in the Model settings, so I can pick the one that suits my language and quality needs.
- **As a user**, when I switch backends, the previous backend is stopped, the new one is loaded, and the config is saved — all in one action, so I see immediate results.
- **As a user**, the backend loads automatically on app startup from my saved config, so I don't have to re-select it each time.
- **As a user**, if the selected model files are missing from disk, the app falls back gracefully with a log message instead of crashing.
- **As a user**, I can manually enter model and config file paths in the Advanced section, so I can use custom or locally placed models.
- **As a user**, I can open the resource folder in my OS file manager to inspect or manage model files directly.

### Language Routing
- **As a user**, when a queue item is processed, the system automatically loads the correct voice model for its detected language.
- **As a user**, I can configure which installed voice to use per language family (e.g. French → fr_FR-siwis), so I have fine-grained control over voice selection.
- **As a user**, I can set a fallback voice for languages without a specific mapping.
- **As a user**, if a language isn't configured in the mapping but has an installed model, the system automatically uses that model.
- **As a user**, installed models are cached with LRU eviction (max 4), so switching between languages is fast without excessive memory use.

### Piper Backend
- **As a user**, I can download Piper voice models from a catalog of 20+ bundled espeak-ng languages, so I can get TTS in French, German, Spanish, and many more.
- **As a user**, I can see download progress as a bar with byte counts, so I know how long the download will take.
- **As a user**, I can select which installed voice model to use, and the backend reloads immediately.
- **As a user**, I can delete installed models I no longer need.

### Kokoro Backend
- **As a user**, I can configure Kokoro with a `.onnx` model file and a `.bin` voice style file, so I can use the Kokoro TTS engine.

## Key Files
- `src-tauri/src/tts/mod.rs` — TtsManager, BackendPool, backend trait, switching
- `src-tauri/src/tts/kokoro.rs` — Kokoro ONNX inference
- `src-tauri/src/tts/piper.rs` — Piper ONNX inference
- `src-tauri/src/tts/session.rs` — ONNX session with XNNPACK/CPU fallback
- `src-tauri/src/tts/config.rs` — BackendConfig persistence
- `src-tauri/src/tts/language.rs` — language detection via whatlang
- `src-tauri/src/tts/voice_mapping.rs` — language-to-voice resolution config
- `src/components/ModelConfig.tsx` — frontend backend selector
- `src/components/VoiceMappingSettings.tsx` — language routing config UI
