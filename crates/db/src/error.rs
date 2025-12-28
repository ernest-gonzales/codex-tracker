#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("time parse error: {0}")]
    TimeParse(#[from] chrono::ParseError),
}

pub type Result<T> = std::result::Result<T, DbError>;
