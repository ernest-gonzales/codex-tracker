use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("db error: {0}")]
    Db(#[from] tracker_db::DbError),
    #[error("ingest error: {0}")]
    Ingest(#[from] ingest::IngestError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        let (status, code) = match err {
            AppError::InvalidInput(_) => (400, Some("invalid_input".to_string())),
            AppError::NotFound(_) => (404, Some("not_found".to_string())),
            AppError::Db(_)
            | AppError::Ingest(_)
            | AppError::Io(_)
            | AppError::Serde(_)
            | AppError::Message(_) => (500, None),
        };
        Self {
            status,
            message: err.to_string(),
            code,
        }
    }
}
