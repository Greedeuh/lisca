# IDEAS.md

Feature ideas for Lisca — Text to Speech app.

## Core Experience

- **Auto-copy on hotkey**: Simulate Ctrl+C before reading clipboard, so user just selects text + presses hotkey
- **Push-to-talk mode**: Hold hotkey while speaking, release to trigger TTS (like Handy's STT)
- **Selection reading**: Use accessibility APIs to read selected text directly without clipboard (no Ctrl+C needed)
- **Stop on re-press**: Press hotkey again to interrupt current speech

## Voice & Model

- **Voice picker UI**: List available Kokoro voices (af_*, am_*, bf_*, bm_*) with preview
- **Multiple voice support**: Switch between voices at runtime
- **Speed control**: Adjustable speech rate slider
- **Model download**: Auto-download model from HuggingFace on first run
- **Model format support**: Support multiple quantization levels (fp32, fp16, q8, q4)

## Audio

- **Audio device selection**: Choose output device (speakers, headphones)
- **Volume control**: Per-app volume slider
- **Audio feedback**: Beep/sound when hotkey triggers (like Handy)
- **Streaming playback**: Start playing before full synthesis completes

## Settings

- **Persistence**: Save all settings to disk (hotkey, voice, speed, model path)
- **Settings UI**: Dedicated settings window/panel
- **Auto-start**: Launch on system login
- **Language selection**: Support en-us, en-gb, and other languages

## UI/UX

- **System tray**: Minimize to tray, right-click menu
- **Overlay**: Minimal floating indicator when speaking
- **History**: List of recently spoken texts
- **Keyboard shortcuts**: Additional shortcuts (stop, previous, next voice)
- **Dark/light theme**: Follow system preference

## Integration

- **CLI mode**: `lisca speak "text"` for scripting
- **Clipboard manager**: Option to speak clipboard history items
- **Browser extension**: Send text from browser to Lisca
- **API server**: Local HTTP API for other apps to trigger TTS

## Advanced

- **SSML support**: Prosody tags for fine-grained control
- **Multi-language**: Load language-specific models
- **Custom voices**: Voice cloning with reference audio
- **Offline mode**: Full offline operation with bundled models
- **Performance**: GPU acceleration, model caching, preloading
