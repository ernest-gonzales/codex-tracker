use std::path::{Path, PathBuf};

use chrono::Utc;
use tauri::{Emitter, Manager};
use tracker_app::AppState;

use crate::app::DesktopState;

pub fn initialize(app: &tauri::App) -> Result<DesktopState, Box<dyn std::error::Error>> {
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
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|err| boxed_err(format!("create app data dir: {}", err)))?;
    let legacy_backup_dir = migrate_legacy_storage(&app_data_dir, &db_path, &pricing_defaults_path)
        .map_err(boxed_err)?;
    let app_state = AppState::new(db_path, pricing_defaults_path);
    let is_fresh_db = app_state.is_fresh_db();
    if let Err(err) = app_state.setup_db() {
        return Err(boxed_err(format!("failed to initialize database: {}", err)));
    }
    if is_fresh_db && let Err(err) = app_state.apply_pricing_defaults() {
        eprintln!("failed to apply pricing defaults: {}", err);
    }
    if let Err(err) = app_state.sync_pricing_defaults() {
        eprintln!("failed to sync pricing defaults: {}", err);
    }
    let refresh_state = app_state.clone();
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = refresh_state.services.ingest.run();
        match result {
            Ok(stats) => {
                if let Err(err) = app_handle.emit("ingest:complete", stats) {
                    eprintln!("failed to emit ingest complete: {}", err);
                }
            }
            Err(err) => {
                eprintln!("failed to refresh data on startup: {}", err);
            }
        }
    });
    Ok(DesktopState {
        app_state,
        app_data_dir,
        legacy_backup_dir,
    })
}

fn boxed_err(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
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
    std::fs::create_dir_all(&backup_dir).map_err(|err| format!("create backup: {}", err))?;
    if legacy_db.exists() {
        std::fs::copy(&legacy_db, backup_dir.join("codex-tracker.sqlite"))
            .map_err(|err| format!("backup legacy db: {}", err))?;
        std::fs::copy(&legacy_db, db_path).map_err(|err| format!("migrate legacy db: {}", err))?;
    }
    if legacy_pricing.exists() && !pricing_defaults_path.exists() {
        std::fs::copy(
            &legacy_pricing,
            backup_dir.join("codex-tracker-pricing.json"),
        )
        .map_err(|err| format!("backup legacy pricing: {}", err))?;
        std::fs::copy(&legacy_pricing, pricing_defaults_path)
            .map_err(|err| format!("migrate legacy pricing: {}", err))?;
    }
    Ok(Some(backup_dir))
}
