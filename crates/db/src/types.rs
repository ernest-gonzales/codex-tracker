use tracker_core::UsageTotals;

#[derive(Debug, Clone, Copy)]
pub enum Bucket {
    Hour,
    Day,
}

#[derive(Debug, Clone, Copy)]
pub enum Metric {
    Tokens,
    Cost,
}

#[derive(Debug, Clone)]
pub struct RowUsage {
    pub id: String,
    pub ts: String,
    pub model: String,
    pub usage: UsageTotals,
    pub cost_usd: Option<f64>,
    pub source: String,
    pub reasoning_effort: Option<String>,
}

/// Cursor metadata for incremental ingest runs.
#[derive(Debug, Clone)]
pub struct IngestCursor {
    pub codex_home_id: i64,
    pub codex_home: String,
    pub file_path: String,
    pub inode: Option<u64>,
    pub mtime: Option<String>,
    pub byte_offset: u64,
    pub last_event_key: Option<String>,
    pub updated_at: String,
    pub last_model: Option<String>,
    pub last_effort: Option<String>,
}
