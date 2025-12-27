use std::path::{Path, PathBuf};

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Datelike, Duration, Local, SecondsFormat, TimeZone, Utc};
use ingest::{IngestStats, ingest_codex_home};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::process::Command;
use tower_http::services::{ServeDir, ServeFile};
use tracker_core::{
    ActiveSession, CodexHome, ContextPressureStats, ModelBreakdown, ModelCostBreakdown,
    ModelEffortCostBreakdown, ModelEffortTokenBreakdown, ModelTokenBreakdown, PricingRuleInput,
    TimeRange, TimeSeriesPoint, UsageLimitCurrentResponse, UsageLimitSnapshot, UsageLimitWindow,
};
use tracker_db::{Bucket, Db, Metric};

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    pricing_defaults_path: PathBuf,
}

#[derive(Deserialize)]
struct RangeQuery {
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
}

#[derive(Deserialize)]
struct ActiveSessionsQuery {
    active_minutes: Option<u32>,
}

#[derive(Deserialize)]
struct TimeSeriesQuery {
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
    bucket: Option<String>,
    metric: Option<String>,
}

#[derive(Deserialize)]
struct EventsQuery {
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct BreakdownQuery {
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
}

#[derive(Deserialize)]
struct SettingsPayload {
    codex_home: Option<String>,
    context_active_minutes: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct HomesResponse {
    active_home_id: Option<i64>,
    homes: Vec<CodexHome>,
}

#[derive(Serialize, Deserialize)]
struct LimitsResponse {
    primary: Option<UsageLimitSnapshot>,
    secondary: Option<UsageLimitSnapshot>,
}

fn resolve_dist_dir() -> PathBuf {
    let env_override = std::env::var_os("CODEX_TRACKER_DIST").map(PathBuf::from);
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from));
    resolve_dist_dir_with(env_override, exe_dir)
}

fn resolve_dist_dir_with(env_override: Option<PathBuf>, exe_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = env_override {
        return dir;
    }
    if let Some(dir) = exe_dir {
        let candidate = dir.join("dist");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("apps/web/dist")
}

fn resolve_app_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
}

#[derive(Serialize)]
struct PricingRuleResponse {
    id: Option<i64>,
    model_pattern: String,
    input_per_1m: f64,
    cached_input_per_1m: f64,
    output_per_1m: f64,
    input_per_1k: f64,
    cached_input_per_1k: f64,
    output_per_1k: f64,
    effective_from: String,
    effective_to: Option<String>,
}

#[derive(Deserialize)]
struct HomeCreatePayload {
    path: String,
    label: Option<String>,
}

#[derive(Deserialize)]
struct ActiveHomePayload {
    id: i64,
}

#[derive(Deserialize)]
struct LimitWindowsQuery {
    limit: Option<usize>,
}

#[tokio::main]
async fn main() {
    let app_dir = resolve_app_dir().or_else(|| std::env::current_dir().ok());
    let db_path = resolve_db_path_with(app_dir.clone());
    let pricing_defaults_path = resolve_pricing_defaults_path(app_dir);
    let is_fresh_db = !db_path.exists();
    if let Err(err) = setup_db(&db_path) {
        eprintln!("failed to initialize database: {}", err);
        std::process::exit(1);
    }
    if is_fresh_db && let Err(err) = apply_pricing_defaults(&db_path, &pricing_defaults_path) {
        eprintln!("failed to apply pricing defaults: {}", err);
    }
    if let Err(err) = sync_pricing_defaults(&db_path, &pricing_defaults_path) {
        eprintln!("failed to sync pricing defaults: {}", err);
    }
    let state = AppState {
        db_path,
        pricing_defaults_path,
    };
    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3030")
        .await
        .expect("bind server");
    let url = "http://127.0.0.1:3030";
    if let Err(err) = open_browser(url) {
        eprintln!("failed to open browser: {}", err);
    }
    axum::serve(listener, app).await.expect("serve");
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }
}

