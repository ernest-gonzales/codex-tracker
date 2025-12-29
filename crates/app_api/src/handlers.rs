use std::path::PathBuf;

use ingest::IngestStats;
use tracker_app::{AppError, RangeParams, Result};
use tracker_core::{
    ActiveSession, ContextPressureStats, ContextStatus, ModelBreakdown, ModelCostBreakdown,
    ModelEffortCostBreakdown, ModelEffortTokenBreakdown, ModelTokenBreakdown, TimeRange,
    TimeSeriesPoint, UsageEvent, UsageSummary,
};
use tracker_db::{Bucket, Metric};

use crate::{
    AppContext, ClearedResponse, ContextSessionsRequest, DeletedResponse, EventsRequest,
    HomesClearDataRequest, HomesCreateRequest, HomesDeleteRequest, HomesResponse,
    HomesSetActiveRequest, LimitsResponse, LimitsWindowsRequest, OkResponse, PricingReplaceRequest,
    PricingRuleResponse, RangeRequest, SettingsPutRequest, SettingsResponse, TimeseriesRequest,
    UpdatedResponse, expand_home_path,
};

fn resolve_range(
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<TimeRange> {
    tracker_app::resolve_range(&RangeParams { range, start, end })
}

fn parse_bucket(bucket: Option<String>) -> Result<Bucket> {
    match bucket.as_deref().unwrap_or("day") {
        "hour" => Ok(Bucket::Hour),
        "day" => Ok(Bucket::Day),
        value => Err(AppError::InvalidInput(format!(
            "unsupported bucket {}",
            value
        ))),
    }
}

fn parse_metric(metric: Option<String>) -> Result<Metric> {
    match metric.as_deref().unwrap_or("tokens") {
        "tokens" => Ok(Metric::Tokens),
        "cost" => Ok(Metric::Cost),
        value => Err(AppError::InvalidInput(format!(
            "unsupported metric {}",
            value
        ))),
    }
}

pub fn summary(ctx: &AppContext, req: RangeRequest) -> Result<UsageSummary> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state.services.analytics.summary(&range)
}

pub fn context_latest(ctx: &AppContext) -> Result<Option<ContextStatus>> {
    ctx.app_state.services.analytics.context_latest()
}

pub fn context_sessions(
    ctx: &AppContext,
    req: ContextSessionsRequest,
) -> Result<Vec<ActiveSession>> {
    ctx.app_state
        .services
        .analytics
        .context_sessions(req.active_minutes)
}

pub fn context_stats(ctx: &AppContext, req: RangeRequest) -> Result<ContextPressureStats> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state.services.analytics.context_stats(&range)
}

pub fn timeseries(ctx: &AppContext, req: TimeseriesRequest) -> Result<Vec<TimeSeriesPoint>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    let bucket = parse_bucket(req.bucket)?;
    let metric = parse_metric(req.metric)?;
    ctx.app_state
        .services
        .analytics
        .timeseries(&range, bucket, metric)
}

pub fn breakdown(ctx: &AppContext, req: RangeRequest) -> Result<Vec<ModelBreakdown>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state.services.analytics.breakdown(&range)
}

pub fn breakdown_tokens(ctx: &AppContext, req: RangeRequest) -> Result<Vec<ModelTokenBreakdown>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state.services.analytics.breakdown_tokens(&range)
}

pub fn breakdown_costs(ctx: &AppContext, req: RangeRequest) -> Result<Vec<ModelCostBreakdown>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state.services.analytics.breakdown_costs(&range)
}

pub fn breakdown_effort_tokens(
    ctx: &AppContext,
    req: RangeRequest,
) -> Result<Vec<ModelEffortTokenBreakdown>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state
        .services
        .analytics
        .breakdown_effort_tokens(&range)
}

pub fn breakdown_effort_costs(
    ctx: &AppContext,
    req: RangeRequest,
) -> Result<Vec<ModelEffortCostBreakdown>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    ctx.app_state
        .services
        .analytics
        .breakdown_effort_costs(&range)
}

pub fn events(ctx: &AppContext, req: EventsRequest) -> Result<Vec<UsageEvent>> {
    let range = resolve_range(req.range, req.start, req.end)?;
    let limit = req.limit.unwrap_or(200).min(1000);
    let offset = req.offset.unwrap_or(0);
    ctx.app_state
        .services
        .analytics
        .events(&range, req.model.as_deref(), limit, offset)
}

