use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::api::to_error;
use crate::app::{expand_home_path, DesktopState};

#[tauri::command]
pub fn open_logs_dir(app: AppHandle, state: State<DesktopState>) -> Result<(), String> {
    let home = state.app_state.services.homes.active().map_err(to_error)?;
    let path = expand_home_path(&home.path);
    if !path.exists() {
        return Err(format!("Codex home not found at {}", path.display()));
    }
    let path_string = path.to_string_lossy().to_string();
    app.opener()
        .open_path(path_string, None::<&str>)
        .map_err(to_error)
}
