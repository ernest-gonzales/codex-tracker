use std::path::Path;

use crate::error::Result;
use crate::services::{open_db, require_active_home, SharedConfig};
use ingest::IngestStats;
use tracker_db::Db;

#[derive(Clone)]
pub struct IngestService {
    config: SharedConfig,
}

impl IngestService {
    pub(super) fn new(config: SharedConfig) -> Self {
        Self { config }
    }

    fn db(&self) -> Result<Db> {
        open_db(&self.config)
    }

    pub fn run(&self) -> Result<IngestStats> {
        let mut db = self.db()?;
        let home = require_active_home(&mut db)?;
        Ok(ingest::ingest_codex_home(&mut db, Path::new(&home.path))?)
    }
}
