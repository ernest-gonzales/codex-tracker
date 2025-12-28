use serde::Serialize;
use std::io;

/// Total/last token counts extracted from a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenTotals {
    pub total_tokens: u64,
    pub last_tokens: u64,
}

/// Ingest summary returned after scanning Codex logs.
#[derive(Debug, Clone, Default, Serialize)]
pub struct IngestStats {
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub events_inserted: usize,
    pub bytes_read: u64,
    pub issues: Vec<IngestIssue>,
}

/// Non-fatal issues encountered during ingest.
#[derive(Debug, Clone, Serialize)]
pub struct IngestIssue {
    pub file_path: String,
    pub message: String,
}

/// Errors emitted by the ingest pipeline.
#[derive(Debug)]
pub enum IngestError {
    Io(io::Error),
    Db(tracker_db::DbError),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {}", err),
            Self::Db(err) => write!(f, "db error: {}", err),
        }
    }
}

impl std::error::Error for IngestError {}

impl From<io::Error> for IngestError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<tracker_db::DbError> for IngestError {
    fn from(err: tracker_db::DbError) -> Self {
        Self::Db(err)
    }
}

pub type Result<T> = std::result::Result<T, IngestError>;
