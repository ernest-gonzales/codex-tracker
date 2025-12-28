mod support;

use support::{insert_events, make_event, setup_db, setup_home};
use tracker_core::{TimeRange, UsageTotals};

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
