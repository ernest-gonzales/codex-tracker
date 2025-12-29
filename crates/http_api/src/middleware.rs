use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header::ORIGIN},
    middleware::Next,
    response::Response,
};

use crate::{errors::HttpError, state::HttpState};

pub async fn require_csrf(
    State(state): State<HttpState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, HttpError> {
    if let Some(origin) = req.headers().get(ORIGIN) {
        let origin = origin.to_str().map_err(|_| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "invalid Origin header",
                Some("invalid_origin".to_string()),
            )
        })?;
        if !is_loopback_origin(origin) {
            return Err(HttpError::new(
                StatusCode::FORBIDDEN,
                "invalid origin",
                Some("invalid_origin".to_string()),
            ));
        }
    }

    let token = req
        .headers()
        .get("x-codex-token")
        .and_then(|value| value.to_str().ok());
    if token != Some(state.csrf_token.as_str()) {
        return Err(HttpError::new(
            StatusCode::UNAUTHORIZED,
            "missing or invalid CSRF token",
            Some("csrf_invalid".to_string()),
        ));
    }

    Ok(next.run(req).await)
}

fn is_loopback_origin(origin: &str) -> bool {
    origin.starts_with("http://127.0.0.1:")
        || origin.starts_with("http://localhost:")
        || origin.starts_with("http://[::1]:")
        || origin.starts_with("https://127.0.0.1:")
        || origin.starts_with("https://localhost:")
        || origin.starts_with("https://[::1]:")
}
