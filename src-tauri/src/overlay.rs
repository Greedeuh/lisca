use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};

const OVERLAY_WIDTH: f64 = 320.0;
const OVERLAY_HEIGHT: f64 = 400.0;
const OVERLAY_MARGIN: f64 = 10.0;

static POSITIONED: AtomicBool = AtomicBool::new(false);

#[allow(unused_mut)]
pub fn create_overlay(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    let visible = true;
    #[cfg(not(target_os = "linux"))]
    let visible = false;

    let mut builder = WebviewWindowBuilder::new(
        app,
        "overlay",
        tauri::WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Lisca Queue")
    .resizable(false)
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .focused(false)
    .visible(visible)
    .accept_first_mouse(true);

    #[cfg(target_os = "windows")]
    {
        builder = builder.maximizable(false).minimizable(false).closable(false);
    }

    builder.build().expect("failed to create overlay window");
}

pub fn show_overlay(app: &AppHandle) {
    if !POSITIONED.swap(true, Ordering::SeqCst) {
        if let Some(w) = app.get_webview_window("overlay") {
            position_top_right(app, &w);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app.emit("overlay-visibility", true);
    }
    #[cfg(not(target_os = "linux"))]
    {
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.show();
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(w) = app.get_webview_window("overlay") {
            force_topmost(&w);
        }
    }
}

pub fn hide_overlay(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    {
        let _ = app.emit("overlay-visibility", false);
    }
    #[cfg(not(target_os = "linux"))]
    {
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.hide();
        }
    }
}

fn position_top_right(app: &AppHandle, window: &tauri::webview::WebviewWindow) {
    // Prefer the monitor the window is currently on
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());

    let monitor = match monitor {
        Some(m) => m,
        None => return,
    };

    let scale = monitor.scale_factor();
    let monitor_x = monitor.position().x as f64 / scale;
    let monitor_y = monitor.position().y as f64 / scale;
    let monitor_width = monitor.size().width as f64 / scale;

    let x = monitor_x + monitor_width - OVERLAY_WIDTH - OVERLAY_MARGIN;
    let y = monitor_y + OVERLAY_MARGIN;

    let _ = window.set_position(tauri::Position::Logical(
        tauri::LogicalPosition { x, y },
    ));
}

#[cfg(target_os = "windows")]
fn force_topmost(window: &tauri::webview::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    let w = window.clone();
    let _ = w.clone().run_on_main_thread(move || {
        if let Ok(hwnd) = w.hwnd() {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    });
}
