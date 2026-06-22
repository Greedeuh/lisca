/// Clipboard operations for the global hotkey flow: reads selected text by
/// simulating Ctrl+C/Cmd+C, then restores the original clipboard contents.
use std::time::Duration;
use enigo::{Direction::{Click, Press, Release}, Enigo, Key, Keyboard, Settings};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

 fn read_text(app: &AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| format!("Clipboard read failed: {}", e))
        .map(|s| s.to_string())
}

 fn write_text(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Clipboard write failed: {}", e))
}

fn simulate_copy() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Enigo init failed: {}", e))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key(modifier, Press).map_err(|e| e.to_string())?;
    enigo.key(Key::Unicode('c'), Click).map_err(|e| e.to_string())?;
    enigo.key(modifier, Release).map_err(|e| e.to_string())?;

    Ok(())
}

pub(super)  fn auto_copy_and_read(app: &AppHandle) -> Option<String> {
    let original = read_text(app).ok();

    if let Err(e) = simulate_copy() {
        log::error!("Auto-copy failed: {}", e);
        return None;
    }
    std::thread::sleep(Duration::from_millis(50));

    let selected = read_text(app).ok().filter(|t| !t.is_empty());

    if let Some(orig) = original {
        if let Err(e) = write_text(app, &orig) {
            log::error!("Clipboard restore failed: {}", e);
        }
    }

    selected
}
