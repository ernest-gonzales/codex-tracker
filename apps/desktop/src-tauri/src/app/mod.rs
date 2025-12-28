use std::path::PathBuf;

use tracker_app::AppState;

pub mod startup;

#[derive(Clone)]
pub struct DesktopState {
    pub app_state: AppState,
    pub app_data_dir: PathBuf,
    pub legacy_backup_dir: Option<PathBuf>,
}

pub fn expand_home_path(path: &str) -> PathBuf {
    if path == "~"
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home);
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}
