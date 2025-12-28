use tempfile::tempdir;
use tracker_app::AppState;
use tracker_core::{session_id_from_source, ContextStatus, TimeRange, UsageEvent, UsageTotals};

#[test]
fn summary_service_smoke() {
    let dir = tempdir().expect("temp dir");
    let db_path = dir.path().join("app.sqlite");
    let pricing_path = dir.path().join("pricing.json");
    let app_state = AppState::new(db_path.clone(), pricing_path);
    app_state.setup_db().expect("setup db");

    let mut db = app_state.open_db().expect("open db");
    let home_path = dir.path().to_string_lossy().to_string();
    let home = db
        .get_or_create_home(&home_path, Some("Default"))
        .expect("home");
    db.set_active_home(home.id).expect("active home");

    let usage = UsageTotals {
        input_tokens: 10,
        cached_input_tokens: 0,
        output_tokens: 2,
        reasoning_output_tokens: 0,
        total_tokens: 12,
    };
    let event = UsageEvent {
        id: "e1".to_string(),
        ts: "2025-12-19T10:00:00Z".to_string(),
        model: "gpt-5.2".to_string(),
        usage,
        context: ContextStatus {
            context_used: 12,
            context_window: 100,
        },
        cost_usd: None,
        reasoning_effort: None,
        source: "source-a".to_string(),
        session_id: session_id_from_source("source-a"),
        request_id: None,
        raw_json: None,
    };
    db.insert_usage_events(home.id, &[event])
        .expect("insert events");

    let range = TimeRange {
        start: "2025-12-19T00:00:00Z".to_string(),
        end: "2025-12-20T00:00:00Z".to_string(),
    };
    let summary = app_state
        .services
        .analytics
        .summary(&range)
        .expect("summary");
    assert_eq!(summary.total_tokens, 12);
}
