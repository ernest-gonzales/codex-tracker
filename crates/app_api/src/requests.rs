use serde::Deserialize;
use tracker_core::PricingRuleInput;

#[derive(Debug, Deserialize, Default)]
pub struct EmptyRequest {}

#[derive(Debug, Deserialize)]
pub struct RangeRequest {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TimeseriesRequest {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub bucket: Option<String>,
    pub metric: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventsRequest {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContextSessionsRequest {
    pub active_minutes: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct LimitsWindowsRequest {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PricingReplaceRequest {
    pub rules: Vec<PricingRuleInput>,
}

#[derive(Debug, Deserialize)]
pub struct HomesCreateRequest {
    pub path: String,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HomesSetActiveRequest {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct HomesDeleteRequest {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct HomesClearDataRequest {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct SettingsPutRequest {
    pub codex_home: Option<String>,
    pub context_active_minutes: Option<u32>,
}
