use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, SecondsFormat, Utc};
use ingest::IngestStats;
use serde::Serialize;
use tauri::{Manager, State};
use tracker_app::{AppState, RangeParams};
use tracker_core::{
    ActiveSession, CodexHome, ContextPressureStats, ModelBreakdown, ModelCostBreakdown,
    ModelEffortCostBreakdown, ModelEffortTokenBreakdown, ModelTokenBreakdown, PricingRuleInput,
    TimeRange, TimeSeriesPoint, UsageLimitCurrentResponse, UsageLimitSnapshot, UsageLimitWindow,
    UsageSummary,
};
use tracker_db::{Bucket, Db, Metric};

#[derive(Clone)]
struct DesktopState {
    app_state: AppState,
    app_data_dir: PathBuf,
    legacy_backup_dir: Option<PathBuf>,
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

#[derive(Serialize)]
struct HomesResponse {
    active_home_id: Option<i64>,
    homes: Vec<CodexHome>,
}

#[derive(Serialize)]
struct LimitsResponse {
    primary: Option<UsageLimitSnapshot>,
    secondary: Option<UsageLimitSnapshot>,
}

#[derive(Serialize)]
struct SettingsResponse {
    codex_home: String,
    active_home_id: i64,
    context_active_minutes: u32,
    db_path: String,
    pricing_defaults_path: String,
    app_data_dir: String,
    legacy_backup_dir: Option<String>,
}


#[tauri::command]
fn summary(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<UsageSummary, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.summary(&range, home.id).map_err(to_error)
}

#[tauri::command]
fn context_latest(state: State<DesktopState>) -> Result<Option<tracker_core::ContextStatus>, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.latest_context(home.id).map_err(to_error)
}

#[tauri::command]
fn context_sessions(
    state: State<DesktopState>,
    active_minutes: Option<u32>,
) -> Result<Vec<ActiveSession>, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let minutes = match active_minutes {
        Some(value) => value,
        None => db.get_context_active_minutes().map_err(to_error)?,
    };
    let since = (Utc::now() - Duration::minutes(minutes as i64))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    db.active_sessions(home.id, &since).map_err(to_error)
}

#[tauri::command]
fn context_stats(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<ContextPressureStats, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.context_pressure_stats(&range, home.id).map_err(to_error)
}

#[tauri::command]
fn timeseries(
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
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.timeseries(&range, bucket, metric, home.id)
        .map_err(to_error)
}

#[tauri::command]
fn breakdown(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model(&range, home.id).map_err(to_error)
}

#[tauri::command]
fn breakdown_tokens(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelTokenBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_tokens(&range, home.id)
        .map_err(to_error)
}

#[tauri::command]
fn breakdown_costs(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelCostBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_costs(&range, home.id)
        .map_err(to_error)
}

#[tauri::command]
fn breakdown_effort_tokens(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelEffortTokenBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_effort_tokens(&range, home.id)
        .map_err(to_error)
}

#[tauri::command]
fn breakdown_effort_costs(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<ModelEffortCostBreakdown>, String> {
    let range = resolve_range(range, start, end)?;
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.breakdown_by_model_effort_costs(&range, home.id)
        .map_err(to_error)
}

#[tauri::command]
fn events(
    state: State<DesktopState>,
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    model: Option<String>,
) -> Result<Vec<tracker_core::UsageEvent>, String> {
    let range = resolve_range(range, start, end)?;
    let limit = limit.unwrap_or(200).min(1000);
    let offset = offset.unwrap_or(0);
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    db.list_usage_events(&range, model.as_deref(), limit, offset, home.id)
        .map_err(to_error)
}

#[tauri::command]
fn limits_latest(state: State<DesktopState>) -> Result<LimitsResponse, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let primary = db
        .latest_limit_snapshot_current(home.id, "5h")
        .map_err(to_error)?;
    let secondary = db
        .latest_limit_snapshot_current(home.id, "7d")
        .map_err(to_error)?;
    Ok(LimitsResponse { primary, secondary })
}

#[tauri::command]
fn limits_current(state: State<DesktopState>) -> Result<UsageLimitCurrentResponse, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let primary = db.limit_current_window(home.id, "5h").map_err(to_error)?;
    let secondary = db.limit_current_window(home.id, "7d").map_err(to_error)?;
    Ok(UsageLimitCurrentResponse { primary, secondary })
}

#[tauri::command]
fn limits_7d_windows(
    state: State<DesktopState>,
    limit: Option<usize>,
) -> Result<Vec<UsageLimitWindow>, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let limit = limit.unwrap_or(8).min(24);
    db.limit_windows_7d(home.id, limit).map_err(to_error)
}

