# TTS Queue

## Feature
A persistent, ordered queue of text items to be spoken sequentially, with full playback controls.

## Scenarios

### Queue Management
- **As a user**, I can add text to the queue by pressing the global hotkey (clipboard text is enqueued), so I can queue up multiple items to hear.
- **As a user**, I can see all items in the queue displayed in order, so I know what's coming up.
- **As a user**, I can see which item is currently playing, highlighted separately from the pending items.
- **As a user**, I can remove a single item from the queue by clicking its remove button.
- **As a user**, I can reorder items in the queue by dragging them, so I can prioritize what I want to hear first.
- **As a user**, I can clear the entire queue at once, which also stops any current playback.
- **As a user**, the queue persists across app restarts, so I don't lose queued items when I close and reopen the app.

### Playback Controls
- **As a user**, I can pause playback and resume it from where it left off.
- **As a user**, I can stop playback entirely, which skips the current item.
- **As a user**, I can toggle "Auto-read" on or off — when on, items play automatically in sequence; when off, only the current item plays and then stops.
- **As a user**, when auto-read is off and the current item finishes, playback stops and the overlay hides (if the main window is hidden).

### Queue Limits
- **As a user**, there is a maximum queue size (default 50), and I get an error message if I try to add beyond the limit.

### Error Handling
- **As a user**, if an item fails to synthesize (e.g. backend error), it is removed from the queue and the next item is attempted automatically, so one bad item doesn't block the whole queue.

## Key Files
- `src-tauri/src/tts/queue.rs` — data types, file persistence
- `src-tauri/src/tts/processor.rs` — async playback loop
- `src-tauri/src/tts/mod.rs` — queue management methods
- `src/hooks/useTtsQueue.ts` — frontend state hook
- `src/components/TtsQueue.tsx` — main queue UI
- `src/components/QueueList.tsx` — queue item list
- `src/components/QueueControls.tsx` — playback controls
