# AGENTS.md

## Project

Lisca — a Tauri v2 desktop app for text-to-speech. React/TypeScript frontend, Rust backend.

## Quick commands

- **Dev (full app):** `bun run tauri dev`
- **Frontend only (Vite):** `bun run dev` (port 1420)
- **Build:** `bun run tauri build`
- **Type check:** `bun run build` (runs `tsc && vite build`)

No lint, test, or CI pipelines are configured.

## Package manager

Uses **bun**, not npm or yarn. The lockfile is `bun.lock`.

## Rust backend

- `src-tauri/src/lib.rs` — all Tauri commands defined here (save_hotkey, load_hotkey, register_hotkey, unregister_hotkey, greet)
- `src-tauri/src/main.rs` — entrypoint, just calls `lisca_lib::run()`
- `time` crate pinned to `=0.3.47` due to Tauri upstream conflict (see Cargo.toml TODO)

## Frontend

- React 18 + TypeScript + Vite
- Entry: `src/main.tsx` → `src/App.tsx`
- Uses `@tauri-apps/api/core` `invoke()` to call Rust commands
- `@tauri-apps/plugin-opener` and `@tauri-apps/plugin-global-shortcut` plugins registered

## Architecture

Tauri process model: Rust core process + WebView. Frontend communicates with backend via `invoke()` (IPC). Settings stored at `{app_data_dir}/lisca/settings.json`. Global hotkey uses `tauri-plugin-global-shortcut`.

## Key config

- `tauri.conf.json` — `devUrl: http://localhost:1420`, `beforeDevCommand: "bun run dev"`
- `src-tauri/capabilities/default.json` — permissions: `core:default`, `opener:default`, `global-shortcut:allow-register`, `global-shortcut:allow-unregister`, `clipboard-manager:allow-read-text`
- CSP is disabled (`"csp": null`)

## Tauri gotcha

`generate_handler![module::function]` registers the command as `"function"` (not `"module::function"`). Use prefixed names (`hotkey_set`, `tts_speak`) to avoid conflicts.
