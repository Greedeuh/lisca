// System tray icon with menu and event handlers.
// Tray menu: Show, Show/Hide Overlay, Quit.
// Left-click shows main window, right-click shows context menu.

use crate::overlay;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub const TRAY_ID: &str = "main-tray";

fn show_main_window(app: &AppHandle) {
    let _ = overlay::hide_overlay(app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn create_tray(app: &AppHandle) -> Result<(), String> {
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let overlay_item = MenuItem::with_id(app, "toggle_overlay", "Show/Hide Overlay", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(app, &[&show_item, &overlay_item, &quit_item])
        .map_err(|e| e.to_string())?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Lisca")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                show_main_window(app);
            }
            "toggle_overlay" => {
                let _ = overlay::toggle_overlay(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}
