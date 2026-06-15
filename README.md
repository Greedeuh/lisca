# Lisca

> **⚠️ Work in Progress** — Lisca is in early development. APIs, config formats, and behavior may change without notice. Not recommended for daily use yet. (Test project for TTS models, ONNX rust libs, tauri workflow & MiMo Code with MiMo2.5(+-pro))

A desktop text-to-speech app. Select text anywhere, press a hotkey, and Lisca reads it aloud.

Built with [Tauri v2](https://v2.tauri.app/) — React/TypeScript frontend, Rust backend.

## Features

- **Global hotkey** — trigger TTS from any app via a customizable keyboard shortcut
- **Piper TTS engine** — ONNX-based TTS with espeak-ng phonemization, 20+ bundled languages
- **Voice catalog** — browse, search, and download Piper voices from HuggingFace
- **Playback queue** — queue multiple texts, reorder, pause/resume, auto-read mode
- **Floating overlay** — frosted-glass queue window that stays on top
- **System tray** — hides to tray on close, left-click to show
- **Persistent config** — hotkey, model selection, and queue survive restarts

## Screenshots

<!-- Add screenshots here -->

## Prerequisites

- [Bun](https://bun.sh/) (package manager)
- [Rust](https://rustup.rs/) (stable toolchain)
- System dependencies for Tauri v2 — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Getting Started

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev

# Build for production
bun run tauri build
```

## Development

```bash
bun run dev          # Frontend only (Vite, port 1420)
bun run tauri dev    # Full app (Rust + frontend)
bun run build        # Type check + Vite build
```

## Testing

```bash
cargo test                    # Rust unit tests (from src-tauri/)
bun run test                  # Frontend tests (vitest)
```

See [docs/testing-strategy.md](docs/testing-strategy.md) for the testing approach.

## Project Structure

```
├── src/                       # React frontend
│   ├── components/            # UI components
│   ├── hooks/                 # useTtsQueue, usePiperModels
│   ├── overlay/               # Floating queue overlay window
│   ├── types/                 # TypeScript type definitions
│   └── utils/                 # Helpers (formatSize)
├── src-tauri/                 # Rust backend
│   └── src/
│       ├── lib.rs             # Tauri setup, command registration
│       ├── hotkey.rs          # Global hotkey management
│       ├── overlay.rs         # Overlay window positioning
│       ├── persist.rs         # JSON file persistence
│       └── tts/               # TTS engine
│           ├── mod.rs         # TtsManager, queue, playback
│           ├── piper.rs       # Piper ONNX model
│           ├── piper_models.rs # Voice catalog & downloads
│           ├── processor.rs   # Async playback loop
│           ├── config.rs      # Backend config persistence
│           ├── queue.rs       # Queue types & persistence
│           ├── text.rs        # Sentence splitting
│           └── session.rs     # ONNX session builder
├── specs/                     # Feature specifications
└── docs/                      # Testing strategy docs
```

## How It Works

1. User selects text in any app
2. Presses the global hotkey (default: configurable)
3. Lisca reads the clipboard text
4. Text is queued for synthesis
5. ONNX model (Piper) generates audio
6. rodio plays the audio through system output

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | [Tauri v2](https://v2.tauri.app/) |
| Frontend | React 18, TypeScript, Vite |
| Backend | Rust (edition 2021) |
| TTS | ONNX Runtime, espeak-ng |
| Audio | rodio |
| Testing | cargo test, Vitest, Testing Library |

## Configuration

Config files are stored at `{app_data_dir}/lisca/`:

- `hotkey.txt` — saved hotkey string
- `config.json` — Piper model paths
- `queue.json` — persistent playback queue
- `queue_config.json` — queue settings (max items, auto-read, overlay)
- `piper_voices_cache.json` — cached voice catalog

## Known Limitations

- **Linux (Wayland):** The floating overlay window does not work on Wayland due to its restrictive window management. X11 is supported. This is a Tauri/WebkitGTK limitation.

## License

<!-- Add license here -->
