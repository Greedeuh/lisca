# Lisca Testing — Future Considerations

These layers are deferred until the codebase stabilizes. The project is young and actively refactoring. Tests written against unstable code will be deleted or rewritten within days.

---

## Layer 2: Rust Integration Tests (Tauri Commands)

**Status**: Deferred — `tauri::test` is unstable, setup cost is high, code is still changing.

**When to revisit**: After `TtsManager`'s field count stabilizes and command signatures settle (2-4 weeks).

**What it would cover**:

| Command | What to verify |
|---|---|
| `hotkey_set` / `hotkey_get` | Set hotkey → get it back. Invalid shortcut returns error |
| `tts_get_config` / `tts_set_config` | Get default config, set new backend config, verify persistence |
| `tts_queue_add/remove/move/clear` | Queue CRUD operations through IPC |
| `tts_queue_state` | Returns correct snapshot |
| `tts_pause` / `tts_resume` / `tts_stop` | Playback state transitions. Requires mock backend |
| `piper_list_installed` | Returns installed models from temp dir |

**How it would work**:

> **Risk**: `tauri::test` is marked **unstable** — API may break across minor Tauri versions.

Requires constructing `InvokeRequest` objects:

```rust
use tauri::test::{mock_builder, assert_ipc_response, INVOKE_KEY};

fn invoke_request(cmd: &str, args: serde_json::Value) -> tauri::webview::InvokeRequest {
    tauri::webview::InvokeRequest {
        cmd: cmd.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::from_json(&args).unwrap(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}
```

Mock backend for state-transition tests:

```rust
struct MockBackend;
impl TtsBackend for MockBackend {
    fn synthesize(&mut self, _: &str, _: f32) -> Result<Vec<f32>, String> {
        Ok(vec![0.0; 24000]) // 1s silence
    }
    fn sample_rate(&self) -> u32 { 24000 }
}
```

**Dev-dependencies**: Merge `test` feature into existing tauri dep — `tauri = { version = "2", features = ["tray-icon", "test"] }`.

**Spike test**: Before committing to full Layer 2, write one command (e.g. `tts_queue_add`) end-to-end. If the spike takes >2 hours, the full layer isn't worth it yet.

**What to skip**: `tts_speak` (needs audio device), overlay positioning (platform-specific), global shortcut registration (plugin internals), `piper_fetch_voices`/`piper_download_model` (network).

---

## Layer 3b: Stateful Frontend Components

**Status**: Deferred — requires mocking `@tauri-apps/api/mocks` and testing async state transitions.

**When to revisit**: After the 3 main components (`HotkeyRecorder`, `ModelConfig`, `TtsQueue`) stabilize.

| Component | Test cases |
|---|---|
| `HotkeyRecorder` | Mount → loads saved hotkey. Record mode → keydown → saves combo |
| `ModelConfig` | Mount → loads config. Backend switch → saves. Path inputs |
| `VoiceBrowser` | Search filters. Family expand/collapse |
| `TtsQueue` | Wires hook data to controls + list |
| `PiperModelPicker` | Mount → fetches catalog. Shows installed/available tabs. Download triggers progress |

---

## Layer 3c: Hook Tests

**Status**: Deferred — higher complexity, lower immediate value.

| Hook | Test cases |
|---|---|
| `useTtsQueue` | Mount loads state. Event dispatch updates. `add()`/`remove()`/`clear()` invoke correct commands. `toggleAutoRead()` flips config |
| `usePiperModels` | Mount loads installed. `fetchCatalog()` invokes. Download progress events update state |

**Pattern** (`renderHook` + event simulation):
```ts
import { renderHook, act } from '@testing-library/react';
import { emit } from '@tauri-apps/api/event';

const { result } = renderHook(() => useTtsQueue());
await act(async () => {
  await emit('tts-queue-event', { type: 'queue_updated', items: [{ id: 1, text: 'hi' }], ... });
});
```

---

## Layer 4: E2E Tests

**Status**: Deferred until release pipeline exists.

**Tooling**: `tauri-driver` + WebdriverIO. Linux needs `webkit2gtk-driver` + `xvfb`. macOS not supported.

**Top user journeys**:
1. Record hotkey → restart → hotkey persists
2. Add text to queue → playback starts → completes
3. Browse catalog → download voice → select → speak

---

## Layer 2 — Full File Organization (when ready)

```
src-tauri/
├── tests/
│   ├── common/mod.rs                ← MockBackend, setup helpers
│   └── commands.rs                  ← Tauri command integration tests
└── Cargo.toml                       ← [dev-dependencies: tauri "test" feature, tempfile]
```

## Layer 3b/3c — Full File Organization (when ready)

```
src/
├── components/__tests__/
│   ├── HotkeyRecorder.test.tsx
│   ├── ModelConfig.test.tsx
│   ├── VoiceBrowser.test.tsx
│   ├── TtsQueue.test.tsx
│   └── PiperModelPicker.test.tsx
└── hooks/__tests__/
    ├── useTtsQueue.test.ts
    └── usePiperModels.test.ts
```
