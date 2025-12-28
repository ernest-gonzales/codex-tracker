use tauri::State;

use crate::api::{resolve_range, to_error};
use crate::app::DesktopState;
use tracker_core::{
    ActiveSession, ContextPressureStats, ModelBreakdown, ModelCostBreakdown,
    ModelEffortCostBreakdown, ModelEffortTokenBreakdown, ModelTokenBreakdown, TimeSeriesPoint,
    UsageEvent, UsageSummary,
};
use tracker_db::{Bucket, Metric};

#[tauri::command]
pub fn summary(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<UsageSummary, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .summary(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn context_latest(
    state: State<DesktopState>,
) -> Result<Option<tracker_core::ContextStatus>, String> {
    state
        .app_state
        .services
        .analytics
        .context_latest()
        .map_err(to_error)
}

#[tauri::command]
pub fn context_sessions(
    state: State<DesktopState>,
    active_minutes: Option<u32>,
) -> Result<Vec<ActiveSession>, String> {
    state
        .app_state
        .services
        .analytics
        .context_sessions(active_minutes)
        .map_err(to_error)
}

#[tauri::command]
pub fn context_stats(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<ContextPressureStats, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .context_stats(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn timeseries(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
    bucket: Option<String>,
    metric: Option<String>,
) -> Result<Vec<TimeSeriesPoint>, String> {
    let range = resolve_range(range, start, end)?;
    let bucket = match bucket.as_deref().unwrap_or("day") {
        "hour" => Bucket::Hour,
        "day" => Bucket::Day,
        value => return Err(format!("unsupported bucket {}", value)),
    };
    let metric = match metric.as_deref().unwrap_or("tokens") {
        "tokens" => Metric::Tokens,
        "cost" => Metric::Cost,
        value => return Err(format!("unsupported metric {}", value)),
    };
    state
        .app_state
        .services
        .analytics
        .timeseries(&range, bucket, metric)
        .map_err(to_error)
}

#[tauri::command]
pub fn breakdown(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .breakdown(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn breakdown_tokens(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelTokenBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .breakdown_tokens(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn breakdown_costs(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelCostBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .breakdown_costs(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn breakdown_effort_tokens(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelEffortTokenBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .breakdown_effort_tokens(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn breakdown_effort_costs(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelEffortCostBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    state
        .app_state
        .services
        .analytics
        .breakdown_effort_costs(&range)
        .map_err(to_error)
}

#[tauri::command]
pub fn events(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    model: Option<String>,
) -> Result<Vec<UsageEvent>, String> {
    let range = resolve_range(range, start, end)?;
    let limit = limit.unwrap_or(200).min(1000);
    let offset = offset.unwrap_or(0);
    state
        .app_state
        .services
        .analytics
        .events(&range, model.as_deref(), limit, offset)
        .map_err(to_error)
}
