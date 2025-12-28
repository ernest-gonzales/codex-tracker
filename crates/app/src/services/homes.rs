use crate::error::{AppError, Result};
use crate::services::{SharedConfig, missing_home, open_db, require_active_home};
use tracker_core::CodexHome;
use tracker_db::Db;

#[derive(Clone)]
pub struct HomesService {
    config: SharedConfig,
}

impl HomesService {
    pub(super) fn new(config: SharedConfig) -> Self {
        Self { config }
    }

    fn db(&self) -> Result<Db> {
        open_db(&self.config)
    }

    pub fn list(&self) -> Result<Vec<CodexHome>> {
        let db = self.db()?;
        Ok(db.list_homes()?)
    }

    pub fn active(&self) -> Result<CodexHome> {
        let mut db = self.db()?;
        require_active_home(&mut db)
    }

    pub fn create(&self, path: &str, label: Option<&str>) -> Result<CodexHome> {
        let db = self.db()?;
        let home = db.get_or_create_home(path, label)?;
        db.set_active_home(home.id)?;
        db.update_home_last_seen(home.id)?;
        Ok(home)
    }

    pub fn set_active(&self, id: i64) -> Result<CodexHome> {
        let db = self.db()?;
        let home = db.get_home_by_id(id)?.ok_or_else(missing_home)?;
        db.set_active_home(home.id)?;
        db.update_home_last_seen(home.id)?;
        Ok(home)
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let mut db = self.db()?;
        let active = require_active_home(&mut db)?;
        if active.id == id {
            let homes = db.list_homes()?;
            let replacement = homes
                .into_iter()
                .find(|home| home.id != id)
                .ok_or_else(|| AppError::InvalidInput("cannot delete the last home".to_string()))?;
            db.set_active_home(replacement.id)?;
        }
        Ok(db.delete_home(id)?)
    }

    pub fn clear_data(&self, id: i64) -> Result<()> {
        let mut db = self.db()?;
        db.get_home_by_id(id)?.ok_or_else(missing_home)?;
        Ok(db.clear_home_data(id)?)
    }
}
