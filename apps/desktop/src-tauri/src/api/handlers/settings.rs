use tauri::State;

use crate::api::to_error;
use crate::api::types::SettingsResponse;
use crate::app::DesktopState;

#[tauri::command]
pub fn settings_get(state: State<DesktopState>) -> Result<SettingsResponse, String> {
    let snapshot = state.app_state.services.settings.get().map_err(to_error)?;
    Ok(SettingsResponse {
        codex_home: snapshot.codex_home,
        active_home_id: snapshot.active_home_id,
        context_active_minutes: snapshot.context_active_minutes,
        db_path: state.app_state.config.db_path.to_string_lossy().to_string(),
        pricing_defaults_path: state
            .app_state
            .config
            .pricing_defaults_path
            .to_string_lossy()
            .to_string(),
        app_data_dir: state.app_data_dir.to_string_lossy().to_string(),
        legacy_backup_dir: state
            .legacy_backup_dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub fn settings_put(
    state: State<DesktopState>,
    codex_home: Option<String>,
    context_active_minutes: Option<u32>,
) -> Result<SettingsResponse, String> {
    state
        .app_state
        .services
        .settings
        .update(codex_home.as_deref(), context_active_minutes)
        .map_err(to_error)?;
    settings_get(state)
}