fn build_app(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/health", get(health))
        .route("/api/summary", get(summary))
        .route("/api/context/latest", get(context_latest))
        .route("/api/context/sessions", get(context_sessions))
        .route("/api/context/stats", get(context_stats))
        .route("/api/timeseries", get(timeseries))
        .route("/api/breakdown", get(breakdown))
        .route("/api/breakdown/tokens", get(breakdown_tokens))
        .route("/api/breakdown/costs", get(breakdown_costs))
        .route("/api/breakdown/effort/tokens", get(breakdown_effort_tokens))
        .route("/api/breakdown/effort/costs", get(breakdown_effort_costs))
        .route("/api/events", get(events))
        .route("/api/limits", get(limits_latest))
        .route("/api/limits/current", get(limits_current))
        .route("/api/limits/7d/windows", get(limits_7d_windows))
        .route("/api/ingest/run", post(ingest))
        .route("/api/pricing", get(pricing_list).put(pricing_replace))
        .route("/api/pricing/recompute", post(pricing_recompute))
        .route("/api/settings", get(settings_get).put(settings_put))
        .route("/api/homes", get(homes_list).post(homes_create))
        .route("/api/homes/active", put(homes_set_active))
        .route("/api/homes/:id", delete(homes_delete))
        .route("/api/homes/:id/data", delete(homes_clear_data))
        .with_state(state.clone());

    let dist_dir = resolve_dist_dir();
    let static_service =
        ServeDir::new(&dist_dir).fallback(ServeFile::new(dist_dir.join("index.html")));

    api.fallback_service(static_service)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn summary(
    State(state): State<AppState>,
    Query(query): Query<RangeQuery>,
) -> Result<Json<tracker_core::UsageSummary>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(query)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.summary(&range, home.id).map(Json).map_err(to_api_error)
}

async fn context_latest(
    State(state): State<AppState>,
) -> Result<Json<Option<tracker_core::ContextStatus>>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.latest_context(home.id).map(Json).map_err(to_api_error)
}

async fn context_sessions(
    State(state): State<AppState>,
    Query(query): Query<ActiveSessionsQuery>,
) -> Result<Json<Vec<ActiveSession>>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let minutes = match query.active_minutes {
        Some(value) => value,
        None => db.get_context_active_minutes().map_err(to_api_error)?,
    };
    let since = (Utc::now() - Duration::minutes(minutes as i64))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    db.active_sessions(home.id, &since)
        .map(Json)
        .map_err(to_api_error)
}

