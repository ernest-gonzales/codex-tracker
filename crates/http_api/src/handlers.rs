use std::process::Command;

use axum::{
    body::Body,
    extract::{Json, State},
    http::{Method, Request, StatusCode},
    response::{IntoResponse, Response},
};

use app_api::{
    ContextSessionsRequest, EventsRequest, HomesClearDataRequest, HomesCreateRequest,
    HomesDeleteRequest, HomesSetActiveRequest, LimitsWindowsRequest, PricingReplaceRequest,
    RangeRequest, SettingsPutRequest, TimeseriesRequest,
};

use crate::{assets, errors::HttpError, state::HttpState};

pub async fn summary(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::summary(&state.context, req)?;
    Ok(Json(response))
}

pub async fn context_latest(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::context_latest(&state.context)?;
    Ok(Json(response))
}

pub async fn context_sessions(
    State(state): State<HttpState>,
    Json(req): Json<ContextSessionsRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::context_sessions(&state.context, req)?;
    Ok(Json(response))
}

pub async fn context_stats(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::context_stats(&state.context, req)?;
    Ok(Json(response))
}

pub async fn timeseries(
    State(state): State<HttpState>,
    Json(req): Json<TimeseriesRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::timeseries(&state.context, req)?;
    Ok(Json(response))
}

pub async fn breakdown(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::breakdown(&state.context, req)?;
    Ok(Json(response))
}

pub async fn breakdown_tokens(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::breakdown_tokens(&state.context, req)?;
    Ok(Json(response))
}

pub async fn breakdown_costs(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::breakdown_costs(&state.context, req)?;
    Ok(Json(response))
}

pub async fn breakdown_effort_tokens(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::breakdown_effort_tokens(&state.context, req)?;
    Ok(Json(response))
}

pub async fn breakdown_effort_costs(
    State(state): State<HttpState>,
    Json(req): Json<RangeRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::breakdown_effort_costs(&state.context, req)?;
    Ok(Json(response))
}

pub async fn events(
    State(state): State<HttpState>,
    Json(req): Json<EventsRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::events(&state.context, req)?;
    Ok(Json(response))
}

pub async fn limits_latest(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::limits_latest(&state.context)?;
    Ok(Json(response))
}

pub async fn limits_current(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::limits_current(&state.context)?;
    Ok(Json(response))
}

pub async fn limits_7d_windows(
    State(state): State<HttpState>,
    Json(req): Json<LimitsWindowsRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::limits_7d_windows(&state.context, req)?;
    Ok(Json(response))
}

pub async fn ingest(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let context = state.context.clone();
    let stats = tokio::task::spawn_blocking(move || app_api::ingest(&context))
        .await
        .map_err(|err| {
            HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string(), None)
        })??;
    Ok(Json(stats))
}

pub async fn open_logs_dir(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let path = app_api::logs_dir(&state.context)?;
    open_path(&path)?;
    Ok(Json(app_api::ok()))
}

pub async fn pricing_list(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::pricing_list(&state.context)?;
    Ok(Json(response))
}

pub async fn pricing_replace(
    State(state): State<HttpState>,
    Json(req): Json<PricingReplaceRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::pricing_replace(&state.context, req)?;
    Ok(Json(response))
}

pub async fn pricing_recompute(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::pricing_recompute(&state.context)?;
    Ok(Json(response))
}

pub async fn settings_get(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::settings_get(&state.context)?;
    Ok(Json(response))
}

pub async fn settings_put(
    State(state): State<HttpState>,
    Json(req): Json<SettingsPutRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::settings_put(&state.context, req)?;
    Ok(Json(response))
}

pub async fn homes_list(
    State(state): State<HttpState>,
    Json(_): Json<app_api::EmptyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::homes_list(&state.context)?;
    Ok(Json(response))
}

pub async fn homes_create(
    State(state): State<HttpState>,
    Json(req): Json<HomesCreateRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::homes_create(&state.context, req)?;
    Ok(Json(response))
}

pub async fn homes_set_active(
    State(state): State<HttpState>,
    Json(req): Json<HomesSetActiveRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::homes_set_active(&state.context, req)?;
    Ok(Json(response))
}

pub async fn homes_delete(
    State(state): State<HttpState>,
    Json(req): Json<HomesDeleteRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::homes_delete(&state.context, req)?;
    Ok(Json(response))
}

pub async fn homes_clear_data(
    State(state): State<HttpState>,
    Json(req): Json<HomesClearDataRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = app_api::homes_clear_data(&state.context, req)?;
    Ok(Json(response))
}

pub async fn ui_fallback(
    State(state): State<HttpState>,
    req: Request<Body>,
) -> Result<Response, HttpError> {
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return Err(HttpError::new(
            StatusCode::METHOD_NOT_ALLOWED,
            "method not allowed",
            None,
        ));
    }

    let path = req.uri().path().trim_start_matches('/');
    if path.is_empty() {
        return render_index(&state.csrf_token);
    }

    if let Some(asset) = assets::asset(path) {
        return Ok(asset_response(asset));
    }

    if !path.contains('.') {
        return render_index(&state.csrf_token);
    }

    Err(HttpError::new(
        StatusCode::NOT_FOUND,
        "not found",
        Some("not_found".to_string()),
    ))
}

fn render_index(csrf_token: &str) -> Result<Response, HttpError> {
    let index = assets::index_asset().ok_or_else(|| {
        HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "missing index.html",
            None,
        )
    })?;
    let html = std::str::from_utf8(index.bytes).map_err(|_| {
        HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid index.html encoding",
            None,
        )
    })?;
    let injected = inject_csrf(html, csrf_token);
    let mut response = Response::new(Body::from(injected));
    response
        .headers_mut()
        .insert("content-type", index.mime.parse().unwrap());
    Ok(response)
}

fn inject_csrf(html: &str, csrf_token: &str) -> String {
    let snippet = format!(
        "<script>window.__CODEX_TRACKER_CSRF__=\"{}\";</script>",
        csrf_token
    );
    if html.contains("</head>") {
        html.replacen("</head>", &format!("{snippet}</head>"), 1)
    } else {
        format!("{html}{snippet}")
    }
}

fn asset_response(asset: &assets::EmbeddedAsset) -> Response {
    let mut response = Response::new(Body::from(asset.bytes));
    response
        .headers_mut()
        .insert("content-type", asset.mime.parse().unwrap());
    response
}

fn open_path(path: &std::path::Path) -> Result<(), HttpError> {
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|err| HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string(), None))?;
    if status.success() {
        Ok(())
    } else {
        Err(HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to open path",
            None,
        ))
    }
}
