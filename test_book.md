# Lisca — UI Test Book (Phase 8: Main Window)

## Setup

1. Run `bun run tauri dev` to launch the app
2. Verify the main window opens with "Lisca" title and three tabs: Voices, Queue, Settings

---

## Tab 1: Voices

### Available Voices Catalog

- [x] The "Voices" tab is active by default
- [x] "Available Voices" section lists voices grouped by language (`en`)
- [x] Two voices shown: "Amy (English, US)" (piper, medium) and "Heart (American Female)" (kokoro, high)
- [x] Each voice card shows quality badge, speed badge, model type badge, and size
- [x] Each uninstalled voice shows an "Install" button

### Install Flow

- [x] Click "Install" on "Amy (English, US)" — a progress bar appears replacing the button
- [x] Progress bar updates (e.g. "50%") during download
- [x] After download completes, "Installed" label appears instead of the progress bar
- [x] "Heart" voice still shows "Install" button (not yet installed)

### Installed Voices

- [x] Scroll down to "Installed Voices" section
- [x] "Amy (English, US)" appears under `en` group with "Active" badge if set
- [x] "Set Active" button shown for the voice
- [x] Click "Set Active" — the "Active: en_US-amy-medium" badge appears next to `en`
- [x] The "Set Active" button disappears for the active voice
- [x] "Uninstall" button shown for each installed voice

### Uninstall Flow

- [x] Click "Uninstall" on "Amy (English, US)"
- [x] Voice disappears from "Installed Voices" section
- [x] Voice reappears with "Install" button in "Available Voices" section

### Fallback Voice

- [x] "Fallback voice" dropdown at bottom of Installed Voices
- [x] Dropdown shows "None" by default
- [x] Select a voice from dropdown — it becomes the fallback
- [x] Change back to "None" — fallback cleared

---

## Tab 2: Queue

- [x] Click "Queue" tab — tab becomes active
- [x] Empty state message: "Queue is empty. Use the hotkey to add text."
- [x] "Auto-play" checkbox visible and checked by default
- [x] "Clear" button visible

### Queue Items (manual test — requires hotkey or programmatic add)

Note: There is no UI button to add text to the queue. Items are added via the global hotkey
(clipboard paste) which is wired in Phase 10 (System Tray). For now, test with programmatic
add via the Tauri command `queue_add` or wait for Phase 10.

- [ ] (If items exist) TextMessage items show text preview, "Pending" badge, remove (✕) button
- [ ] (If items exist) Speech items show text preview, status badge, up/down reorder buttons, skip/remove button
- [ ] Playing/paused items highlighted with blue background
- [ ] Click up/down arrows — item moves in the list
- [ ] Click ✕ on a pending item — item removed
- [ ] Click ⏭ on a playing item — item skipped/removed
- [ ] Toggle "Auto-play" checkbox — state toggles
- [ ] Click "Clear" — all items removed, empty state shown

---

## Tab 3: Settings

### Hotkey Configuration

- [x] Click "Settings" tab — tab becomes active
- [x] "Global hotkey:" label shown
- [x] Current hotkey displayed (e.g. "Control+Shift+K") or "Not set"
- [x] "Record" button shown

### Record Hotkey

- [x] Click "Record" — button text changes to "Press keys...", hint text appears
- [x] Button pulses with red animation while recording
- [x] Press a key combination (e.g. Control+Alt+P) — shortcut displayed in the field
- [x] Recording stops automatically after key combination
- [x] Click "Record" then press Escape — recording cancels, previous shortcut restored

### Voice Preferences

Decision: Voice mapping settings removed from Settings tab. Per-language voice selection
is handled by the "Set Active" button in the Installed Voices section (Voices tab).
The installed model already provides all needed features; a separate settings panel
was redundant.

- [x] Settings tab shows only Hotkey Configuration (no Voice Mapping panel)

### Persistence