pub fn limits_latest(ctx: &AppContext) -> Result<LimitsResponse> {
    let (primary, secondary) = ctx.app_state.services.limits.latest()?;
    Ok(LimitsResponse { primary, secondary })
}

pub fn limits_current(ctx: &AppContext) -> Result<tracker_core::UsageLimitCurrentResponse> {
    ctx.app_state.services.limits.current()
}

pub fn limits_7d_windows(
    ctx: &AppContext,
    req: LimitsWindowsRequest,
) -> Result<Vec<tracker_core::UsageLimitWindow>> {
    let limit = req.limit.unwrap_or(8).min(24);
    ctx.app_state.services.limits.windows_7d(limit)
}

pub fn pricing_list(ctx: &AppContext) -> Result<Vec<PricingRuleResponse>> {
    let rules = ctx.app_state.services.pricing.list_rules()?;
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

pub fn pricing_replace(ctx: &AppContext, req: PricingReplaceRequest) -> Result<UpdatedResponse> {
    let count = ctx.app_state.services.pricing.replace_rules(&req.rules)?;
    Ok(UpdatedResponse {
        updated: count as i64,
    })
}

pub fn pricing_recompute(ctx: &AppContext) -> Result<UpdatedResponse> {
    let updated = ctx.app_state.services.pricing.recompute_costs()?;
    Ok(UpdatedResponse {
        updated: updated as i64,
    })
}

pub fn settings_get(ctx: &AppContext) -> Result<SettingsResponse> {
    let snapshot = ctx.app_state.services.settings.get()?;
    Ok(SettingsResponse {
        codex_home: snapshot.codex_home,
        active_home_id: snapshot.active_home_id,
        context_active_minutes: snapshot.context_active_minutes,
        db_path: ctx.app_state.config.db_path.to_string_lossy().to_string(),
        pricing_defaults_path: ctx
            .app_state
            .config
            .pricing_defaults_path
            .to_string_lossy()
            .to_string(),
        app_data_dir: ctx.app_data_dir.to_string_lossy().to_string(),
        legacy_backup_dir: ctx
            .legacy_backup_dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    })
}

pub fn settings_put(ctx: &AppContext, req: SettingsPutRequest) -> Result<SettingsResponse> {
    ctx.app_state
        .services
        .settings
        .update(req.codex_home.as_deref(), req.context_active_minutes)?;
    settings_get(ctx)
}

pub fn homes_list(ctx: &AppContext) -> Result<HomesResponse> {
    let active = ctx.app_state.services.homes.active()?;
    let homes = ctx.app_state.services.homes.list()?;
    Ok(HomesResponse {
        active_home_id: Some(active.id),
        homes,
    })
}

pub fn homes_create(ctx: &AppContext, req: HomesCreateRequest) -> Result<tracker_core::CodexHome> {
    let path = req.path.trim();
    if path.is_empty() {
        return Err(AppError::InvalidInput("path is required".to_string()));
    }
    let label = req
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    ctx.app_state.services.homes.create(path, label)
}

pub fn homes_set_active(
    ctx: &AppContext,
    req: HomesSetActiveRequest,
) -> Result<tracker_core::CodexHome> {
    ctx.app_state.services.homes.set_active(req.id)
}

pub fn homes_delete(ctx: &AppContext, req: HomesDeleteRequest) -> Result<DeletedResponse> {
    ctx.app_state.services.homes.delete(req.id)?;
    Ok(DeletedResponse { deleted: req.id })
}

pub fn homes_clear_data(ctx: &AppContext, req: HomesClearDataRequest) -> Result<ClearedResponse> {
    ctx.app_state.services.homes.clear_data(req.id)?;
    Ok(ClearedResponse { cleared: req.id })
}

pub fn logs_dir(ctx: &AppContext) -> Result<PathBuf> {
    let home = ctx.app_state.services.homes.active()?;
    let path = expand_home_path(&home.path);
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Codex home not found at {}",
            path.display()
        )));
    }
    Ok(path)
}

pub fn ingest(ctx: &AppContext) -> Result<IngestStats> {
    ctx.app_state.services.ingest.run()
}

pub fn ok() -> OkResponse {
    OkResponse { ok: true }
}
