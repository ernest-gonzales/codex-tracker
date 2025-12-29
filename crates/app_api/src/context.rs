use std::path::PathBuf;

use tracker_app::AppState;

#[derive(Clone)]
pub struct AppContext {
    pub app_state: AppState,
    pub app_data_dir: PathBuf,
    pub legacy_backup_dir: Option<PathBuf>,
}
