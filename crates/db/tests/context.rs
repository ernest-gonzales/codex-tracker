mod support;

use support::{insert_events, make_event, setup_db, setup_home};
use tracker_core::{ContextStatus, TimeRange, UsageTotals};

#[test]
fn context_pressure_stats_averages_known_context_only() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    let mut event1 = make_event(
        "e1",
        "2025-12-19T10:00:00Z",
        "gpt-5.2",
        UsageTotals {
            input_tokens: 500,
            cached_input_tokens: 0,
            output_tokens: 500,
            reasoning_output_tokens: 0,
            total_tokens: 1000,
        },
        "source-a",
    );
    event1.context = ContextStatus {
        context_used: 1000,
        context_window: 2000,
    };
    let mut event2 = make_event(
        "e2",
        "2025-12-19T10:05:00Z",
        "gpt-5.2",
        UsageTotals {
            input_tokens: 300,
            cached_input_tokens: 0,
            output_tokens: 200,
            reasoning_output_tokens: 0,
            total_tokens: 500,
        },
        "source-a",
    );
    event2.context = ContextStatus {
        context_used: 500,
        context_window: 1000,
    };
    let mut event3 = make_event(
        "e3",
        "2025-12-19T10:06:00Z",
        "gpt-5.2",
        UsageTotals {
            input_tokens: 100,
            cached_input_tokens: 0,
            output_tokens: 200,
            reasoning_output_tokens: 0,
            total_tokens: 300,
        },
        "source-a",
    );
    event3.context = ContextStatus {
        context_used: 300,
        context_window: 0,
    };
    let mut event4 = make_event(
        "e4",
        "2025-12-20T10:06:00Z",
        "gpt-5.2",
        UsageTotals {
            input_tokens: 200,
            cached_input_tokens: 0,
            output_tokens: 100,
            reasoning_output_tokens: 0,
            total_tokens: 300,
        },
        "source-a",
    );
    event4.context = ContextStatus {
        context_used: 400,
        context_window: 800,
    };
    insert_events(db, home.id, vec![event1, event2, event3, event4]);

    let range = TimeRange {
        start: "2025-12-19T09:00:00Z".to_string(),
        end: "2025-12-19T12:00:00Z".to_string(),
    };
    let stats = db.context_pressure_stats(&range, home.id).expect("stats");

    assert_eq!(stats.sample_count, 2);
    assert!((stats.avg_context_used.unwrap() - 750.0).abs() < 1e-6);
    assert!((stats.avg_context_window.unwrap() - 1500.0).abs() < 1e-6);
    assert!((stats.avg_pressure_pct.unwrap() - 50.0).abs() < 1e-6);
}

#[test]
fn active_sessions_returns_latest_per_session() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_events(
        db,
        home.id,
        vec![
            make_event(
                "e1",
                "2025-12-19T19:00:00Z",
                "gpt-5.2",
                UsageTotals {
                    input_tokens: 10,
                    cached_input_tokens: 0,
                    output_tokens: 2,
                    reasoning_output_tokens: 0,
                    total_tokens: 12,
                },
                "/tmp/rollout-2025-12-19T19-00-00Z-sessiona.jsonl",
            ),
            make_event(
                "e2",
                "2025-12-19T19:05:00Z",
                "gpt-5.2",
                UsageTotals {
                    input_tokens: 12,
                    cached_input_tokens: 0,
                    output_tokens: 3,
                    reasoning_output_tokens: 0,
                    total_tokens: 15,
                },
                "/tmp/rollout-2025-12-19T19-00-00Z-sessiona.jsonl",
            ),
            make_event(
                "e3",
                "2025-12-19T19:03:00Z",
                "gpt-4.1",
                UsageTotals {
                    input_tokens: 5,
                    cached_input_tokens: 0,
                    output_tokens: 1,
                    reasoning_output_tokens: 0,
                    total_tokens: 6,
                },
                "/tmp/rollout-2025-12-19T19-03-00Z-sessionb.jsonl",
            ),
        ],
    );

    let sessions = db
        .active_sessions(home.id, "2025-12-19T18:00:00Z")
        .expect("sessions");
    assert_eq!(sessions.len(), 2);
    let session_a = sessions
        .iter()
        .find(|session| session.session_id == "sessiona")
        .expect("session a");
    assert_eq!(session_a.last_seen, "2025-12-19T19:05:00Z");
    assert_eq!(session_a.session_start, "2025-12-19T19:00:00Z");
}
