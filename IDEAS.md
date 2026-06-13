# IDEAS.md

Feature ideas for Lisca — Text to Speech app.

## Core Experience

- **Auto-copy on hotkey**: Simulate Ctrl+C before reading clipboard, so user just selects text + presses hotkey
- **Selection reading**: Use accessibility APIs to read selected text directly without clipboard (no Ctrl+C needed)
- **Stop on re-press**: Press hotkey again to interrupt current speech

## Queue System

- **Refine**

## Voice & Model

- **Speed control**: Adjustable speech rate slider
- **Multi-binding**: Multiple hotkeys mapped to different languages/models — e.g. Ctrl+Shift+1 for English, Ctrl+Shift+2 for French
- **Auto-detect language**: Use text heuristics (character ranges, common words) or a lightweight classifier to pick the right voice automatically

## Audio

- **Audio device selection**: Choose output device (speakers, headphones)
- **Volume control**: Per-app volume slider

## Settings

- **Auto-start**: Launch on system login

## UI/UX

- **History**: List of recently spoken texts
- **Dark/light theme**: Follow system preference

## Integration

- **CLI mode**: `lisca speak "text"` for scripting
- **API server**: Local HTTP API for other apps to trigger TTS

## Performance

- **Idle unload**: Unload model after N seconds of inactivity to free memory (like Handy's idle watcher)
- **GPU acceleration**: Enable CUDA/DirectML execution providers for faster inference
