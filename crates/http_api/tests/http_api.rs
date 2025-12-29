use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::util::ServiceExt;

use app_api::AppContext;
use tracker_app::{AppPaths, AppState, ensure_app_data_dir};

use http_api::HttpState;

const TEST_TOKEN: &str = "testtoken";

struct TestApp {
    _temp_dir: tempfile::TempDir,
    router: axum::Router,
}

fn build_app() -> TestApp {
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
    let state = HttpState::new(context, TEST_TOKEN.to_string());
    let router = http_api::router(state);

    TestApp {
        _temp_dir: temp_dir,
        router,
    }
}

#[tokio::test]
async fn serves_index_and_injects_token() {
    let app = build_app();

    let response = app
        .router
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
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("text/html"));

    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body_text = String::from_utf8_lossy(&body);
    assert!(body_text.contains("__CODEX_TRACKER_CSRF__"));
    assert!(body_text.contains(TEST_TOKEN));
}

#[tokio::test]
async fn api_rejects_missing_csrf() {
    let app = build_app();

    let response = app
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/summary")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(payload["code"], "csrf_invalid");
}

#[tokio::test]
async fn api_allows_valid_csrf() {
    let app = build_app();

    let response = app
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings_get")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-codex-token", TEST_TOKEN)
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let payload: Value = serde_json::from_slice(&body).expect("json body");
    assert!(payload.get("db_path").is_some());
    assert!(payload.get("app_data_dir").is_some());
}
