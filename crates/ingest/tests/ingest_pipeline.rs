use std::fs::{self, OpenOptions};
use std::io::Write;

use ingest::ingest_codex_home;
use tempfile::tempdir;
use tracker_core::TimeRange;
use tracker_db::Db;

#[test]
fn ingest_resume_seeds_model_and_effort_from_cursor() {
    let dir = tempdir().expect("temp dir");
    let db_path = dir.path().join("ingest.sqlite");
    let mut db = Db::open(&db_path).expect("open db");
    db.migrate().expect("migrate db");

    let log_dir = dir.path().join("sessions/2025/01/01");
    fs::create_dir_all(&log_dir).expect("create log dir");
    let log_path = log_dir.join("rollout-2025-01-01T00-00-00-1234.jsonl");
    let initial = r#"
{"type":"session_meta","payload":{"info":{"model":"gpt-5.2-codex"}}}
{"type":"event_msg","payload":{"type":"turn_context","info":{"effort":"high"}}}
{"timestamp":"2025-01-01T00:00:10Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":2},"model_context_window":100}}}
"#;
    fs::write(&log_path, initial.trim()).expect("write log");

    let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest");
    assert_eq!(stats.events_inserted, 1);

    let appended = r#"
{"timestamp":"2025-01-01T00:00:20Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":2,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":3},"model_context_window":100}}}
"#;
    let mut file = OpenOptions::new()
        .append(true)
        .open(&log_path)
        .expect("open log");
    writeln!(file, "{}", appended.trim()).expect("append log");

    let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest again");
    assert_eq!(stats.events_inserted, 1);

    let home = db
        .get_home_by_path(&dir.path().to_string_lossy())
        .expect("home lookup")
        .expect("home");
    assert_eq!(db.count_usage_events(home.id).expect("count"), 2);
    let range = TimeRange {
        start: "0000-01-01T00:00:00Z".to_string(),
        end: "9999-12-31T23:59:59Z".to_string(),
    };
    let events = db
        .list_usage_events(&range, None, 10, 0, home.id)
        .expect("events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].model, "gpt-5.2-codex");
    assert_eq!(events[0].reasoning_effort.as_deref(), Some("high"));
}

#[test]
fn ingest_does_not_advance_cursor_on_invalid_utf8() {
    let dir = tempdir().expect("temp dir");
    let db_path = dir.path().join("ingest.sqlite");
    let mut db = Db::open(&db_path).expect("open db");
    db.migrate().expect("migrate db");

    let log_dir = dir.path().join("sessions/2025/01/01");
    fs::create_dir_all(&log_dir).expect("create log dir");
    let log_path = log_dir.join("bad.log");
    let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":2},"model_context_window":100}}}"#;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(line.as_bytes());
    bytes.push(b'\n');
    bytes.push(0xff);
    fs::write(&log_path, bytes).expect("write log");

    let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest");
    assert_eq!(stats.events_inserted, 1);
    assert_eq!(stats.issues.len(), 1);

    let home = db
        .get_home_by_path(&dir.path().to_string_lossy())
        .expect("home lookup")
        .expect("home");
    let cursor = db
        .get_cursor(home.id, &log_path.to_string_lossy())
        .expect("cursor lookup")
        .expect("cursor");
    let expected_offset = (line.len() + 1) as u64;
    assert_eq!(cursor.byte_offset, expected_offset);
}

#[test]
fn ingest_skips_plain_log_files() {
    let dir = tempdir().expect("tempdir");
    let log_dir = dir.path().join("sessions/2025/01/01");
    fs::create_dir_all(&log_dir).expect("create log dir");
    let log_path = log_dir.join("codex-tui.log");
    let json_path = log_dir.join("rollout-2025-12-19T21-31-36.jsonl");
    let db_path = dir.path().join("ingest.sqlite");
    let mut db = Db::open(&db_path).expect("open db");
    db.migrate().expect("migrate");
    fs::write(&log_path, "not json\nmore text\n").expect("write log");
    fs::write(
        &json_path,
        r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":10,"cached_input_tokens":0,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}"#,
    )
    .expect("write json");
    let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest");
    assert_eq!(stats.events_inserted, 1);
    assert!(stats.files_skipped >= 1);
}

#[test]
fn ingest_sets_cost_on_insert() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("ingest.sqlite");
    let mut db = Db::open(&db_path).expect("open db");
    db.migrate().expect("migrate");
    db.replace_pricing_rules(&[tracker_core::PricingRuleInput {
        model_pattern: "gpt-test".to_string(),
        input_per_1m: 1750.0,
        cached_input_per_1m: 175.0,
        output_per_1m: 14000.0,
        effective_from: "2025-01-01T00:00:00Z".to_string(),
        effective_to: None,
    }])
    .expect("pricing");

    let log_dir = dir.path().join("sessions/2025/01/01");
    fs::create_dir_all(&log_dir).expect("create log dir");
    let log_path = log_dir.join("rollout-2025-12-19T21-31-36.jsonl");
    fs::write(
        &log_path,
        r#"{"timestamp":"2025-12-19T19:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":1000,"cached_input_tokens":200,"output_tokens":300,"reasoning_output_tokens":0,"total_tokens":1500},"model_context_window":100}}}"#,
    )
    .expect("write json");

    ingest_codex_home(&mut db, dir.path()).expect("ingest");
    let home = db
        .get_home_by_path(dir.path().to_string_lossy().as_ref())
        .expect("get home")
        .expect("home");
    let range = TimeRange {
        start: "2025-12-19T18:40:00Z".to_string(),
        end: "2025-12-19T20:00:00Z".to_string(),
    };
    let events = db
        .list_usage_events(&range, None, 10, 0, home.id)
        .expect("list events");
    assert_eq!(events.len(), 1);
    let cost = events[0].cost_usd.expect("cost");
    let expected_input = (800.0 / 1_000_000.0) * 1750.0;
    let expected_cached = (200.0 / 1_000_000.0) * 175.0;
    let expected_output = (300.0 / 1_000_000.0) * 14000.0;
    let expected_total = expected_input + expected_cached + expected_output;
    assert!((cost - expected_total).abs() < 1e-9);
}
