use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use tracker_core::session_id_from_source;

use crate::Db;
use crate::error::Result;
use crate::homes::load_codex_home_path;

const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_add_cached_input_pricing.sql");
const MIGRATION_0003: &str = include_str!("../migrations/0003_add_codex_home.sql");
const MIGRATION_0004: &str = include_str!("../migrations/0004_pricing_per_1m.sql");
const MIGRATION_0005: &str = include_str!("../migrations/0005_add_session_id.sql");
const MIGRATION_0006: &str = include_str!("../migrations/0006_add_reasoning_effort.sql");
const MIGRATION_0007: &str = include_str!("../migrations/0007_add_usage_limits.sql");
const MIGRATION_0008: &str = include_str!("../migrations/0008_add_message_events.sql");
const MIGRATION_0009: &str = include_str!("../migrations/0009_add_cursor_state.sql");

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", MIGRATION_0001),
    ("0002_add_cached_input_pricing", MIGRATION_0002),
    ("0003_add_codex_home", MIGRATION_0003),
    ("0004_pricing_per_1m", MIGRATION_0004),
    ("0005_add_session_id", MIGRATION_0005),
    ("0006_add_reasoning_effort", MIGRATION_0006),
    ("0007_add_usage_limits", MIGRATION_0007),
    ("0008_add_message_events", MIGRATION_0008),
    ("0009_add_cursor_state", MIGRATION_0009),
];

impl Db {
    pub fn migrate(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (name, sql) in MIGRATIONS {
            if *name == "0002_add_cached_input_pricing" && pricing_rule_has_cached_column(&tx)? {
                continue;
            }
            if *name == "0003_add_codex_home" {
                tx.execute_batch(sql)?;
                ensure_codex_home_columns(&tx)?;
                ensure_codex_home_indexes(&tx)?;
                backfill_codex_home(&tx)?;
                continue;
            }
            if *name == "0004_pricing_per_1m" && pricing_rule_has_per_1m_columns(&tx)? {
                continue;
            }
            if *name == "0005_add_session_id" {
                if table_has_column(&tx, "usage_event", "session_id")? {
                    ensure_session_id_indexes(&tx)?;
                    backfill_session_ids(&tx)?;
                    continue;
                }
                tx.execute_batch(sql)?;
                ensure_session_id_indexes(&tx)?;
                backfill_session_ids(&tx)?;
                continue;
            }
            if *name == "0006_add_reasoning_effort" {
                if table_has_column(&tx, "usage_event", "reasoning_effort")? {
                    ensure_effort_indexes(&tx)?;
                    continue;
                }
                tx.execute_batch(sql)?;
                continue;
            }
            if *name == "0009_add_cursor_state" {
                ensure_ingest_cursor_state_columns(&tx)?;
                continue;
            }
            tx.execute_batch(sql)?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn pricing_rule_has_cached_column(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(pricing_rule)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "cached_input_per_1k" {
            return Ok(true);
        }
    }
    Ok(false)
}

fn pricing_rule_has_per_1m_columns(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(pricing_rule)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "input_per_1m" {
            return Ok(true);
        }
    }
    Ok(false)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_codex_home_columns(conn: &Connection) -> Result<()> {
    if !table_has_column(conn, "usage_event", "codex_home_id")? {
        conn.execute(
            "ALTER TABLE usage_event ADD COLUMN codex_home_id INTEGER",
            [],
        )?;
    }
    if !table_has_column(conn, "ingest_cursor", "codex_home_id")? {
        conn.execute(
            "ALTER TABLE ingest_cursor ADD COLUMN codex_home_id INTEGER",
            [],
        )?;
    }
    Ok(())
}

fn ensure_codex_home_indexes(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_usage_event_home_ts ON usage_event (codex_home_id, ts)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ingest_cursor_home_file ON ingest_cursor (codex_home_id, file_path)",
        [],
    )?;
    Ok(())
}

fn ensure_session_id_indexes(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_usage_event_home_session_ts ON usage_event (codex_home_id, session_id, ts)",
        [],
    )?;
    Ok(())
}

fn ensure_effort_indexes(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_usage_event_home_model_effort ON usage_event (codex_home_id, model, reasoning_effort, ts)",
        [],
    )?;
    Ok(())
}

fn ensure_ingest_cursor_state_columns(conn: &Connection) -> Result<()> {
    if !table_has_column(conn, "ingest_cursor", "last_model")? {
        conn.execute("ALTER TABLE ingest_cursor ADD COLUMN last_model TEXT", [])?;
    }
    if !table_has_column(conn, "ingest_cursor", "last_effort")? {
        conn.execute("ALTER TABLE ingest_cursor ADD COLUMN last_effort TEXT", [])?;
    }
    Ok(())
}

fn backfill_codex_home(conn: &Connection) -> Result<()> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM codex_home ORDER BY id ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let home_id = if let Some(id) = existing {
        id
    } else {
        let path = load_codex_home_path(conn)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO codex_home (label, path, created_at, last_seen_at) VALUES (?1, ?2, ?3, ?4)",
            params!["Default", path, now, now],
        )?;
        conn.last_insert_rowid()
    };
    conn.execute(
        "UPDATE usage_event SET codex_home_id = ?1 WHERE codex_home_id IS NULL",
        params![home_id],
    )?;
    conn.execute(
        "UPDATE ingest_cursor SET codex_home_id = ?1 WHERE codex_home_id IS NULL",
        params![home_id],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO app_setting (key, value) VALUES ('active_codex_home_id', ?1)",
        params![home_id.to_string()],
    )?;
    Ok(())
}

fn backfill_session_ids(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT DISTINCT source, session_id FROM usage_event")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let source: String = row.get(0)?;
        let session_id: Option<String> = row.get(1)?;
        let derived = session_id_from_source(&source);
        if session_id.as_deref() != Some(derived.as_str()) {
            conn.execute(
                "UPDATE usage_event SET session_id = ?1 WHERE source = ?2",
                params![derived, source],
            )?;
        }
    }
    Ok(())
}
