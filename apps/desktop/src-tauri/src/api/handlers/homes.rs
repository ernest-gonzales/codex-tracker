use tauri::State;

use crate::api::to_error;
use crate::app::DesktopState;
use app_api::HomesResponse;
use tracker_core::CodexHome;

#[tauri::command]
pub fn homes_list(state: State<DesktopState>) -> Result<HomesResponse, String> {
    app_api::homes_list(&state).map_err(to_error)
}

#[tauri::command]
pub fn homes_create(
    state: State<DesktopState>,
    path: String,
    label: Option<String>,
) -> Result<CodexHome, String> {
    app_api::homes_create(&state, app_api::HomesCreateRequest { path, label }).map_err(to_error)
}

#[tauri::command]
pub fn homes_set_active(state: State<DesktopState>, id: i64) -> Result<CodexHome, String> {
    app_api::homes_set_active(&state, app_api::HomesSetActiveRequest { id }).map_err(to_error)
}

#[tauri::command]
pub fn homes_delete(state: State<DesktopState>, id: i64) -> Result<serde_json::Value, String> {
    let response =
        app_api::homes_delete(&state, app_api::HomesDeleteRequest { id }).map_err(to_error)?;
    Ok(serde_json::json!({ "deleted": response.deleted }))
}

#[tauri::command]
pub fn homes_clear_data(state: State<DesktopState>, id: i64) -> Result<serde_json::Value, String> {
    let response = app_api::homes_clear_data(&state, app_api::HomesClearDataRequest { id })
        .map_err(to_error)?;
    Ok(serde_json::json!({ "cleared": response.cleared }))
}
