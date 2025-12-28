mod analytics;
mod homes;
mod ingest;
mod limits;
mod pricing;
mod settings;

use std::sync::Arc;

use crate::app::AppConfig;
use crate::error::{AppError, Result};
use tracker_core::CodexHome;
use tracker_db::Db;

pub use analytics::AnalyticsService;
pub use homes::HomesService;
pub use ingest::IngestService;
pub use limits::LimitsService;
pub use pricing::PricingService;
pub use settings::{SettingsService, SettingsSnapshot};

type SharedConfig = Arc<AppConfig>;

/// Service registry for app-level operations.
#[derive(Clone)]
pub struct AppServices {
    pub analytics: AnalyticsService,
    pub ingest: IngestService,
    pub limits: LimitsService,
    pub pricing: PricingService,
    pub homes: HomesService,
    pub settings: SettingsService,
}

impl AppServices {
    pub fn new(config: &AppConfig) -> Self {
        let shared = Arc::new(config.clone());
        Self {
            analytics: AnalyticsService::new(shared.clone()),
            ingest: IngestService::new(shared.clone()),
            limits: LimitsService::new(shared.clone()),
            pricing: PricingService::new(shared.clone()),
            homes: HomesService::new(shared.clone()),
            settings: SettingsService::new(shared),
        }
    }
}

fn open_db(config: &SharedConfig) -> Result<Db> {
    Ok(Db::open(&config.db_path)?)
}

fn require_active_home(db: &mut Db) -> Result<CodexHome> {
    Ok(db.ensure_active_home()?)
}

fn missing_home() -> AppError {
    AppError::NotFound("home not found".to_string())
}
