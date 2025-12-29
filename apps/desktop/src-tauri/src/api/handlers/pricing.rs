use tauri::State;

use crate::api::to_error;
use crate::app::DesktopState;
use app_api::PricingRuleResponse;
use tracker_core::PricingRuleInput;

#[tauri::command]
pub fn pricing_list(state: State<DesktopState>) -> Result<Vec<PricingRuleResponse>, String> {
    app_api::pricing_list(&state).map_err(to_error)
}

#[tauri::command]
pub fn pricing_replace(
    state: State<DesktopState>,
    rules: Vec<PricingRuleInput>,
) -> Result<serde_json::Value, String> {
    let response = app_api::pricing_replace(&state, app_api::PricingReplaceRequest { rules })
        .map_err(to_error)?;
    Ok(serde_json::json!({ "updated": response.updated }))
}

#[tauri::command]
pub fn pricing_recompute(state: State<DesktopState>) -> Result<serde_json::Value, String> {
    let response = app_api::pricing_recompute(&state).map_err(to_error)?;
    Ok(serde_json::json!({ "updated": response.updated }))
}
