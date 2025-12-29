mod assets;
mod errors;
mod handlers;
mod middleware;
mod state;

use axum::{Router, middleware as axum_middleware, routing::post};

pub use state::{HttpState, generate_csrf_token};

pub fn router(state: HttpState) -> Router<()> {
    let api = Router::new()
        .route("/summary", post(handlers::summary))
        .route("/context_latest", post(handlers::context_latest))
        .route("/context_sessions", post(handlers::context_sessions))
        .route("/context_stats", post(handlers::context_stats))
        .route("/timeseries", post(handlers::timeseries))
        .route("/breakdown", post(handlers::breakdown))
        .route("/breakdown_tokens", post(handlers::breakdown_tokens))
        .route("/breakdown_costs", post(handlers::breakdown_costs))
        .route(
            "/breakdown_effort_tokens",
            post(handlers::breakdown_effort_tokens),
        )
        .route(
            "/breakdown_effort_costs",
            post(handlers::breakdown_effort_costs),
        )
        .route("/events", post(handlers::events))
        .route("/limits_latest", post(handlers::limits_latest))
        .route("/limits_current", post(handlers::limits_current))
        .route("/limits_7d_windows", post(handlers::limits_7d_windows))
        .route("/ingest", post(handlers::ingest))
        .route("/open_logs_dir", post(handlers::open_logs_dir))
        .route("/pricing_list", post(handlers::pricing_list))
        .route("/pricing_replace", post(handlers::pricing_replace))
        .route("/pricing_recompute", post(handlers::pricing_recompute))
        .route("/settings_get", post(handlers::settings_get))
        .route("/settings_put", post(handlers::settings_put))
        .route("/homes_list", post(handlers::homes_list))
        .route("/homes_create", post(handlers::homes_create))
        .route("/homes_set_active", post(handlers::homes_set_active))
        .route("/homes_delete", post(handlers::homes_delete))
        .route("/homes_clear_data", post(handlers::homes_clear_data))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_csrf,
        ));

    Router::new()
        .nest("/api", api)
        .fallback(handlers::ui_fallback)
        .with_state(state)
}

#[cfg(test)]
mod tests;
