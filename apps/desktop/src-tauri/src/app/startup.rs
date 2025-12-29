use tauri::{Emitter, Manager};
use tracker_app::{AppPaths, AppState, ensure_app_data_dir, migrate_legacy_storage};

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
    let paths = AppPaths {
        app_data_dir: app_data_dir.clone(),
        db_path,
        pricing_defaults_path,
    };
    ensure_app_data_dir(&paths)
        .map_err(|err| boxed_err(format!("create app data dir: {}", err)))?;
    let legacy_backup_dir =
        migrate_legacy_storage(&paths).map_err(|err| boxed_err(err.to_string()))?;
    let app_state = AppState::new(paths.db_path, paths.pricing_defaults_path);
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
        app_data_dir: paths.app_data_dir,
        legacy_backup_dir,
    })
}

fn boxed_err(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}