async fn context_stats(
    State(state): State<AppState>,
    Query(query): Query<RangeQuery>,
) -> Result<Json<ContextPressureStats>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(query)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.context_pressure_stats(&range, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn timeseries(
    State(state): State<AppState>,
    Query(query): Query<TimeSeriesQuery>,
) -> Result<Json<Vec<TimeSeriesPoint>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let bucket = match query.bucket.as_deref().unwrap_or("day") {
        "hour" => Bucket::Hour,
        "day" => Bucket::Day,
        value => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("unsupported bucket {}", value),
                }),
            ));
        }
    };
    let metric = match query.metric.as_deref().unwrap_or("tokens") {
        "tokens" => Metric::Tokens,
        "cost" => Metric::Cost,
        value => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("unsupported metric {}", value),
                }),
            ));
        }
    };
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.timeseries(&range, bucket, metric, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn breakdown(
    State(state): State<AppState>,
    Query(query): Query<BreakdownQuery>,
) -> Result<Json<Vec<ModelBreakdown>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model(&range, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn breakdown_tokens(
    State(state): State<AppState>,
    Query(query): Query<BreakdownQuery>,
) -> Result<Json<Vec<ModelTokenBreakdown>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_tokens(&range, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn breakdown_costs(
    State(state): State<AppState>,
    Query(query): Query<BreakdownQuery>,
) -> Result<Json<Vec<ModelCostBreakdown>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_costs(&range, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn breakdown_effort_tokens(
    State(state): State<AppState>,
    Query(query): Query<BreakdownQuery>,
) -> Result<Json<Vec<ModelEffortTokenBreakdown>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_effort_tokens(&range, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn breakdown_effort_costs(
    State(state): State<AppState>,
    Query(query): Query<BreakdownQuery>,
) -> Result<Json<Vec<ModelEffortCostBreakdown>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_effort_costs(&range, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Vec<tracker_core::UsageEvent>>, (StatusCode, Json<ApiError>)> {
    let range = resolve_range(RangeQuery {
        range: query.range,
        start: query.start,
        end: query.end,
    })?;
    let limit = query.limit.unwrap_or(200).min(1000);
    let offset = query.offset.unwrap_or(0);
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.list_usage_events(&range, query.model.as_deref(), limit, offset, home.id)
        .map(Json)
        .map_err(to_api_error)
}

async fn limits_latest(
    State(state): State<AppState>,
) -> Result<Json<LimitsResponse>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let primary = db
        .latest_limit_snapshot_current(home.id, "5h")
        .map_err(to_api_error)?;
    let secondary = db
        .latest_limit_snapshot_current(home.id, "7d")
        .map_err(to_api_error)?;
    Ok(Json(LimitsResponse { primary, secondary }))
}

async fn limits_current(
    State(state): State<AppState>,
) -> Result<Json<UsageLimitCurrentResponse>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let primary = db
        .limit_current_window(home.id, "5h")
        .map_err(to_api_error)?;
    let secondary = db
        .limit_current_window(home.id, "7d")
        .map_err(to_api_error)?;
    Ok(Json(UsageLimitCurrentResponse { primary, secondary }))
}

async fn limits_7d_windows(
    State(state): State<AppState>,
    Query(query): Query<LimitWindowsQuery>,
) -> Result<Json<Vec<UsageLimitWindow>>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let limit = query.limit.unwrap_or(8).min(24);
    db.limit_windows_7d(home.id, limit)
        .map(Json)
        .map_err(to_api_error)
}

async fn ingest(
    State(state): State<AppState>,
) -> Result<Json<IngestStats>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let stats = ingest_codex_home(&mut db, Path::new(&home.path)).map_err(to_api_error)?;
    db.update_event_costs(home.id).map_err(to_api_error)?;
    Ok(Json(stats))
}

async fn pricing_list(
    State(state): State<AppState>,
) -> Result<Json<Vec<PricingRuleResponse>>, (StatusCode, Json<ApiError>)> {
    let db = open_db(&state)?;
    let rules = db.list_pricing_rules().map_err(to_api_error)?;
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
    Ok(Json(response))
}

async fn pricing_replace(
    State(state): State<AppState>,
    Json(payload): Json<Vec<PricingRuleInput>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let count = db.replace_pricing_rules(&payload).map_err(to_api_error)?;
    if let Err(err) = write_pricing_defaults(&state.pricing_defaults_path, &payload) {
        eprintln!("failed to update pricing defaults: {}", err);
    }
    Ok(Json(serde_json::json!({ "updated": count })))
}

async fn pricing_recompute(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let updated = db.update_event_costs(home.id).map_err(to_api_error)?;
    Ok(Json(serde_json::json!({ "updated": updated })))
}

async fn settings_get(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let context_active_minutes = db.get_context_active_minutes().map_err(to_api_error)?;
    Ok(Json(serde_json::json!({
        "codex_home": home.path,
        "active_home_id": home.id,
        "context_active_minutes": context_active_minutes
    })))
}

async fn settings_put(
    State(state): State<AppState>,
    Json(payload): Json<SettingsPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let db = open_db(&state)?;
    if let Some(codex_home) = payload.codex_home {
        let home = db
            .get_or_create_home(&codex_home, Some("Default"))
            .map_err(to_api_error)?;
        db.set_active_home(home.id).map_err(to_api_error)?;
    }
    if let Some(minutes) = payload.context_active_minutes {
        db.set_context_active_minutes(minutes)
            .map_err(to_api_error)?;
    }
    settings_get(State(state)).await
}

async fn homes_list(
    State(state): State<AppState>,
) -> Result<Json<HomesResponse>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let active = require_active_home(&mut db)?;
    let homes = db.list_homes().map_err(to_api_error)?;
    Ok(Json(HomesResponse {
        active_home_id: Some(active.id),
        homes,
    }))
}

async fn homes_create(
    State(state): State<AppState>,
    Json(payload): Json<HomeCreatePayload>,
) -> Result<Json<CodexHome>, (StatusCode, Json<ApiError>)> {
    let db = open_db(&state)?;
    let path = payload.path.trim();
    if path.is_empty() {
        return Err(to_bad_request("path is required"));
    }
    let label = payload
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let home = db.get_or_create_home(path, label).map_err(to_api_error)?;
    db.set_active_home(home.id).map_err(to_api_error)?;
    db.update_home_last_seen(home.id).map_err(to_api_error)?;
    Ok(Json(home))
}

async fn homes_set_active(
    State(state): State<AppState>,
    Json(payload): Json<ActiveHomePayload>,
) -> Result<Json<CodexHome>, (StatusCode, Json<ApiError>)> {
    let db = open_db(&state)?;
    let home = db
        .get_home_by_id(payload.id)
        .map_err(to_api_error)?
        .ok_or_else(|| to_bad_request("home not found"))?;
    db.set_active_home(home.id).map_err(to_api_error)?;
    db.update_home_last_seen(home.id).map_err(to_api_error)?;
    Ok(Json(home))
}

async fn homes_delete(
    State(state): State<AppState>,
    AxumPath(home_id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    let active = require_active_home(&mut db)?;
    if active.id == home_id {
        let homes = db.list_homes().map_err(to_api_error)?;
        let replacement = homes
            .into_iter()
            .find(|home| home.id != home_id)
            .ok_or_else(|| to_bad_request("cannot delete the last home"))?;
        db.set_active_home(replacement.id).map_err(to_api_error)?;
    }
    db.delete_home(home_id).map_err(to_api_error)?;
    Ok(Json(serde_json::json!({ "deleted": home_id })))
}

async fn homes_clear_data(
    State(state): State<AppState>,
    AxumPath(home_id): AxumPath<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let mut db = open_db(&state)?;
    db.get_home_by_id(home_id)
        .map_err(to_api_error)?
        .ok_or_else(|| to_bad_request("home not found"))?;
    db.clear_home_data(home_id).map_err(to_api_error)?;
    Ok(Json(serde_json::json!({ "cleared": home_id })))
}

fn open_db(state: &AppState) -> Result<Db, (StatusCode, Json<ApiError>)> {
    Db::open(&state.db_path).map_err(to_api_error)
}

fn require_active_home(db: &mut Db) -> Result<CodexHome, (StatusCode, Json<ApiError>)> {
    db.ensure_active_home().map_err(to_api_error)
}

fn to_api_error(err: impl std::fmt::Display) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            error: err.to_string(),
        }),
    )
}

