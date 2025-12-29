use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = "codex-tracker";
const CONFIG_FILE_NAME: &str = "config.toml";
const DEFAULT_PORT: u16 = 3845;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub port: u16,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self { port: DEFAULT_PORT }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ConfigLoad {
    pub config: CliConfig,
    pub paths: ConfigPaths,
    pub created: bool,
}

pub fn load_or_create() -> Result<ConfigLoad, String> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("create config dir {}: {}", dir.display(), err))?;
    let file = dir.join(CONFIG_FILE_NAME);
    let paths = ConfigPaths { file };

    if paths.file.exists() {
        let contents = fs::read_to_string(&paths.file)
            .map_err(|err| format!("read config {}: {}", paths.file.display(), err))?;
        let config: CliConfig = toml::from_str(&contents)
            .map_err(|err| format!("parse config {}: {}", paths.file.display(), err))?;
        return Ok(ConfigLoad {
            config,
            paths,
            created: false,
        });
    }

    let config = CliConfig::default();
    let contents =
        toml::to_string_pretty(&config).map_err(|err| format!("serialize config: {}", err))?;
    fs::write(&paths.file, contents)
        .map_err(|err| format!("write config {}: {}", paths.file.display(), err))?;

    Ok(ConfigLoad {
        config,
        paths,
        created: true,
    })
}

fn config_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|err| format!("resolve HOME: {}", err))?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join(CONFIG_DIR_NAME))
}
