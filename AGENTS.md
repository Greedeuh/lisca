# AGENTS.md

## Project

Lisca — a Tauri v2 desktop app for text-to-speech. React/TypeScript frontend, Rust backend.

## Quick commands

- **Dev (full app):** `bun run tauri dev`
- **Frontend only (Vite):** `bun run dev` (port 1420)
- **Build:** `bun run tauri build`
- **Type check:** `bun run build` (runs `tsc && vite build`)
- **Rust tests:** `cargo test` (from `src-tauri/`)
- **Frontend tests:** `bun run test` (vitest)
- **CI:** `.github/workflows/test.yml` — runs `cargo test`, `bun run build`, `bun run vitest run`

## Package manager

Uses **bun**, not npm or yarn. The lockfile is `bun.lock`.

## Rust backend

- `src-tauri/src/lib.rs` — Tauri setup and command registration
- `src-tauri/src/main.rs` — entrypoint, calls `lisca_lib::run()`
- `src-tauri/src/hotkey.rs` — global hotkey register/save/load, clipboard read
- `src-tauri/src/tts/mod.rs` — TtsManager: speak, stop, preload (Piper ORT + rodio playback)
- `src-tauri/src/tts/session.rs` — ONNX session creation with XNNPACK or CPU fallback
- `time` crate pinned to `=0.3.47` due to Tauri upstream conflict (see Cargo.toml TODO)

## Frontend

- React 18 + TypeScript + Vite
- Entry: `src/main.tsx` → `src/App.tsx`
- Components: `HotkeyRecorder`, `ModelConfig`, `PiperModelPicker`, `VoiceBrowser`, `VoiceRow`, `InstalledModels`, `DownloadProgress`, `TtsQueue`, `QueueControls`, `QueueList`
- Overlay: `src/overlay/QueueOverlay.tsx` (separate window)
- Hooks: `useTtsQueue` (queue state + events), `usePiperModels` (catalog/download state)
- Uses `@tauri-apps/api/core` `invoke()` to call Rust commands
- `@tauri-apps/plugin-global-shortcut` registered

## Architecture

Tauri process model: Rust core process + WebView. Frontend communicates with backend via `invoke()` (IPC). Hotkey config stored as plain text at `{app_data_dir}/lisca/hotkey.txt`. Global hotkey uses `tauri-plugin-global-shortcut`.

Core flow: user selects text → presses hotkey → Rust reads clipboard → Piper synthesizes audio → rodio plays it.

## Key config

- `tauri.conf.json` — `devUrl: http://localhost:1420`, `beforeDevCommand: "bun run dev"`
- `src-tauri/capabilities/default.json` — permissions: `core:default`, `global-shortcut:allow-register`, `global-shortcut:allow-unregister`
- CSP is disabled (`"cstrl": null`)

## Testing

- `.claude/skills/lisca-testing/SKILL.md` — what to test, what to skip, patterns
- `docs/testing-strategy.md` — current strategy (Layer 1 + Layer 3a)
- `docs/testing-future.md` — deferred layers (Layer 2, 3b, 3c, 4)

## Tauri gotcha

`generate_handler![module::function]` registers the command as `"function"` (not `"module::function"`). Use prefixed names (`hotkey_set`, `tts_speak`) to avoid conflicts.
