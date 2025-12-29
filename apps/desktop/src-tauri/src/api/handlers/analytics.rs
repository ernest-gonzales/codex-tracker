use tauri::State;

use crate::api::to_error;
use crate::app::DesktopState;
use app_api::{ContextSessionsRequest, EventsRequest, RangeRequest, TimeseriesRequest};
use tracker_core::{
    ActiveSession, ContextPressureStats, ModelBreakdown, ModelCostBreakdown,
    ModelEffortCostBreakdown, ModelEffortTokenBreakdown, ModelTokenBreakdown, TimeSeriesPoint,
    UsageEvent, UsageSummary,
};

#[tauri::command]
pub fn summary(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<UsageSummary, String> {
    app_api::summary(&state, RangeRequest { range, start, end }).map_err(to_error)
}

#[tauri::command]
pub fn context_latest(
    state: State<DesktopState>,
) -> Result<Option<tracker_core::ContextStatus>, String> {
    app_api::context_latest(&state).map_err(to_error)
}

#[tauri::command]
pub fn context_sessions(
    state: State<DesktopState>,
    active_minutes: Option<u32>,
) -> Result<Vec<ActiveSession>, String> {
    app_api::context_sessions(&state, ContextSessionsRequest { active_minutes }).map_err(to_error)
}

#[tauri::command]
pub fn context_stats(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<ContextPressureStats, String> {
    app_api::context_stats(&state, RangeRequest { range, start, end }).map_err(to_error)
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
    app_api::timeseries(
        &state,
        TimeseriesRequest {
            range,
            start,
            end,
            bucket,
            metric,
        },
    )
    .map_err(to_error)
}

#[tauri::command]
pub fn breakdown(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelBreakdown>, String> {
    app_api::breakdown(&state, RangeRequest { range, start, end }).map_err(to_error)
}

#[tauri::command]
pub fn breakdown_tokens(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelTokenBreakdown>, String> {
    app_api::breakdown_tokens(&state, RangeRequest { range, start, end }).map_err(to_error)
}

#[tauri::command]
pub fn breakdown_costs(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelCostBreakdown>, String> {
    app_api::breakdown_costs(&state, RangeRequest { range, start, end }).map_err(to_error)
}

#[tauri::command]
pub fn breakdown_effort_tokens(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelEffortTokenBreakdown>, String> {
    app_api::breakdown_effort_tokens(&state, RangeRequest { range, start, end }).map_err(to_error)
}

#[tauri::command]
pub fn breakdown_effort_costs(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelEffortCostBreakdown>, String> {
    app_api::breakdown_effort_costs(&state, RangeRequest { range, start, end }).map_err(to_error)
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
    app_api::events(
        &state,
        EventsRequest {
            range,
            start,
            end,
            limit,
            offset,
            model,
        },
    )
    .map_err(to_error)
}
