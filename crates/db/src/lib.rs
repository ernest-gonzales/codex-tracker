mod analytics;
mod breakdowns;
mod context;
mod error;
mod helpers;
mod homes;
mod ingest;
mod limits;
mod migrations;
mod pricing;
mod settings;
mod types;
mod usage_rows;

use std::path::Path;

use rusqlite::Connection;

pub use error::{DbError, Result};
pub use types::{Bucket, IngestCursor, Metric, RowUsage};

/// SQLite-backed repository for tracker data.
pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "cache_size", -20_000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn })
    }
}
