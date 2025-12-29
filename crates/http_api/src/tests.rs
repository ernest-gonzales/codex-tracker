use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

use app_api::AppContext;
use tracker_app::{AppPaths, AppState, ensure_app_data_dir};

use crate::HttpState;

#[tokio::test]
async fn serves_index() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::new(temp_dir.path().to_path_buf());
    ensure_app_data_dir(&paths).expect("ensure app data dir");
    let app_state = AppState::new(paths.db_path, paths.pricing_defaults_path);
    app_state.setup_db().expect("setup db");

    let context = AppContext {
        app_state,
        app_data_dir: paths.app_data_dir,
        legacy_backup_dir: None,
    };
    let state = HttpState::new(context, "testtoken".to_string());
    let app = crate::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("text/html"));
}
