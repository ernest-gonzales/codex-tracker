use tauri::State;

use crate::api::to_error;
use crate::app::DesktopState;
use app_api::LimitsResponse;
use tracker_core::{UsageLimitCurrentResponse, UsageLimitWindow};

#[tauri::command]
pub fn limits_latest(state: State<DesktopState>) -> Result<LimitsResponse, String> {
    app_api::limits_latest(&state).map_err(to_error)
}

#[tauri::command]
pub fn limits_current(state: State<DesktopState>) -> Result<UsageLimitCurrentResponse, String> {
    app_api::limits_current(&state).map_err(to_error)
}

#[tauri::command]
pub fn limits_7d_windows(
    state: State<DesktopState>,
    limit: Option<usize>,
) -> Result<Vec<UsageLimitWindow>, String> {
    app_api::limits_7d_windows(&state, app_api::LimitsWindowsRequest { limit }).map_err(to_error)
}
