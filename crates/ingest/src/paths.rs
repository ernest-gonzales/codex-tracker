use std::path::PathBuf;

pub fn default_codex_home() -> PathBuf {
    if let Ok(path) = std::env::var("CODEX_HOME") {
        return PathBuf::from(path);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".codex");
    }
    PathBuf::from(".codex")
}