fn resolve_db_path_with(app_dir: Option<PathBuf>) -> PathBuf {
    let base = app_dir.unwrap_or_else(|| PathBuf::from("."));
    base.join("codex-tracker.sqlite")
}

fn resolve_pricing_defaults_path(app_dir: Option<PathBuf>) -> PathBuf {
    let base = app_dir.unwrap_or_else(|| PathBuf::from("."));
    base.join("codex-tracker-pricing.json")
}

fn apply_pricing_defaults(db_path: &Path, defaults_path: &Path) -> Result<(), String> {
    let rules = if defaults_path.exists() {
        load_pricing_defaults(defaults_path)?
    } else {
        load_initial_pricing()?
    };
    let mut db = Db::open(db_path).map_err(|err| format!("open db: {}", err))?;
    db.replace_pricing_rules(&rules)
        .map_err(|err| format!("replace pricing: {}", err))?;
    Ok(())
}

fn sync_pricing_defaults(db_path: &Path, defaults_path: &Path) -> Result<(), String> {
    let db = Db::open(db_path).map_err(|err| format!("open db: {}", err))?;
    let rules = db
        .list_pricing_rules()
        .map_err(|err| format!("list pricing: {}", err))?;
    if rules.is_empty() && !defaults_path.exists() {
        return Ok(());
    }
    let inputs = rules
        .into_iter()
        .map(|rule| PricingRuleInput {
            model_pattern: rule.model_pattern,
            input_per_1m: rule.input_per_1m,
            cached_input_per_1m: rule.cached_input_per_1m,
            output_per_1m: rule.output_per_1m,
            effective_from: rule.effective_from,
            effective_to: rule.effective_to,
        })
        .collect::<Vec<_>>();
    write_pricing_defaults(defaults_path, &inputs)
}

fn load_pricing_defaults(path: &Path) -> Result<Vec<PricingRuleInput>, String> {
    let file = fs::File::open(path).map_err(|err| format!("open defaults: {}", err))?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|err| format!("parse defaults: {}", err))
}

fn load_initial_pricing() -> Result<Vec<PricingRuleInput>, String> {
    let data = include_str!("../initial-pricing.json");
    serde_json::from_str(data).map_err(|err| format!("parse initial pricing: {}", err))
}

fn write_pricing_defaults(path: &Path, rules: &[PricingRuleInput]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create defaults dir: {}", err))?;
    }
    let file = fs::File::create(path).map_err(|err| format!("create defaults: {}", err))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, rules).map_err(|err| format!("write defaults: {}", err))
}

fn setup_db(path: &Path) -> Result<(), tracker_db::DbError> {
    let mut db = Db::open(path)?;
    db.migrate()?;
    Ok(())
}

fn resolve_range(query: RangeQuery) -> Result<TimeRange, (StatusCode, Json<ApiError>)> {
    if let (Some(start), Some(end)) = (query.start.clone(), query.end.clone()) {
        let start = normalize_rfc3339_to_utc(&start)?;
        let end = normalize_rfc3339_to_utc(&end)?;
        return Ok(TimeRange { start, end });
    }
    if let Some(start) = query.start {
        let start = normalize_rfc3339_to_utc(&start)?;
        let end = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        return Ok(TimeRange { start, end });
    }
    let now_local = Local::now();
    let (start_local, end_local) = match query.range.as_deref().unwrap_or("last7days") {
        "today" => {
            let start = Local
                .with_ymd_and_hms(
                    now_local.year(),
                    now_local.month(),
                    now_local.day(),
                    0,
                    0,
                    0,
                )
                .single()
                .ok_or_else(|| to_bad_request("invalid local date"))?;
            (start, now_local)
        }
        "last7days" => {
            let start = now_local - Duration::days(7);
            (start, now_local)
        }
        "last14days" => {
            let start = now_local - Duration::days(14);
            (start, now_local)
        }
        "thismonth" => {
            let start = Local
                .with_ymd_and_hms(now_local.year(), now_local.month(), 1, 0, 0, 0)
                .single()
                .ok_or_else(|| to_bad_request("invalid local date"))?;
            (start, now_local)
        }
        "alltime" => {
            let start = Local
                .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| to_bad_request("invalid local date"))?;
            (start, now_local)
        }
        value => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("unsupported range {}", value),
                }),
            ));
        }
    };
    let start = start_local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let end = end_local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    Ok(TimeRange { start, end })
}

