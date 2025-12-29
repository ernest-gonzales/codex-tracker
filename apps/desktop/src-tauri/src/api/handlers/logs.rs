use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::api::to_error;
use crate::app::DesktopState;

#[tauri::command]
pub fn open_logs_dir(app: AppHandle, state: State<DesktopState>) -> Result<(), String> {
    let path = app_api::logs_dir(&state).map_err(to_error)?;
    let path_string = path.to_string_lossy().to_string();
    app.opener()
        .open_path(path_string, None::<&str>)
        .map_err(to_error)
}
