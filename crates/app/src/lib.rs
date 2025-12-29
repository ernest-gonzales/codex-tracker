pub mod app;
pub mod config;
pub mod error;
pub mod pricing;
pub mod services;
pub mod startup;
pub mod util;

pub use app::{AppConfig, AppState};
pub use config::RangeParams;
pub use error::{ApiError, AppError, Result};
pub use pricing::{
    apply_pricing_defaults, load_initial_pricing, load_pricing_defaults, sync_pricing_defaults,
    write_pricing_defaults,
};
pub use services::{AppServices, SettingsSnapshot};
pub use startup::{AppPaths, ensure_app_data_dir, migrate_legacy_storage};
pub use util::time::{normalize_rfc3339_to_utc, resolve_range};
