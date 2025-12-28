use tauri::State;

use crate::api::to_error;
use crate::api::types::LimitsResponse;
use crate::app::DesktopState;
use tracker_core::{UsageLimitCurrentResponse, UsageLimitWindow};

#[tauri::command]
pub fn limits_latest(state: State<DesktopState>) -> Result<LimitsResponse, String> {
    let (primary, secondary) = state.app_state.services.limits.latest().map_err(to_error)?;
    Ok(LimitsResponse { primary, secondary })
}

#[tauri::command]
pub fn limits_current(state: State<DesktopState>) -> Result<UsageLimitCurrentResponse, String> {
    state.app_state.services.limits.current().map_err(to_error)
}

#[tauri::command]
pub fn limits_7d_windows(
    state: State<DesktopState>,
    limit: Option<usize>,
) -> Result<Vec<UsageLimitWindow>, String> {
    let limit = limit.unwrap_or(8).min(24);
    state
        .app_state
        .services
        .limits
        .windows_7d(limit)
        .map_err(to_error)
}
