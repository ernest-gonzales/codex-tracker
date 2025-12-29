use tauri::State;

use crate::api::to_error;
use crate::app::DesktopState;
use app_api::SettingsResponse;

#[tauri::command]
pub fn settings_get(state: State<DesktopState>) -> Result<SettingsResponse, String> {
    app_api::settings_get(&state).map_err(to_error)
}

#[tauri::command]
pub fn settings_put(
    state: State<DesktopState>,
    codex_home: Option<String>,
    context_active_minutes: Option<u32>,
) -> Result<SettingsResponse, String> {
    app_api::settings_put(
        &state,
        app_api::SettingsPutRequest {
            codex_home,
            context_active_minutes,
        },
    )
    .map_err(to_error)
}
