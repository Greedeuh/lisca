use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

pub fn read_text(app: &AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| format!("Clipboard read failed: {}", e))
        .map(|s| s.to_string())
}

#[tauri::command]
pub fn read_selected_text(app: AppHandle) -> Result<String, String> {
    read_text(&app)
}
