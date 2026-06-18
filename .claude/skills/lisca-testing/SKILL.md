---
name: lisca-testing
description: Guides testing the Lisca TTS app — what to test, what to skip, how to write tests, and project-specific patterns.
---

# Testing Lisca

## Quick Reference

```bash
cargo test              # Rust unit tests (from src-tauri/)
bun run test            # Frontend tests (vitest)
bun run build           # Type check + vite build
```

## Principles

- **User-first**: test what users care about — features work, data persists, no crashes
- **Confidence over coverage**: cover critical paths, don't chase 100% line coverage
- **No implementation testing**: test behavior, not internals
- **Mock the heavy stuff**: ONNX models and audio playback are always mocked
- **Regression first**: when a bug is found, write a failing test *before* fixing it

## What to Test

### Layer 1: Rust Unit Tests

Pure functions — no Tauri runtime, no models, no network.

**Pattern**: inline `#[cfg(test)] mod tests` in each file. Use `tempfile::tempdir()` for file tests.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_text_multiple_sentences() {
        assert_eq!(
            split_text("Hello. World?"),
            vec!["Hello.", "World?"]
        );
    }
}
```

### Layer 3a: Pure Frontend Component Tests

Presentational components — render correctly, call callbacks on interaction.

**Pattern**: Vitest + `@testing-library/react` + `@testing-library/user-event`.

### Testing Hooks

Use `renderHook` from `@testing-library/react`. Mock IPC with `mockIPC` from `@tauri-apps/api/mocks`. Simulate events with `emit` from `@tauri-apps/api/event`.

## What NOT to Test

- **rodio audio playback** — platform-dependent, needs audio device
- **ONNX inference correctness** — upstream dependency, not our code
- **Overlay window positioning** — platform-specific Win32/X11 calls
- **espeak-ng phonemization accuracy** — upstream
- **Tauri plugin internals** — global shortcut registration, clipboard access
- **Catalog fetch/download** — hits HuggingFace network
- **CSS/styling** — no visual regression for a small app

## Tech Details

### Vitest Config

`vitest.config.ts` at project root — separate from `vite.config.ts` (which has multi-page build + dev server config). Uses `jsdom` environment + `@vitejs/plugin-react`.

### Setup File (`src/test/setup.ts`)

Three things happen before each test:
1. `@testing-library/jest-dom` matchers loaded (`.toBeInTheDocument()`, etc.)
2. `crypto.getRandomValues` polyfilled — jsdom doesn't have it, `@tauri-apps/api/mocks` needs it
3. `window.__TAURI_INTERNALS__` mocked with `vi.fn()` — components call `invoke()` which reads this

### TypeScript Config

`tsconfig.json` excludes `src/test` and `src/**/__tests__` from the build (test files use vitest globals not available in production tsc). Vitest resolves its own types.

### Rust Dev Dependency

`tempfile = "3"` in `src-tauri/Cargo.toml` `[dev-dependencies]` — used for file-based tests

### Mocking `@tauri-apps/api/event` `listen`

Components that call `listen()` directly (like `QueueOverlay`) need `__TAURI_INTERNALS__.transformCallback`. Mock the module before importing the component:

```ts
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));
```

### Mocking hooks with `vi.mock`

```ts
vi.mock("../../hooks/useTtsQueue", () => ({
  useTtsQueue: vi.fn(),
}));
import { useTtsQueue } from "../../hooks/useTtsQueue";
const mockUseTtsQueue = vi.mocked(useTtsQueue);

// In each test:
mockUseTtsQueue.mockReturnValue({ items: [], current: null, playback: "idle", ... });
```

### Adding a New Test

**Rust**: Add a `#[cfg(test)] mod tests { ... }` block at the bottom of the source file. Import `super::*`. Use `tempfile::tempdir()` for any file I/O.

**Frontend**: Create `src/components/__tests__/ComponentName.test.tsx`. Import `render`, `screen`, `userEvent` from testing libraries. Mock IPC with `mockIPC` if the component calls `invoke()`.

## Adding New Tests Checklist

1. What does the user see/do? (start from user behavior)
2. Is this pure logic (Rust unit test) or UI (frontend test)?
3. Does it need Tauri runtime? If yes, defer or mock at `invoke()` level
4. Does it need an ONNX model? If yes, create a `MockBackend` or test without backend
5. Does it hit the network? If yes, skip or mock HTTP
6. Write the test first if it's a bug regression
