# IDEAS.md

Feature ideas for Lisca — Text to Speech app.

## Core Experience

- **Auto-copy on hotkey**: Simulate Ctrl+C before reading clipboard, so user just selects text + presses hotkey
- **Selection reading**: Use accessibility APIs to read selected text directly without clipboard (no Ctrl+C needed)
- **Stop on re-press**: Press hotkey again to interrupt current speech

## Voice & Model

- **Voice picker UI**: List available Kokoro voices (af_*, am_*, bf_*, bm_*) with preview
- **Multiple voice support**: Switch between voices at runtime
- **Speed control**: Adjustable speech rate slider
- **Model download**: Auto-download model from HuggingFace on first run
- **Multi-language**: Load language-specific models

## Audio

- **Audio device selection**: Choose output device (speakers, headphones)
- **Volume control**: Per-app volume slider

## Settings

- **Auto-start**: Launch on system login
- **Language selection**: Support en-us, en-gb, and other languages

## UI/UX

- **System tray**: Minimize to tray, right-click menu
- **Overlay**: Minimal floating indicator when speaking
- **History**: List of recently spoken texts
- **Dark/light theme**: Follow system preference

## Integration

- **CLI mode**: `lisca speak "text"` for scripting
- **API server**: Local HTTP API for other apps to trigger TTS

## Performance

- **Idle unload**: Unload model after N seconds of inactivity to free memory (like Handy's idle watcher)
- **GPU acceleration**: Enable CUDA/DirectML execution providers for faster inference