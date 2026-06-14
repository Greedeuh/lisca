# Floating Overlay

## Feature
A transparent, always-on-top floating window that shows the queue status and playback controls when the main window is hidden.

## Scenarios

- **As a user**, when I close the main window, a floating overlay appears showing my queue — if the "Show overlay" setting is enabled and the queue is not empty.
- **As a user**, the overlay shows the currently playing item with play/pause and skip buttons, so I can control playback without opening the main window.
- **As a user**, the overlay shows pending queue items with remove buttons, so I can manage the queue at a glance.
- **As a user**, I can drag the overlay by its header to reposition it anywhere on screen.
- **As a user**, I can toggle "Auto-read" from the overlay header.
- **As a user**, I can close the overlay with the ✕ button — this also disables the "Show overlay" setting.
- **As a user**, I can toggle the "Show overlay" setting from the main window's Queue controls.
- **As a user**, the overlay has a frosted glass appearance with rounded corners, so it looks polished and doesn't obstruct my desktop.
- **As a user**, on Windows the overlay stays on top of all other windows even when switching apps.
- **As a user**, on Linux (Wayland), the overlay position persists across hide/show cycles (the window is never unmapped, only CSS-toggled).

## Platform Behavior

- **Windows**: Window uses Win32 `HWND_TOPMOST` to force always-on-top. Position is set via `set_position`. Window has no decorations, maximize, minimize, or close buttons.
- **Linux**: Window starts visible. Show/hide uses CSS visibility (opacity + pointer-events) via events, never calling native hide/show. This preserves compositor-assigned position on Wayland.
- **macOS**: Standard Tauri transparent window behavior.

## Key Files
- `src-tauri/src/overlay.rs` — window creation, positioning, show/hide
- `src/overlay/QueueOverlay.tsx` — overlay UI component
- `src/overlay/QueueOverlay.css` — frosted glass styling
- `src/overlay/main.tsx` — React entry point
- `src/overlay/index.html` — HTML shell
