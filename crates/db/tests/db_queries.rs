mod support;

use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::Connection;
use support::{
    insert_events, make_event, make_limit_snapshot, make_message_event, setup_db, setup_home,
};
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
fn breakdown_by_model_tokens_handles_resets() {
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
                "gpt-5.1",
                UsageTotals {
                    input_tokens: 700,
                    cached_input_tokens: 200,
                    output_tokens: 300,
                    reasoning_output_tokens: 100,
                    total_tokens: 1000,
                },
                "source-a",
            ),
            make_event(
                "e2",
                "2025-12-19T19:10:00Z",
                "gpt-5.1",
                UsageTotals {
                    input_tokens: 300,
                    cached_input_tokens: 50,
                    output_tokens: 100,
                    reasoning_output_tokens: 20,
                    total_tokens: 400,
                },
                "source-a",
            ),
        ],
    );

    let range = TimeRange {
        start: "2025-12-19T18:40:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let breakdown = db
        .breakdown_by_model_tokens(&range, home.id)
        .expect("breakdown");
    let row = breakdown
        .iter()
        .find(|item| item.model == "gpt-5.1")
        .expect("row");

    assert_eq!(row.input_tokens, 1000);
    assert_eq!(row.cached_input_tokens, 250);
    assert_eq!(row.output_tokens, 400);
    assert_eq!(row.reasoning_output_tokens, 120);
    assert_eq!(row.total_tokens, 1400);
}

#[test]
fn breakdown_by_model_effort_tokens_splits_effort() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    let mut event_a = make_event(
        "e1",
        "2025-12-19T19:00:00Z",
        "gpt-5.2",
        UsageTotals {
            input_tokens: 100,
            cached_input_tokens: 0,
            output_tokens: 20,
            reasoning_output_tokens: 0,
            total_tokens: 120,
        },
        "source-a",
    );
    event_a.reasoning_effort = Some("high".to_string());
    let mut event_b = make_event(
        "e2",
        "2025-12-19T19:10:00Z",
        "gpt-5.2",
        UsageTotals {
            input_tokens: 150,
            cached_input_tokens: 0,
            output_tokens: 30,
            reasoning_output_tokens: 0,
            total_tokens: 180,
        },
        "source-b",
    );
    event_b.reasoning_effort = Some("low".to_string());
    insert_events(db, home.id, vec![event_a, event_b]);

    let range = TimeRange {
        start: "2025-12-19T18:00:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let breakdown = db
        .breakdown_by_model_effort_tokens(&range, home.id)
        .expect("breakdown");
    assert_eq!(breakdown.len(), 2);
    let high = breakdown
        .iter()
        .find(|row| row.reasoning_effort.as_deref() == Some("high"))
        .expect("high effort");
    assert_eq!(high.total_tokens, 120);
    let low = breakdown
        .iter()
        .find(|row| row.reasoning_effort.as_deref() == Some("low"))
        .expect("low effort");
    assert_eq!(low.total_tokens, 180);
}

#[test]
fn list_usage_events_defaults_effort_to_low() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_events(
        db,
        home.id,
        vec![make_event(
            "e1",
            "2025-12-19T19:00:00Z",
            "gpt-5.2",
            UsageTotals {
                input_tokens: 100,
                cached_input_tokens: 0,
                output_tokens: 20,
                reasoning_output_tokens: 0,
                total_tokens: 120,
            },
            "source-a",
        )],
    );

    let range = TimeRange {
        start: "2025-12-19T18:00:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let events = db
        .list_usage_events(&range, None, 10, 0, home.id)
        .expect("events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reasoning_effort.as_deref(), Some("low"));
}

#[test]
fn set_active_home_returns_expected_home() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = db
        .add_home("/tmp/codex-secondary", Some("Secondary"))
        .expect("add home");
    db.set_active_home(home.id).expect("set active");

    let active = db.get_active_home().expect("active home").expect("home");
    assert_eq!(active.id, home.id);
    assert_eq!(active.path, "/tmp/codex-secondary");
    assert_eq!(active.label, "Secondary");
}

#[test]
fn insert_limit_snapshots_dedupes_by_percent_and_reset() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    let snapshots = vec![
        make_limit_snapshot(
            "5h",
            40.0,
            "2025-01-01T05:00:00Z",
            "2025-01-01T00:00:00Z",
            "source-a",
        ),
        make_limit_snapshot(
            "5h",
            40.0,
            "2025-01-01T05:00:00Z",
            "2025-01-01T00:05:00Z",
            "source-a",
        ),
        make_limit_snapshot(
            "5h",
            35.0,
            "2025-01-01T05:00:00Z",
            "2025-01-01T00:10:00Z",
            "source-a",
        ),
    ];
    let inserted = db
        .insert_limit_snapshots(home.id, &snapshots)
        .expect("insert limits");
    assert_eq!(inserted, 2);
    let conn = Connection::open(&test_db.path).expect("open conn");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_limit_snapshot WHERE codex_home_id = ?1",
            [home.id],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 2);
}

#[test]
fn limit_current_window_ignores_stale_snapshot() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    let now = Utc::now();
    let reset_at = (now - Duration::hours(1)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let observed_at = (now - Duration::hours(2)).to_rfc3339_opts(SecondsFormat::Millis, true);
    let snapshots = vec![make_limit_snapshot(
        "7d",
        0.0,
        &reset_at,
        &observed_at,
        "source-a",
    )];
    db.insert_limit_snapshots(home.id, &snapshots)
        .expect("insert limits");

    let current = db
        .limit_current_window(home.id, "7d")
        .expect("current window");
    assert!(current.is_none());
}

