use std::path::PathBuf;

const DB_FILE_NAME: &str = "codex-tracker.sqlite";

#[derive(Debug, Clone)]
pub struct DataDirResolution {
    pub dir: PathBuf,
    pub matched_existing: bool,
}

pub fn resolve_data_dir() -> Result<DataDirResolution, String> {
    let home = std::env::var("HOME").map_err(|err| format!("resolve HOME: {}", err))?;
    let base = PathBuf::from(home)
        .join("Library")
        .join("Application Support");

    let candidates = [
        base.join("Codex Tracker"),
        base.join("com.codex.tracker"),
        base.join("codex-tracker"),
    ];

    for candidate in candidates {
        if candidate.join(DB_FILE_NAME).exists() {
            return Ok(DataDirResolution {
                dir: candidate,
                matched_existing: true,
            });
        }
    }

    Ok(DataDirResolution {
        dir: base.join("codex-tracker"),
        matched_existing: false,
    })
}
