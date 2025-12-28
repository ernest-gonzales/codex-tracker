use tauri::State;

use crate::api::to_error;
use crate::api::types::PricingRuleResponse;
use crate::app::DesktopState;
use tracker_core::PricingRuleInput;

#[tauri::command]
pub fn pricing_list(state: State<DesktopState>) -> Result<Vec<PricingRuleResponse>, String> {
    let rules = state
        .app_state
        .services
        .pricing
        .list_rules()
        .map_err(to_error)?;
    let response = rules
        .into_iter()
        .map(|rule| PricingRuleResponse {
            id: rule.id,
            model_pattern: rule.model_pattern,
            input_per_1m: rule.input_per_1m,
            cached_input_per_1m: rule.cached_input_per_1m,
            output_per_1m: rule.output_per_1m,
            input_per_1k: rule.input_per_1m / 1000.0,
            cached_input_per_1k: rule.cached_input_per_1m / 1000.0,
            output_per_1k: rule.output_per_1m / 1000.0,
            effective_from: rule.effective_from,
            effective_to: rule.effective_to,
        })
        .collect();
    Ok(response)
}

#[tauri::command]
pub fn pricing_replace(
    state: State<DesktopState>,
    rules: Vec<PricingRuleInput>,
) -> Result<serde_json::Value, String> {
    let count = state
        .app_state
        .services
        .pricing
        .replace_rules(&rules)
        .map_err(to_error)?;
    Ok(serde_json::json!({ "updated": count }))
}

#[tauri::command]
pub fn pricing_recompute(state: State<DesktopState>) -> Result<serde_json::Value, String> {
    let updated = state
        .app_state
        .services
        .pricing
        .recompute_costs()
        .map_err(to_error)?;
    Ok(serde_json::json!({ "updated": updated }))
}
