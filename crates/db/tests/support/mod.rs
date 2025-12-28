#![allow(dead_code)]

use std::path::PathBuf;

use tempfile::TempDir;
use tracker_core::{
    CodexHome, ContextStatus, MessageEvent, PricingRuleInput, UsageEvent, UsageLimitSnapshot,
    UsageTotals, session_id_from_source,
};
use tracker_db::Db;

pub struct TestDb {
    pub _dir: TempDir,
    pub db: Db,
    pub path: PathBuf,
}

pub fn setup_db() -> TestDb {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.sqlite");
    let mut db = Db::open(&path).expect("open db");
    db.migrate().expect("migrate db");
    TestDb {
        _dir: dir,
        db,
        path,
    }
}

pub fn insert_rules(db: &mut Db, rules: Vec<PricingRuleInput>) {
    db.replace_pricing_rules(&rules).expect("replace pricing");
}

pub fn insert_events(db: &mut Db, codex_home_id: i64, events: Vec<UsageEvent>) {
    db.insert_usage_events(codex_home_id, &events)
        .expect("insert events");
}

pub fn setup_home(db: &mut Db) -> CodexHome {
    let home = db
        .get_or_create_home("/tmp/codex-home", Some("Default"))
        .expect("home");
    db.set_active_home(home.id).expect("active");
    home
}

pub fn make_event(id: &str, ts: &str, model: &str, usage: UsageTotals, source: &str) -> UsageEvent {
    UsageEvent {
        id: id.to_string(),
        ts: ts.to_string(),
        model: model.to_string(),
        usage,
        context: ContextStatus {
            context_used: usage.total_tokens,
            context_window: 100_000,
        },
        cost_usd: None,
        reasoning_effort: None,
        source: source.to_string(),
        session_id: session_id_from_source(source),
        request_id: None,
        raw_json: None,
    }
}

pub fn make_limit_snapshot(
    limit_type: &str,
    percent_left: f64,
    reset_at: &str,
    observed_at: &str,
    source: &str,
) -> UsageLimitSnapshot {
    UsageLimitSnapshot {
        limit_type: limit_type.to_string(),
        percent_left,
        reset_at: reset_at.to_string(),
        observed_at: observed_at.to_string(),
        source: source.to_string(),
        raw_line: None,
    }
}

pub fn make_message_event(id: &str, ts: &str, source: &str) -> MessageEvent {
    MessageEvent {
        id: id.to_string(),
        ts: ts.to_string(),
        role: "user".to_string(),
        source: source.to_string(),
        session_id: session_id_from_source(source),
        raw_json: None,
    }
}
