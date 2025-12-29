use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracker_app::{ApiError, AppError};

#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    body: ApiError,
}

impl HttpError {
    pub fn new(status: StatusCode, message: impl Into<String>, code: Option<String>) -> Self {
        let body = ApiError {
            status: status.as_u16(),
            message: message.into(),
            code,
        };
        Self { status, body }
    }
}

impl From<AppError> for HttpError {
    fn from(err: AppError) -> Self {
        let api_error = ApiError::from(err);
        let status =
            StatusCode::from_u16(api_error.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: api_error,
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
