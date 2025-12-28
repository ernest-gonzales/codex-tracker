use std::path::PathBuf;

use crate::error::{AppError, Result};
use crate::pricing;
use crate::services::AppServices;
use tracker_db::Db;

/// Paths and files needed to run the local tracker.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub db_path: PathBuf,
    pub pricing_defaults_path: PathBuf,
}

/// Application state shared by frontend backends (desktop, CLI).
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub services: AppServices,
}

impl AppState {
    pub fn new(db_path: PathBuf, pricing_defaults_path: PathBuf) -> Self {
        let config = AppConfig {
            db_path,
            pricing_defaults_path,
        };
        let services = AppServices::new(&config);
        Self { config, services }
    }

    pub fn is_fresh_db(&self) -> bool {
        !self.config.db_path.exists()
    }

    pub fn setup_db(&self) -> Result<()> {
        setup_db(&self.config.db_path)
    }

    pub fn initialize(&self) -> Result<()> {
        let is_fresh_db = self.is_fresh_db();
        self.setup_db()
            .map_err(|err| AppError::Message(format!("initialize db: {}", err)))?;
        if is_fresh_db {
            self.apply_pricing_defaults()?;
        }
        self.sync_pricing_defaults()?;
        self.refresh_data()?;
        Ok(())
    }

    pub fn open_db(&self) -> Result<Db> {
        Ok(Db::open(&self.config.db_path)?)
    }

    pub fn apply_pricing_defaults(&self) -> Result<()> {
        pricing::apply_pricing_defaults(&self.config.db_path, &self.config.pricing_defaults_path)
    }

    pub fn sync_pricing_defaults(&self) -> Result<()> {
        pricing::sync_pricing_defaults(&self.config.db_path, &self.config.pricing_defaults_path)
    }

    pub fn refresh_data(&self) -> Result<()> {
        self.services.ingest.run().map(|_| ())
    }

    pub fn write_pricing_defaults(&self, rules: &[tracker_core::PricingRuleInput]) -> Result<()> {
        pricing::write_pricing_defaults(&self.config.pricing_defaults_path, rules)
    }
}

pub fn setup_db(path: &std::path::Path) -> Result<()> {
    let mut db = Db::open(path)?;
    db.migrate()?;
    Ok(())
}