#[tauri::command]
fn ingest(state: State<DesktopState>) -> Result<IngestStats, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let stats = ingest::ingest_codex_home(&mut db, Path::new(&home.path)).map_err(to_error)?;
    db.update_event_costs(home.id).map_err(to_error)?;
    Ok(stats)
}

#[tauri::command]
fn pricing_list(state: State<DesktopState>) -> Result<Vec<PricingRuleResponse>, String> {
    let db = open_db(&state)?;
    let rules = db.list_pricing_rules().map_err(to_error)?;
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
fn pricing_replace(
    state: State<DesktopState>,
    rules: Vec<PricingRuleInput>,
) -> Result<serde_json::Value, String> {
    let mut db = open_db(&state)?;
    let count = db.replace_pricing_rules(&rules).map_err(to_error)?;
    if let Err(err) = state.app_state.write_pricing_defaults(&rules) {
        eprintln!("failed to update pricing defaults: {}", err);
    }
    Ok(serde_json::json!({ "updated": count }))
}

#[tauri::command]
fn pricing_recompute(state: State<DesktopState>) -> Result<serde_json::Value, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let updated = db.update_event_costs(home.id).map_err(to_error)?;
    Ok(serde_json::json!({ "updated": updated }))
}

#[tauri::command]
fn settings_get(state: State<DesktopState>) -> Result<SettingsResponse, String> {
    let mut db = open_db(&state)?;
    let home = require_active_home(&mut db)?;
    let context_active_minutes = db.get_context_active_minutes().map_err(to_error)?;
    Ok(SettingsResponse {
        codex_home: home.path,
        active_home_id: home.id,
        context_active_minutes,
        db_path: state.app_state.db_path.to_string_lossy().to_string(),
        pricing_defaults_path: state
            .app_state
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
fn settings_put(
    state: State<DesktopState>,
    codex_home: Option<String>,
    context_active_minutes: Option<u32>,
) -> Result<SettingsResponse, String> {
    let db = open_db(&state)?;
    if let Some(codex_home) = codex_home {
        let home = db
            .get_or_create_home(&codex_home, Some("Default"))
            .map_err(to_error)?;
        db.set_active_home(home.id).map_err(to_error)?;
    }
    if let Some(minutes) = context_active_minutes {
        db.set_context_active_minutes(minutes).map_err(to_error)?;
    }
    settings_get(state)
}

#[tauri::command]
fn homes_list(state: State<DesktopState>) -> Result<HomesResponse, String> {
    let mut db = open_db(&state)?;
    let active = require_active_home(&mut db)?;
    let homes = db.list_homes().map_err(to_error)?;
    Ok(HomesResponse {
        active_home_id: Some(active.id),
        homes,
    })
}

#[tauri::command]
fn homes_create(
    state: State<DesktopState>,
    path: String,
    label: Option<String>,
) -> Result<CodexHome, String> {
    let db = open_db(&state)?;
    let path = path.trim();
    if path.is_empty() {
        return Err("path is required".to_string());
    }
    let label = label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let home = db.get_or_create_home(path, label).map_err(to_error)?;
    db.set_active_home(home.id).map_err(to_error)?;
    db.update_home_last_seen(home.id).map_err(to_error)?;
    Ok(home)
}

#[tauri::command]
fn homes_set_active(state: State<DesktopState>, id: i64) -> Result<CodexHome, String> {
    let db = open_db(&state)?;
    let home = db
        .get_home_by_id(id)
        .map_err(to_error)?
        .ok_or_else(|| "home not found".to_string())?;
    db.set_active_home(home.id).map_err(to_error)?;
    db.update_home_last_seen(home.id).map_err(to_error)?;
    Ok(home)
}

#[tauri::command]
fn homes_delete(state: State<DesktopState>, id: i64) -> Result<serde_json::Value, String> {
    let mut db = open_db(&state)?;
    let active = require_active_home(&mut db)?;
    if active.id == id {
        let homes = db.list_homes().map_err(to_error)?;
        let replacement = homes
            .into_iter()
            .find(|home| home.id != id)
            .ok_or_else(|| "cannot delete the last home".to_string())?;
        db.set_active_home(replacement.id).map_err(to_error)?;
    }
    db.delete_home(id).map_err(to_error)?;
    Ok(serde_json::json!({ "deleted": id }))
}

#[tauri::command]
fn homes_clear_data(state: State<DesktopState>, id: i64) -> Result<serde_json::Value, String> {
    let mut db = open_db(&state)?;
    db.get_home_by_id(id)
        .map_err(to_error)?
        .ok_or_else(|| "home not found".to_string())?;
    db.clear_home_data(id).map_err(to_error)?;
    Ok(serde_json::json!({ "cleared": id }))
}

fn resolve_range(
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<TimeRange, String> {
    tracker_app::resolve_range(&RangeParams { range, start, end })
}

fn open_db(state: &DesktopState) -> Result<Db, String> {
    state.app_state.open_db().map_err(to_error)
}

fn require_active_home(db: &mut Db) -> Result<CodexHome, String> {
    db.ensure_active_home().map_err(to_error)
}

fn to_error(err: impl std::fmt::Display) -> String {
    err.to_string()
}

fn boxed_err(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.into()))
}

fn migrate_legacy_storage(
    app_data_dir: &Path,
    db_path: &Path,
    pricing_defaults_path: &Path,
) -> Result<Option<PathBuf>, String> {
    if db_path.exists() {
        return Ok(None);
    }
    let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
    else {
        return Ok(None);
    };
    let legacy_db = exe_dir.join("codex-tracker.sqlite");
    let legacy_pricing = exe_dir.join("codex-tracker-pricing.json");
    if !legacy_db.exists() && !legacy_pricing.exists() {
        return Ok(None);
    }
    let backup_dir = app_data_dir.join(format!(
        "legacy-backup-{}",
        Utc::now().format("%Y%m%d%H%M%S")
    ));
    fs::create_dir_all(&backup_dir).map_err(|err| format!("create backup: {}", err))?;
    if legacy_db.exists() {
        fs::copy(&legacy_db, backup_dir.join("codex-tracker.sqlite"))
            .map_err(|err| format!("backup legacy db: {}", err))?;
        fs::copy(&legacy_db, db_path).map_err(|err| format!("migrate legacy db: {}", err))?;
    }
    if legacy_pricing.exists() && !pricing_defaults_path.exists() {
        fs::copy(
            &legacy_pricing,
            backup_dir.join("codex-tracker-pricing.json"),
        )
        .map_err(|err| format!("backup legacy pricing: {}", err))?;
        fs::copy(&legacy_pricing, pricing_defaults_path)
            .map_err(|err| format!("migrate legacy pricing: {}", err))?;
    }
    Ok(Some(backup_dir))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let db_path = app
                .path()
                .resolve("codex-tracker.sqlite", tauri::path::BaseDirectory::AppData)
                .map_err(|err| boxed_err(format!("resolve db path: {}", err)))?;
            let pricing_defaults_path = app
                .path()
                .resolve(
                    "codex-tracker-pricing.json",
                    tauri::path::BaseDirectory::AppData,
                )
                .map_err(|err| boxed_err(format!("resolve pricing path: {}", err)))?;
            let app_data_dir = db_path
                .parent()
                .ok_or_else(|| boxed_err("failed to resolve app data dir"))?
                .to_path_buf();
            fs::create_dir_all(&app_data_dir)
                .map_err(|err| boxed_err(format!("create app data dir: {}", err)))?;
            let legacy_backup_dir =
                migrate_legacy_storage(&app_data_dir, &db_path, &pricing_defaults_path)
                    .map_err(boxed_err)?;
            let app_state = AppState::new(db_path, pricing_defaults_path);
            let is_fresh_db = app_state.is_fresh_db();
            if let Err(err) = app_state.setup_db() {
                return Err(boxed_err(format!("failed to initialize database: {}", err)));
            }
            if is_fresh_db {
                if let Err(err) = app_state.apply_pricing_defaults() {
                    eprintln!("failed to apply pricing defaults: {}", err);
                }
            }
            if let Err(err) = app_state.sync_pricing_defaults() {
                eprintln!("failed to sync pricing defaults: {}", err);
            }
            if let Err(err) = app_state.refresh_data() {
                eprintln!("failed to refresh data on startup: {}", err);
            }
            app.manage(DesktopState {
                app_state,
                app_data_dir,
                legacy_backup_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            summary,
            context_latest,
            context_sessions,
            context_stats,
            timeseries,
            breakdown,
            breakdown_tokens,
            breakdown_costs,
            breakdown_effort_tokens,
            breakdown_effort_costs,
            events,
            limits_latest,
            limits_current,
            limits_7d_windows,
            ingest,
            pricing_list,
            pricing_replace,
            pricing_recompute,
            settings_get,
            settings_put,
            homes_list,
            homes_create,
            homes_set_active,
            homes_delete,
            homes_clear_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
