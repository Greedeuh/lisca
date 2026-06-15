# Lisca — Feature Specifications

Text-to-speech desktop app built with Tauri v2 (React/TypeScript frontend, Rust backend).

## Features

| Spec | Description |
|------|-------------|
| [Hotkey](hotkey.md) | Global keyboard shortcut to enqueue clipboard text for speech |
| [TTS Backend](tts-backend.md) | Piper ONNX engine with language-based routing |
| [Voice Catalog](voice-catalog.md) | Browse, search, download, and manage Piper voice models |
| [Queue](queue.md) | Persistent TTS playback queue with controls |
| [Overlay](overlay.md) | Floating transparent queue window (frosted glass) |
| [System Tray](system-tray.md) | Tray icon with show/quit menu, close-to-tray |
| [Settings](settings.md) | Config persistence and startup restoration |

## Architecture

- **Process model**: Rust core process + WebView (Tauri v2)
- **IPC**: Frontend calls Rust via `invoke()`, backend emits events via `app.emit()`
- **Audio**: ONNX inference → rodio playback (mono, f32→i16 conversion)
- **Persistence**: JSON files in `{app_data_dir}/lisca/`
