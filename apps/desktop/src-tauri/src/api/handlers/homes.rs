use tauri::State;

use crate::api::to_error;
use crate::api::types::HomesResponse;
use crate::app::DesktopState;
use tracker_core::CodexHome;

#[tauri::command]
pub fn homes_list(state: State<DesktopState>) -> Result<HomesResponse, String> {
    let active = state.app_state.services.homes.active().map_err(to_error)?;
    let homes = state.app_state.services.homes.list().map_err(to_error)?;
    Ok(HomesResponse {
        active_home_id: Some(active.id),
        homes,
    })
}

#[tauri::command]
pub fn homes_create(
    state: State<DesktopState>,
    path: String,
    label: Option<String>,
) -> Result<CodexHome, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("path is required".to_string());
    }
    let label = label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    state
        .app_state
        .services
        .homes
        .create(path, label)
        .map_err(to_error)
}

#[tauri::command]
pub fn homes_set_active(state: State<DesktopState>, id: i64) -> Result<CodexHome, String> {
    state
        .app_state
        .services
        .homes
        .set_active(id)
        .map_err(to_error)
}

#[tauri::command]
pub fn homes_delete(state: State<DesktopState>, id: i64) -> Result<serde_json::Value, String> {
    state
        .app_state
        .services
        .homes
        .delete(id)
        .map_err(to_error)?;
    Ok(serde_json::json!({ "deleted": id }))
}

#[tauri::command]
pub fn homes_clear_data(state: State<DesktopState>, id: i64) -> Result<serde_json::Value, String> {
    state
        .app_state
        .services
        .homes
        .clear_data(id)
        .map_err(to_error)?;
    Ok(serde_json::json!({ "cleared": id }))
}
