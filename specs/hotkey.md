# Global Hotkey

## Feature
A system-wide keyboard shortcut that reads clipboard text and queues it for speech.

## Scenarios

- **As a user**, I can record a custom global hotkey by pressing a key combination in the settings, so I can trigger TTS from any application.
- **As a user**, I can see my currently registered hotkey displayed in the UI, so I know which shortcut to press.
- **As a user**, when I press the global hotkey anywhere in the OS, the clipboard text is read and added to the TTS queue, so I hear it spoken aloud.
- **As a user**, if my clipboard is empty when I press the hotkey, nothing happens silently, so I'm not confused by error messages.
- **As a user**, my hotkey setting persists across app restarts, so I don't have to reconfigure it every time.
- **As a user**, I can change my hotkey at any time — the old one is unregistered and the new one takes effect immediately.

## Key Files
- `src-tauri/src/hotkey.rs` — parse, register, persist, clipboard read
- `src/components/HotkeyRecorder.tsx` — key capture UI
- `src-tauri/capabilities/default.json` — global-shortcut permissions