fn normalize_rfc3339_to_utc(value: &str) -> Result<String, (StatusCode, Json<ApiError>)> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(to_bad_request)?;
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn to_bad_request(err: impl std::fmt::Display) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: err.to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::{Request, StatusCode as HttpStatus};
    use http_body_util::BodyExt;
    use std::fs;
    use std::io::Write;
    use tower::util::ServiceExt;
    use tracker_core::{
        ContextStatus, MessageEvent, PricingRuleInput, UsageEvent, UsageLimitCurrentResponse,
        UsageLimitSnapshot, UsageTotals,
    };
    use tracker_db::IngestCursor;

    struct TestState {
        state: AppState,
        _dir: tempfile::TempDir,
    }

    async fn setup_state_with_data() -> TestState {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");
        let home = db
            .get_or_create_home("/tmp/codex-home", Some("Default"))
            .expect("home");
        db.set_active_home(home.id).expect("active");
        db.replace_pricing_rules(&[PricingRuleInput {
            model_pattern: "gpt-5.2".to_string(),
            input_per_1m: 1750.0,
            cached_input_per_1m: 175.0,
            output_per_1m: 14000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        }])
        .expect("insert pricing");
        db.insert_usage_events(
            home.id,
            &[UsageEvent {
                id: "e1".to_string(),
                ts: "2025-12-19T19:00:00Z".to_string(),
                model: "gpt-5.2".to_string(),
                usage: UsageTotals {
                    input_tokens: 1000,
                    cached_input_tokens: 200,
                    output_tokens: 300,
                    reasoning_output_tokens: 120,
                    total_tokens: 1300,
                },
                context: ContextStatus {
                    context_used: 1300,
                    context_window: 100_000,
                },
                cost_usd: None,
                reasoning_effort: Some("high".to_string()),
                source: "source-a".to_string(),
                session_id: "source-a".to_string(),
                request_id: None,
                raw_json: None,
            }],
        )
        .expect("insert events");
        db.insert_limit_snapshots(
            home.id,
            &[
                UsageLimitSnapshot {
                    limit_type: "5h".to_string(),
                    percent_left: 70.0,
                    reset_at: "2025-12-19T23:00:00Z".to_string(),
                    observed_at: "2025-12-19T19:00:00Z".to_string(),
                    source: "source-a".to_string(),
                    raw_line: None,
                },
                UsageLimitSnapshot {
                    limit_type: "7d".to_string(),
                    percent_left: 40.0,
                    reset_at: "2025-12-26T00:00:00Z".to_string(),
                    observed_at: "2025-12-19T19:00:00Z".to_string(),
                    source: "source-a".to_string(),
                    raw_line: None,
                },
            ],
        )
        .expect("insert limits");
        TestState {
            state: AppState {
                db_path,
                pricing_defaults_path: dir.path().join("pricing-defaults.json"),
            },
            _dir: dir,
        }
    }

    #[test]
    fn resolve_dist_dir_prefers_env_override() {
        let dir = tempfile::tempdir().expect("temp dir");
        let resolved = resolve_dist_dir_with(Some(dir.path().to_path_buf()), None);
        assert_eq!(resolved, dir.path());
    }

    #[test]
    fn resolve_dist_dir_uses_exe_dist_when_present() {
        let dir = tempfile::tempdir().expect("temp dir");
        let dist_dir = dir.path().join("dist");
        fs::create_dir_all(&dist_dir).expect("create dist dir");
        let resolved = resolve_dist_dir_with(None, Some(dir.path().to_path_buf()));
        assert_eq!(resolved, dist_dir);
    }

    #[test]
    fn resolve_dist_dir_falls_back_to_repo_dist() {
        let dir = tempfile::tempdir().expect("temp dir");
        let resolved = resolve_dist_dir_with(None, Some(dir.path().to_path_buf()));
        assert_eq!(resolved, PathBuf::from("apps/web/dist"));
    }

    #[test]
    fn resolve_db_path_uses_app_dir() {
        let dir = tempfile::tempdir().expect("temp dir");
        let resolved = resolve_db_path_with(Some(dir.path().to_path_buf()));
        assert_eq!(resolved, dir.path().join("codex-tracker.sqlite"));
    }

    #[test]
    fn apply_pricing_defaults_inserts_rules() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("pricing.sqlite");
        let defaults_path = dir.path().join("pricing-defaults.json");

        setup_db(&db_path).expect("setup db");
        write_pricing_defaults(
            &defaults_path,
            &[PricingRuleInput {
                model_pattern: "gpt-5.2".to_string(),
                input_per_1m: 1000.0,
                cached_input_per_1m: 100.0,
                output_per_1m: 2000.0,
                effective_from: "2024-01-01T00:00:00Z".to_string(),
                effective_to: None,
            }],
        )
        .expect("write defaults");

        apply_pricing_defaults(&db_path, &defaults_path).expect("apply defaults");

        let db = Db::open(&db_path).expect("open db");
        let rules = db.list_pricing_rules().expect("list rules");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].model_pattern, "gpt-5.2");
    }

    #[test]
    fn load_initial_pricing_is_non_empty() {
        let rules = load_initial_pricing().expect("load initial pricing");
        assert!(!rules.is_empty());
    }

    #[tokio::test]
    async fn breakdown_costs_endpoint_returns_costs() {
        let test_state = setup_state_with_data().await;
        let app = build_app(test_state.state);
        let request = Request::builder()
            .uri("/api/breakdown/costs?start=2025-12-19T18:40:00Z&end=2025-12-19T20:00:00Z")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: Vec<ModelCostBreakdown> = serde_json::from_slice(&body).expect("parse body");
        let row = payload
            .iter()
            .find(|item| item.model == "gpt-5.2")
            .expect("row");

        let expected_input = (800.0 / 1_000_000.0) * 1750.0;
        let expected_cached = (200.0 / 1_000_000.0) * 175.0;
        let expected_output = (300.0 / 1_000_000.0) * 14000.0;

        assert_eq!(row.total_tokens, 1300);
        assert!((row.input_cost_usd.unwrap() - expected_input).abs() < 1e-9);
        assert!((row.cached_input_cost_usd.unwrap() - expected_cached).abs() < 1e-9);
        assert!((row.output_cost_usd.unwrap() - expected_output).abs() < 1e-9);
    }

    #[tokio::test]
    async fn breakdown_tokens_endpoint_returns_tokens() {
        let test_state = setup_state_with_data().await;
        let app = build_app(test_state.state);
        let request = Request::builder()
            .uri("/api/breakdown/tokens?start=2025-12-19T18:40:00Z&end=2025-12-19T20:00:00Z")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: Vec<ModelTokenBreakdown> = serde_json::from_slice(&body).expect("parse body");
        let row = payload
            .iter()
            .find(|item| item.model == "gpt-5.2")
            .expect("row");

        assert_eq!(row.input_tokens, 1000);
        assert_eq!(row.cached_input_tokens, 200);
        assert_eq!(row.output_tokens, 300);
        assert_eq!(row.reasoning_output_tokens, 120);
        assert_eq!(row.total_tokens, 1300);
    }

    #[tokio::test]
    async fn summary_all_time_range_succeeds() {
        let test_state = setup_state_with_data().await;
        let app = build_app(test_state.state);
        let request = Request::builder()
            .uri("/api/summary?range=alltime")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);
    }

    #[tokio::test]
    async fn limits_latest_endpoint_returns_limits() {
        let test_state = setup_state_with_data().await;
        let app = build_app(test_state.state);
        let request = Request::builder()
            .uri("/api/limits")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: LimitsResponse = serde_json::from_slice(&body).expect("parse body");
        let primary = payload.primary.expect("primary");
        assert_eq!(primary.limit_type, "5h");
        let secondary = payload.secondary.expect("secondary");
        assert_eq!(secondary.limit_type, "7d");
    }

    #[tokio::test]
    async fn limits_current_endpoint_returns_window_totals() {
        let test_state = setup_state_with_data().await;
        let mut db = Db::open(&test_state.state.db_path).expect("open db");
        let home = db.get_active_home().expect("active home").expect("home");
        db.insert_message_events(
            home.id,
            &[MessageEvent {
                id: "m1".to_string(),
                ts: "2025-12-19T19:30:00Z".to_string(),
                role: "user".to_string(),
                source: "source-a".to_string(),
                session_id: "source-a".to_string(),
                raw_json: None,
            }],
        )
        .expect("insert messages");

        let app = build_app(test_state.state);
        let request = Request::builder()
            .uri("/api/limits/current")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: UsageLimitCurrentResponse = serde_json::from_slice(&body).expect("parse body");
        let primary = payload.primary.expect("primary");
        assert_eq!(primary.total_tokens, Some(1300));
        assert_eq!(primary.message_count, Some(1));
        let secondary = payload.secondary.expect("secondary");
        assert_eq!(secondary.total_tokens, Some(1300));
        assert_eq!(secondary.message_count, Some(1));
    }

    #[tokio::test]
    async fn limits_windows_endpoint_returns_windows() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");
        let home = db
            .get_or_create_home("/tmp/codex-home", Some("Default"))
            .expect("home");
        db.set_active_home(home.id).expect("active");
        db.insert_usage_events(
            home.id,
            &[
                UsageEvent {
                    id: "e1".to_string(),
                    ts: "2025-01-09T00:00:00Z".to_string(),
                    model: "gpt-5.2".to_string(),
                    usage: UsageTotals {
                        input_tokens: 50,
                        cached_input_tokens: 0,
                        output_tokens: 10,
                        reasoning_output_tokens: 0,
                        total_tokens: 60,
                    },
                    context: ContextStatus {
                        context_used: 60,
                        context_window: 100_000,
                    },
                    cost_usd: None,
                    reasoning_effort: None,
                    source: "source-window".to_string(),
                    session_id: "source-window".to_string(),
                    request_id: None,
                    raw_json: None,
                },
                UsageEvent {
                    id: "e2".to_string(),
                    ts: "2025-01-10T00:00:00Z".to_string(),
                    model: "gpt-5.2".to_string(),
                    usage: UsageTotals {
                        input_tokens: 100,
                        cached_input_tokens: 0,
                        output_tokens: 20,
                        reasoning_output_tokens: 0,
                        total_tokens: 120,
                    },
                    context: ContextStatus {
                        context_used: 120,
                        context_window: 100_000,
                    },
                    cost_usd: None,
                    reasoning_effort: None,
                    source: "source-window".to_string(),
                    session_id: "source-window".to_string(),
                    request_id: None,
                    raw_json: None,
                },
            ],
        )
        .expect("insert events");
        db.insert_message_events(
            home.id,
            &[
                MessageEvent {
                    id: "m1".to_string(),
                    ts: "2025-01-09T01:00:00Z".to_string(),
                    role: "user".to_string(),
                    source: "source-window".to_string(),
                    session_id: "source-window".to_string(),
                    raw_json: None,
                },
                MessageEvent {
                    id: "m2".to_string(),
                    ts: "2025-01-10T01:00:00Z".to_string(),
                    role: "user".to_string(),
                    source: "source-window".to_string(),
                    session_id: "source-window".to_string(),
                    raw_json: None,
                },
            ],
        )
        .expect("insert messages");
        db.insert_limit_snapshots(
            home.id,
            &[
                UsageLimitSnapshot {
                    limit_type: "7d".to_string(),
                    percent_left: 70.0,
                    reset_at: "2025-01-08T00:00:00Z".to_string(),
                    observed_at: "2025-01-07T12:00:00Z".to_string(),
                    source: "source-a".to_string(),
                    raw_line: None,
                },
                UsageLimitSnapshot {
                    limit_type: "7d".to_string(),
                    percent_left: 55.0,
                    reset_at: "2025-01-15T00:00:00Z".to_string(),
                    observed_at: "2025-01-10T12:00:00Z".to_string(),
                    source: "source-a".to_string(),
                    raw_line: None,
                },
            ],
        )
        .expect("insert limits");

        let app = build_app(AppState {
            db_path: db_path.clone(),
            pricing_defaults_path: dir.path().join("pricing-defaults.json"),
        });
        let request = Request::builder()
            .uri("/api/limits/7d/windows?limit=3")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: Vec<UsageLimitWindow> = serde_json::from_slice(&body).expect("parse body");
        assert_eq!(payload.len(), 2);
        assert!(!payload[0].complete);
        assert!(payload[1].complete);
        assert_eq!(payload[1].total_tokens, Some(120));
        assert_eq!(payload[1].message_count, Some(2));
    }

    #[tokio::test]
    async fn homes_endpoint_returns_active_home() {
        let test_state = setup_state_with_data().await;
        let app = build_app(test_state.state);
        let request = Request::builder()
            .uri("/api/homes")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: HomesResponse = serde_json::from_slice(&body).expect("parse body");
        let active_id = payload.active_home_id.expect("active id");
        let active = payload
            .homes
            .iter()
            .find(|home| home.id == active_id)
            .expect("home");
        assert_eq!(active.label, "Default");
    }

    #[tokio::test]
    async fn delete_home_removes_home() {
        let test_state = setup_state_with_data().await;
        let db = Db::open(&test_state.state.db_path).expect("open db");
        let home = db
            .add_home("/tmp/codex-secondary", Some("Secondary"))
            .expect("add home");

        let app = build_app(test_state.state);
        let delete_request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/homes/{}", home.id))
            .body(Body::empty())
            .expect("delete request");
        let delete_response = app
            .clone()
            .oneshot(delete_request)
            .await
            .expect("delete response");
        assert_eq!(delete_response.status(), HttpStatus::OK);

        let list_request = Request::builder()
            .uri("/api/homes")
            .body(Body::empty())
            .expect("list request");
        let list_response = app.oneshot(list_request).await.expect("list response");
        let body = list_response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: HomesResponse = serde_json::from_slice(&body).expect("parse body");
        assert!(payload.homes.iter().all(|item| item.id != home.id));
    }

    #[tokio::test]
    async fn clear_home_data_endpoint_clears_events_and_cursors() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");
        let home = db
            .get_or_create_home("/tmp/codex-home", Some("Default"))
            .expect("home");
        db.set_active_home(home.id).expect("active");
        db.insert_usage_events(
            home.id,
            &[UsageEvent {
                id: "e1".to_string(),
                ts: "2025-12-19T19:00:00Z".to_string(),
                model: "gpt-5.2".to_string(),
                usage: UsageTotals {
                    input_tokens: 100,
                    cached_input_tokens: 0,
                    output_tokens: 20,
                    reasoning_output_tokens: 0,
                    total_tokens: 120,
                },
                context: ContextStatus {
                    context_used: 120,
                    context_window: 100_000,
                },
                cost_usd: None,
                reasoning_effort: None,
                source: "source-a".to_string(),
                session_id: "source-a".to_string(),
                request_id: None,
                raw_json: None,
            }],
        )
        .expect("insert events");
        db.insert_message_events(
            home.id,
            &[MessageEvent {
                id: "m1".to_string(),
                ts: "2025-12-19T19:01:00Z".to_string(),
                role: "user".to_string(),
                source: "source-a".to_string(),
                session_id: "source-a".to_string(),
                raw_json: None,
            }],
        )
        .expect("insert messages");
        db.upsert_cursor(&IngestCursor {
            codex_home_id: home.id,
            codex_home: "/tmp/codex-home".to_string(),
            file_path: "/tmp/codex-home/log.ndjson".to_string(),
            inode: None,
            mtime: None,
            byte_offset: 123,
            last_event_key: Some("e1".to_string()),
            updated_at: "2025-12-19T19:10:00Z".to_string(),
            last_model: None,
            last_effort: None,
        })
        .expect("insert cursor");

        let app = build_app(AppState {
            db_path: db_path.clone(),
            pricing_defaults_path: dir.path().join("pricing-defaults.json"),
        });
        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/homes/{}/data", home.id))
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), HttpStatus::OK);

        let db = Db::open(&db_path).expect("open db");
        assert_eq!(db.count_usage_events(home.id).expect("count events"), 0);
        assert_eq!(db.count_message_events(home.id).expect("count messages"), 0);
        assert_eq!(db.count_ingest_cursors(home.id).expect("count cursors"), 0);
    }

    #[tokio::test]
    async fn ingest_endpoint_updates_event_costs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");
        db.set_setting("codex_home", dir.path().to_string_lossy().as_ref())
            .expect("set codex home");
        let home = db
            .get_or_create_home(dir.path().to_string_lossy().as_ref(), Some("Default"))
            .expect("home");
        db.set_active_home(home.id).expect("set active");
        db.replace_pricing_rules(&[PricingRuleInput {
            model_pattern: "gpt-test".to_string(),
            input_per_1m: 1750.0,
            cached_input_per_1m: 175.0,
            output_per_1m: 14000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        }])
        .expect("insert pricing");

        let log_path = dir.path().join("session.log");
        let mut log_file = std::fs::File::create(&log_path).expect("create log");
        let line = r#"{"timestamp":"2025-12-19T19:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":1000,"cached_input_tokens":200,"output_tokens":300,"reasoning_output_tokens":0,"total_tokens":1500},"model_context_window":100}}}"#;
        writeln!(log_file, "{}", line).expect("write log");

        let app = build_app(AppState {
            db_path,
            pricing_defaults_path: dir.path().join("pricing-defaults.json"),
        });
        let ingest_request = Request::builder()
            .method("POST")
            .uri("/api/ingest/run")
            .body(Body::empty())
            .expect("ingest request");
        let ingest_response = app
            .clone()
            .oneshot(ingest_request)
            .await
            .expect("ingest response");
        assert_eq!(ingest_response.status(), HttpStatus::OK);

        let events_request = Request::builder()
            .uri("/api/events?start=2025-12-19T18:40:00Z&end=2025-12-19T20:00:00Z&limit=10")
            .body(Body::empty())
            .expect("events request");
        let events_response = app.oneshot(events_request).await.expect("events response");
        assert_eq!(events_response.status(), HttpStatus::OK);
        let body = events_response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let payload: Vec<tracker_core::UsageEvent> =
            serde_json::from_slice(&body).expect("parse body");
        assert_eq!(payload.len(), 1);
        let cost = payload[0].cost_usd.expect("cost");
        let expected_input = (800.0 / 1_000_000.0) * 1750.0;
        let expected_cached = (200.0 / 1_000_000.0) * 175.0;
        let expected_output = (300.0 / 1_000_000.0) * 14000.0;
        let expected_total = expected_input + expected_cached + expected_output;
        assert!((cost - expected_total).abs() < 1e-9);
    }
}
