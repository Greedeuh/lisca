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

## Frontend

- React 18 + TypeScript + Vite

## Architecture

Tauri process model: Rust core process + WebView. Frontend communicates with backend via `invoke()` (IPC). 

## Testing

- `.claude/skills/lisca-testing/SKILL.md` — what to test, what to skip, patterns
- `docs/testing-strategy.md` — current strategy (Layer 1 + Layer 3a)
- `docs/testing-future.md` — deferred layers (Layer 2, 3b, 3c, 4)