use crate::error::Result;
use crate::services::{open_db, require_active_home, SharedConfig};
use tracker_core::{UsageLimitCurrentResponse, UsageLimitSnapshot, UsageLimitWindow};
use tracker_db::Db;

#[derive(Clone)]
pub struct LimitsService {
    config: SharedConfig,
}

impl LimitsService {
    pub(super) fn new(config: SharedConfig) -> Self {
        Self { config }
    }

    fn db(&self) -> Result<Db> {
        open_db(&self.config)
    }

    pub fn latest(&self) -> Result<(Option<UsageLimitSnapshot>, Option<UsageLimitSnapshot>)> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        let primary = db.latest_limit_snapshot_current(home.id, "5h")?;
        let secondary = db.latest_limit_snapshot_current(home.id, "7d")?;
        Ok((primary, secondary))
    }

    pub fn current(&self) -> Result<UsageLimitCurrentResponse> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        let primary = db.limit_current_window(home.id, "5h")?;
        let secondary = db.limit_current_window(home.id, "7d")?;
        Ok(UsageLimitCurrentResponse { primary, secondary })
    }

    pub fn windows_7d(&self, limit: usize) -> Result<Vec<UsageLimitWindow>> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(db.limit_windows_7d(home.id, limit)?)
    }
}
