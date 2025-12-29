use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::{AppError, Result};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub app_data_dir: PathBuf,
    pub db_path: PathBuf,
    pub pricing_defaults_path: PathBuf,
}

impl AppPaths {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let db_path = app_data_dir.join("codex-tracker.sqlite");
        let pricing_defaults_path = app_data_dir.join("codex-tracker-pricing.json");
        Self {
            app_data_dir,
            db_path,
            pricing_defaults_path,
        }
    }
}

pub fn ensure_app_data_dir(paths: &AppPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.app_data_dir)?;
    Ok(())
}

pub fn migrate_legacy_storage(paths: &AppPaths) -> Result<Option<PathBuf>> {
    migrate_legacy_storage_paths(
        &paths.app_data_dir,
        &paths.db_path,
        &paths.pricing_defaults_path,
    )
}

fn migrate_legacy_storage_paths(
    app_data_dir: &Path,
    db_path: &Path,
    pricing_defaults_path: &Path,
) -> Result<Option<PathBuf>> {
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
    std::fs::create_dir_all(&backup_dir)?;
    if legacy_db.exists() {
        std::fs::copy(&legacy_db, backup_dir.join("codex-tracker.sqlite"))
            .map_err(|err| AppError::Message(format!("backup legacy db: {}", err)))?;
        std::fs::copy(&legacy_db, db_path)
            .map_err(|err| AppError::Message(format!("migrate legacy db: {}", err)))?;
    }
    if legacy_pricing.exists() && !pricing_defaults_path.exists() {
        std::fs::copy(
            &legacy_pricing,
            backup_dir.join("codex-tracker-pricing.json"),
        )
        .map_err(|err| AppError::Message(format!("backup legacy pricing: {}", err)))?;
        std::fs::copy(&legacy_pricing, pricing_defaults_path)
            .map_err(|err| AppError::Message(format!("migrate legacy pricing: {}", err)))?;
    }
    Ok(Some(backup_dir))
}
