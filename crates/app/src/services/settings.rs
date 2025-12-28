use crate::error::Result;
use crate::services::{open_db, require_active_home, SharedConfig};
use tracker_db::Db;

/// Snapshot of user-configurable settings stored in the DB.
#[derive(Debug, Clone)]
pub struct SettingsSnapshot {
    pub codex_home: String,
    pub active_home_id: i64,
    pub context_active_minutes: u32,
}

#[derive(Clone)]
pub struct SettingsService {
    config: SharedConfig,
}

impl SettingsService {
    pub(super) fn new(config: SharedConfig) -> Self {
        Self { config }
    }

    fn db(&self) -> Result<Db> {
        open_db(&self.config)
    }

    pub fn get(&self) -> Result<SettingsSnapshot> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        let context_active_minutes = db.get_context_active_minutes()?;
        Ok(SettingsSnapshot {
            codex_home: home.path,
            active_home_id: home.id,
            context_active_minutes,
        })
    }

    pub fn update(
        &self,
        codex_home: Option<&str>,
        context_active_minutes: Option<u32>,
    ) -> Result<()> {
        let db = self.db()?;
        if let Some(codex_home) = codex_home {
            let home = db.get_or_create_home(codex_home, Some("Default"))?;
            db.set_active_home(home.id)?;
        }
        if let Some(minutes) = context_active_minutes {
            db.set_context_active_minutes(minutes)?;
        }
        Ok(())
    }
}
