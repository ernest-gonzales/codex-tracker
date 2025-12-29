use serde::Serialize;
use tracker_core::{CodexHome, UsageLimitSnapshot};

#[derive(Serialize)]
pub struct PricingRuleResponse {
    pub id: Option<i64>,
    pub model_pattern: String,
    pub input_per_1m: f64,
    pub cached_input_per_1m: f64,
    pub output_per_1m: f64,
    pub input_per_1k: f64,
    pub cached_input_per_1k: f64,
    pub output_per_1k: f64,
    pub effective_from: String,
    pub effective_to: Option<String>,
}

#[derive(Serialize)]
pub struct HomesResponse {
    pub active_home_id: Option<i64>,
    pub homes: Vec<CodexHome>,
}

#[derive(Serialize)]
pub struct LimitsResponse {
    pub primary: Option<UsageLimitSnapshot>,
    pub secondary: Option<UsageLimitSnapshot>,
}

#[derive(Serialize)]
pub struct SettingsResponse {
    pub codex_home: String,
    pub active_home_id: i64,
    pub context_active_minutes: u32,
    pub db_path: String,
    pub pricing_defaults_path: String,
    pub app_data_dir: String,
    pub legacy_backup_dir: Option<String>,
}

#[derive(Serialize)]
pub struct UpdatedResponse {
    pub updated: i64,
}

#[derive(Serialize)]
pub struct DeletedResponse {
    pub deleted: i64,
}

#[derive(Serialize)]
pub struct ClearedResponse {
    pub cleared: i64,
}

#[derive(Serialize)]
pub struct OkResponse {
    pub ok: bool,
}
