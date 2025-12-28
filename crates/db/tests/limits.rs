mod support;

use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::Connection;
use support::{
    insert_events, make_event, make_limit_snapshot, make_message_event, setup_db, setup_home,
};
use tracker_core::UsageTotals;

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
