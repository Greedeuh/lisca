// Overlay window management: create, show, hide, toggle.
// The overlay is a transparent, always-on-top, frameless window
// that shows the queue when the main window is closed.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const OVERLAY_LABEL: &str = "overlay";
pub const OVERLAY_WIDTH: f64 = 340.0;
pub const OVERLAY_HEIGHT: f64 = 400.0;

fn position_overlay(win: &tauri::WebviewWindow) {
    if let Some(monitor) = win.primary_monitor().ok().flatten() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let x = (size.width as f64 / scale) - OVERLAY_WIDTH - 16.0;
        let y = 48.0;
        let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
    }
}

pub fn create_overlay(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    let _win = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Lisca Overlay")
        .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    position_overlay(&_win);
    Ok(())
}

pub fn show_overlay(app: &AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window not found")?;
    position_overlay(&win);
    win.show().map_err(|e| e.to_string())
}

pub fn hide_overlay(app: &AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window not found")?;
    win.hide().map_err(|e| e.to_string())
}

pub fn toggle_overlay(app: &AppHandle) -> Result<bool, String> {
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window not found")?;
    if win.is_visible().unwrap_or(false) {
        win.hide().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        position_overlay(&win);
        win.show().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

pub fn update_overlay_visibility(app: &AppHandle, has_items: bool) {
    if !has_items {
        if let Some(win) = app.get_webview_window(OVERLAY_LABEL) {
            if win.is_visible().unwrap_or(false) {
                let _ = win.hide();
                let _ = app.emit("overlay_hidden", ());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_constants_are_sane() {
        assert!(OVERLAY_WIDTH > 0.0);
        assert!(OVERLAY_HEIGHT > 0.0);
        assert_eq!(OVERLAY_LABEL, "overlay");
    }
}