#[test]
fn limit_windows_7d_uses_reset_boundaries() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_events(
        db,
        home.id,
        vec![
            make_event(
                "e1",
                "2025-01-09T00:00:00Z",
                "gpt-5.2",
                UsageTotals {
                    input_tokens: 50,
                    cached_input_tokens: 0,
                    output_tokens: 10,
                    reasoning_output_tokens: 0,
                    total_tokens: 60,
                },
                "source-window",
            ),
            make_event(
                "e2",
                "2025-01-10T00:00:00Z",
                "gpt-5.2",
                UsageTotals {
                    input_tokens: 100,
                    cached_input_tokens: 0,
                    output_tokens: 20,
                    reasoning_output_tokens: 0,
                    total_tokens: 120,
                },
                "source-window",
            ),
        ],
    );
    db.insert_message_events(
        home.id,
        &[
            make_message_event("m1", "2025-01-09T02:00:00Z", "source-window"),
            make_message_event("m2", "2025-01-10T02:00:00Z", "source-window"),
        ],
    )
    .expect("insert messages");
    let snapshots = vec![
        make_limit_snapshot(
            "7d",
            70.0,
            "2025-01-08T00:00:00Z",
            "2025-01-07T12:00:00Z",
            "source-a",
        ),
        make_limit_snapshot(
            "7d",
            55.0,
            "2025-01-15T00:00:00Z",
            "2025-01-10T12:00:00Z",
            "source-a",
        ),
    ];
    db.insert_limit_snapshots(home.id, &snapshots)
        .expect("insert limits");
    let windows = db.limit_windows_7d(home.id, 0).expect("windows");
    assert_eq!(windows.len(), 2);
    assert!(!windows[0].complete);
    assert_eq!(
        windows[0].window_start.as_deref(),
        Some("2025-01-01T00:00:00.000Z")
    );
    assert_eq!(windows[0].total_tokens, Some(0));
    assert!(windows[1].complete);
    assert_eq!(
        windows[1].window_start.as_deref(),
        Some("2025-01-08T00:00:00.000Z")
    );
    assert_eq!(windows[1].window_end, "2025-01-15T00:00:00.000Z");
    assert_eq!(windows[1].total_tokens, Some(120));
    assert_eq!(windows[1].message_count, Some(2));
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

#[test]
fn migrate_backfills_codex_home() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("backfill.sqlite");
    let codex_home = "/tmp/codex-home";
    {
        let conn = Connection::open(&db_path).expect("open conn");
        let migration = include_str!("../migrations/0001_init.sql");
        conn.execute_batch(migration).expect("migrate 0001");
        conn.execute(
            "INSERT INTO app_setting (key, value) VALUES ('codex_home', ?1)",
            [codex_home],
        )
        .expect("insert app setting");
        conn.execute(
            r#"
            INSERT INTO usage_event (
              id, ts, model, input_tokens, cached_input_tokens, output_tokens,
              reasoning_output_tokens, total_tokens, context_used, context_window,
              cost_usd, source, request_id, raw_json
            ) VALUES (
              'e1', '2025-12-19T19:00:00Z', 'gpt-5.2', 10, 0, 2, 0, 12, 12, 100, NULL, 'source-a', NULL, NULL
            )
            "#,
            [],
        )
        .expect("insert usage event");
        conn.execute(
            r#"
            INSERT INTO ingest_cursor (
              codex_home, file_path, inode, mtime, byte_offset, last_event_key, updated_at
            ) VALUES (
              ?1, 'log.ndjson', NULL, NULL, 123, 'e1', '2025-12-19T19:10:00Z'
            )
            "#,
            [codex_home],
        )
        .expect("insert cursor");
    }
    let mut db = tracker_db::Db::open(&db_path).expect("open db");
    db.migrate().expect("migrate db");

    let conn = Connection::open(&db_path).expect("open conn");
    let home_id: i64 = conn
        .query_row("SELECT id FROM codex_home LIMIT 1", [], |row| row.get(0))
        .expect("load home id");
    let stored_path: String = conn
        .query_row("SELECT path FROM codex_home LIMIT 1", [], |row| row.get(0))
        .expect("load home path");
    assert_eq!(stored_path, codex_home);

    let active_id: String = conn
        .query_row(
            "SELECT value FROM app_setting WHERE key = 'active_codex_home_id'",
            [],
            |row| row.get(0),
        )
        .expect("active home");
    assert_eq!(active_id, home_id.to_string());

    let event_home_id: i64 = conn
        .query_row(
            "SELECT codex_home_id FROM usage_event WHERE id = 'e1'",
            [],
            |row| row.get(0),
        )
        .expect("usage home");
    assert_eq!(event_home_id, home_id);

    let cursor_home_id: i64 = conn
        .query_row(
            "SELECT codex_home_id FROM ingest_cursor WHERE file_path = 'log.ndjson'",
            [],
            |row| row.get(0),
        )
        .expect("cursor home");
    assert_eq!(cursor_home_id, home_id);

    let session_id: String = conn
        .query_row(
            "SELECT session_id FROM usage_event WHERE id = 'e1'",
            [],
            |row| row.get(0),
        )
        .expect("session id");
    assert_eq!(session_id, "source-a");
}
