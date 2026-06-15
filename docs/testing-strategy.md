# Lisca Testing Strategy

## Principles

- **User-first**: test what users care about (features work, data persists, no crashes)
- **Confidence over coverage**: cover the critical paths, don't chase 100% line coverage
- **No implementation testing**: test behavior, not internals
- **Mock the heavy stuff**: ONNX models and audio playback are always mocked
- **Simple and scalable**: flat structure, standard tools, easy to add new tests

## Before tests: fix real crash risks

The `.unwrap()` calls in `src-tauri/src/` are panic points. Fix the hot paths first — they're a bigger risk than missing test coverage.

```bash
cargo clippy -- -W clippy::unwrap_used
```

Replace with `?` or `.unwrap_or_else()` in file I/O, model loading, and IPC handlers.

## Before tests: add CI

Tests without CI don't exist. Minimum viable GitHub Actions:

```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libasound2-dev
      - run: cargo test --lib
        working-directory: src-tauri
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
      - run: bun install
      - run: bun run build
      - run: bun run vitest run
```

Set this up *before* writing tests, so every test is immediately enforced.

---

## Layer 1: Rust Unit Tests

**What**: Pure logic functions — no Tauri runtime, no models, no network.

Examples:

| Function | File | Key cases |
|---|---|---|
| `parse_shortcut()` | `hotkey.rs` | Valid combos (`"Control+Shift+K"`, `"Alt+A"`, `"Super+F1"`), invalid (`""`, `"Control"` only, unknown key) |
| `QueueConfig` defaults + serde | `tts/queue.rs` | Default values, JSON round-trip |
| `QueueEvent` serde (7 variants) | `tts/queue.rs` | Each variant serializes with correct `"type"` tag and `snake_case` naming |
| Queue file persistence | `tts/queue.rs` | `save_queue`/`load_queue` and `save_queue_config`/`load_queue_config` with temp dir |

**How**: Standard `#[cfg(test)]` modules inline in each file. Use `tempfile::tempdir()` for file-based tests.

**New dev-dependency**: `tempfile = "3"` in `src-tauri/Cargo.toml`.

**Run**: `cargo test` from `src-tauri/`.

---

## Layer 3a: Pure Frontend Component Tests

**What**: Presentational components render correctly and call callbacks on interaction.

**Tooling**:
```bash
bun add -D vitest @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom
```

**Config**: `vitest.config.ts` with `environment: 'jsdom'` + `@vitejs/plugin-react`.

**Mock pattern** (`src/test/setup.ts`):
```ts
import { mockIPC, mockWindows, clearMocks } from '@tauri-apps/api/mocks';
beforeEach(() => { clearMocks(); mockWindows('main'); });
```


Examples:
| Component | Test cases |
|---|---|
| `VoiceRow` | Renders name/quality/size. "Download" button for uninstalled voices. Click handlers called |
| `InstalledModels` | Lists models, "Active" badge, delete callback |
| `DownloadProgress` | Progress bar width, percentage text |
| `QueueControls` | Pause/Resume toggle, stop/clear callbacks, checkbox state |
| `QueueList` | Now-playing item, queued items, reorder/remove buttons |

**Run**: `bun run vitest`.

---

## What NOT to Test

- **rodio audio playback** — platform-dependent, needs audio device
- **ONNX inference correctness** — upstream dependency, not our code
- **Overlay window positioning** — platform-specific Win32/X11 calls
- **espeak-ng phonemization accuracy** — upstream
- **Tauri plugin internals** — global shortcut, clipboard
- **Piper catalog fetch/download** — hits HuggingFace network
- **CSS/styling** — no visual regression for a small app

---

## Regression Strategy

When a bug is found:
1. Write a failing test that reproduces the bug *before* fixing it
2. Fix the bug
3. Verify the test passes
4. The test stays — it prevents regression

This applies to both Rust (Layer 1) and frontend (Layer 3a) bugs.

## Verification

- `cargo test` from `src-tauri/` — all Rust tests pass
- `bun run vitest run` — all frontend tests pass
- `bun run build` — type check still passes
