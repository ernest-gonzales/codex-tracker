mod support;

use support::{insert_events, insert_rules, make_event, setup_db, setup_home};
use tracker_core::{PricingRuleInput, TimeRange, UsageTotals};

#[test]
fn breakdown_by_model_costs_uses_output_only() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_rules(
        db,
        vec![PricingRuleInput {
            model_pattern: "gpt-5.2".to_string(),
            input_per_1m: 1750.0,
            cached_input_per_1m: 175.0,
            output_per_1m: 14000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        }],
    );
    insert_events(
        db,
        home.id,
        vec![
            make_event(
                "e1",
                "2025-12-19T19:00:00Z",
                "gpt-5.2",
                UsageTotals {
                    input_tokens: 800,
                    cached_input_tokens: 200,
                    output_tokens: 200,
                    reasoning_output_tokens: 100,
                    total_tokens: 1000,
                },
                "source-a",
            ),
            make_event(
                "e2",
                "2025-12-19T19:05:00Z",
                "gpt-5.2",
                UsageTotals {
                    input_tokens: 1600,
                    cached_input_tokens: 300,
                    output_tokens: 400,
                    reasoning_output_tokens: 200,
                    total_tokens: 2000,
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
        .breakdown_by_model_costs(&range, home.id)
        .expect("breakdown");
    let row = breakdown
        .iter()
        .find(|item| item.model == "gpt-5.2")
        .expect("row");

    let expected_input = (1300.0 / 1_000_000.0) * 1750.0;
    let expected_cached = (300.0 / 1_000_000.0) * 175.0;
    let expected_output = (400.0 / 1_000_000.0) * 14000.0;
    let expected_total = expected_input + expected_cached + expected_output;

    assert_eq!(row.input_tokens, 1600);
    assert_eq!(row.cached_input_tokens, 300);
    assert_eq!(row.output_tokens, 400);
    assert_eq!(row.reasoning_output_tokens, 200);
    assert_eq!(row.total_tokens, 2000);
    assert!((row.input_cost_usd.unwrap() - expected_input).abs() < 1e-9);
    assert!((row.cached_input_cost_usd.unwrap() - expected_cached).abs() < 1e-9);
    assert!((row.output_cost_usd.unwrap() - expected_output).abs() < 1e-9);
    assert!((row.total_cost_usd.unwrap() - expected_total).abs() < 1e-9);
}

#[test]
fn breakdown_by_model_costs_returns_none_without_pricing() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_events(
        db,
        home.id,
        vec![make_event(
            "e1",
            "2025-12-19T19:00:00Z",
            "unknown-model",
            UsageTotals {
                input_tokens: 100,
                cached_input_tokens: 0,
                output_tokens: 50,
                reasoning_output_tokens: 0,
                total_tokens: 150,
            },
            "source-a",
        )],
    );

    let range = TimeRange {
        start: "2025-12-19T18:40:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let breakdown = db
        .breakdown_by_model_costs(&range, home.id)
        .expect("breakdown");
    let row = breakdown
        .iter()
        .find(|item| item.model == "unknown-model")
        .expect("row");

    assert_eq!(row.total_tokens, 150);
    assert!(row.total_cost_usd.is_none());
    assert!(row.input_cost_usd.is_none());
    assert!(row.cached_input_cost_usd.is_none());
    assert!(row.output_cost_usd.is_none());
}

#[test]
fn update_event_costs_keeps_none_without_pricing() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_events(
        db,
        home.id,
        vec![make_event(
            "e1",
            "2025-12-19T19:00:00Z",
            "unknown-model",
            UsageTotals {
                input_tokens: 100,
                cached_input_tokens: 0,
                output_tokens: 50,
                reasoning_output_tokens: 0,
                total_tokens: 150,
            },
            "source-a",
        )],
    );

    db.update_event_costs(home.id).expect("update costs");

    let range = TimeRange {
        start: "2025-12-19T18:40:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let events = db
        .list_usage_events(&range, None, 10, 0, home.id)
        .expect("events");
    assert_eq!(events.len(), 1);
    assert!(events[0].cost_usd.is_none());
}

#[test]
fn update_event_costs_sets_value_with_pricing() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = setup_home(db);
    insert_rules(
        db,
        vec![PricingRuleInput {
            model_pattern: "gpt-5.2".to_string(),
            input_per_1m: 1750.0,
            cached_input_per_1m: 175.0,
            output_per_1m: 14000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        }],
    );
    insert_events(
        db,
        home.id,
        vec![make_event(
            "e1",
            "2025-12-19T19:00:00Z",
            "gpt-5.2",
            UsageTotals {
                input_tokens: 1000,
                cached_input_tokens: 200,
                output_tokens: 300,
                reasoning_output_tokens: 0,
                total_tokens: 1500,
            },
            "source-a",
        )],
    );

    db.update_event_costs(home.id).expect("update costs");

    let range = TimeRange {
        start: "2025-12-19T18:40:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let events = db
        .list_usage_events(&range, None, 10, 0, home.id)
        .expect("events");
    let cost = events[0].cost_usd.expect("cost");
    let expected_input = (800.0 / 1_000_000.0) * 1750.0;
    let expected_cached = (200.0 / 1_000_000.0) * 175.0;
    let expected_output = (300.0 / 1_000_000.0) * 14000.0;
    let expected_total = expected_input + expected_cached + expected_output;
    assert!((cost - expected_total).abs() < 1e-9);
}
