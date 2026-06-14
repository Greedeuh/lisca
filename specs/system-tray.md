# System Tray

## Feature
A system tray icon with context menu for managing the app window and quitting.

## Scenarios

- **As a user**, I can left-click the tray icon to show the main window and bring it to focus.
- **As a user**, I can right-click the tray icon to access a context menu with "Show" and "Quit" options.
- **As a user**, when I close the main window, it hides to the tray instead of quitting — the app keeps running in the background.
- **As a user**, I can click "Show" in the tray menu to restore the main window and hide the overlay.
- **As a user**, I can click "Quit" in the tray menu to fully exit the application.

## Key Files
- `src-tauri/src/lib.rs` — tray creation, menu events, window close interception