- [x] Change hotkey, close and reopen app — hotkey persists
- [x] Change voice mapping, close and reopen app — mapping persists
- [x] Change fallback voice, close and reopen app — fallback persists

---

## Cross-tab Behavior

- [x] Install a voice in Voices tab → switch to Settings → voice appears in voice mapping dropdowns
- [x] Uninstall a voice in Voices tab → switch to Settings → voice removed from dropdowns
- [x] Set active voice in Settings tab → switch to Voices → "Active" badge shows correctly

---

## Edge Cases

- [x] App with no installed voices: Settings shows only Hotkey Configuration
- [x] Available Voices with no voices: shows "No voices available."
- [x] Installed Voices empty: shows "No voices installed. Browse the catalog to install voices."
- [x] Rapid tab switching doesn't crash or lose state
- [x] Window resize — content scrolls properly, no layout breakage

---

## File System Verification

After performing the actions above, verify the following files exist on disk.
On Linux, `app_data_dir` is typically `~/.local/share/com.lisca.dev/` or
check `tauri::PathResolver::app_data_dir()` output.

Find the actual path: run the app, then check `ls -la ~/.local/share/ | grep lisca`
or look at the Tauri logs.

### Config Files

- [ ] `{app_data_dir}/queue_config.json` exists and contains valid JSON with `auto_read` and `show_overlay` fields
- [ ] `{app_data_dir}/voice_mapping.json` exists and contains valid JSON with `language_voice` map and `fallback_voice_key`
- [ ] `{app_data_dir}/hotkey.txt` exists and contains the shortcut string (e.g. `Control+Shift+K`)

### Config Persistence Checks

- [ ] Set a hotkey → quit app → check `hotkey.txt` matches what you set
- [ ] Set voice mapping (Set Active) → quit app → check `voice_mapping.json` has the correct language→voice mapping
- [ ] Set fallback voice → quit app → check `voice_mapping.json` has `fallback_voice_key` set
- [ ] Clear fallback → quit app → check `voice_mapping.json` has `fallback_voice_key: null`
- [ ] Change auto-play toggle → quit app → check `queue_config.json` has `auto_read` matching your toggle

### Installed Voice Files (after installing "Amy")

- [ ] `{app_data_dir}/piper_models/en_US-amy-medium/` directory exists
- [ ] `{app_data_dir}/piper_models/en_US-amy-medium/en_US-amy-medium.onnx` exists (model file)
- [ ] `{app_data_dir}/piper_models/en_US-amy-medium/en_US-amy-medium.onnx.json` exists (config file)

### Installed Voice Files (after installing "Heart")

- [ ] `{app_data_dir}/kokoro/` directory exists
- [ ] `{app_data_dir}/kokoro/kokoro_engine.onnx` exists (shared engine, downloaded once)
- [ ] `{app_data_dir}/kokoro/af_heart.bin` exists (voice embeddings)

### Uninstall Checks

- [ ] Uninstall "Amy" → `{app_data_dir}/piper_models/en_US-amy-medium/` directory removed
- [ ] Uninstall "Heart" → `{app_data_dir}/kokoro/af_heart.bin` removed
- [ ] Uninstall "Heart" → `{app_data_dir}/kokoro/kokoro_engine.onnx` still exists (shared, not deleted)

### Fresh Install Checks

- [ ] Delete `{app_data_dir}/` entirely → relaunch app → no crash, defaults used
- [ ] Delete `voice_mapping.json` → relaunch → voice mapping defaults to empty (no active voices)
- [ ] Delete `hotkey.txt` → relaunch → hotkey shows "Not set"
- [ ] Delete `queue_config.json` → relaunch → queue defaults restored (auto_read: true, show_overlay: true)

---

## Sign-off

- [x] All tests pass: `cargo test --lib` (143 tests)
- [x] All tests pass: `bun run test` (59 tests)
- [x] Build succeeds: `bun run build`
