# IDEAS.md

Feature ideas for Lisca — Text to Speech app.

## Core Experience

- **Stop on re-press**: Press hotkey again to interrupt current speech

## Queue System

- **Refine**

## Voice & Model

- **Speed control**: Adjustable speech rate slider
- **Multi-binding**: Multiple hotkeys mapped to different languages/models — e.g. Ctrl+Shift+1 for English, Ctrl+Shift+2 for French
- **Auto-detect language**: Use text heuristics (character ranges, common words) or a lightweight classifier to pick the right voice automatically
- **Auto-translate to voice language**: If selected text is in a different language than the active voice, translate it before synthesis (e.g. German text + English voice → translate to English first)

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
