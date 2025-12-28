use tauri::State;

use crate::api::to_error;
use crate::app::DesktopState;
use ingest::IngestStats;

#[tauri::command]
pub async fn ingest(state: State<'_, DesktopState>) -> Result<IngestStats, String> {
    let app_state = state.app_state.clone();
    tauri::async_runtime::spawn_blocking(move || app_state.services.ingest.run())
        .await
        .map_err(|err| format!("ingest task: {}", err))?
        .map_err(to_error)
}
