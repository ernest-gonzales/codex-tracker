use std::collections::{BTreeMap, HashMap};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::{DateTime, Duration, Local, SecondsFormat, Timelike, Utc};
use rusqlite::OptionalExtension;
use rusqlite::{Connection, Row, params};
use tracker_core::{
    ActiveSession, CodexHome, ContextPressureStats, ContextStatus, CostBreakdown, MessageEvent,
    ModelBreakdown, ModelCostBreakdown, ModelEffortCostBreakdown, ModelEffortTokenBreakdown,
    ModelTokenBreakdown, PricingRule, PricingRuleInput, TimeRange, TimeSeriesPoint, UsageEvent,
    UsageLimitCurrentWindow, UsageLimitSnapshot, UsageLimitWindow, UsageSummary, UsageTotals,
    compute_cost_breakdown, model_matches_pattern, session_id_from_source,
};

pub const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");
pub const MIGRATION_0002: &str = include_str!("../migrations/0002_add_cached_input_pricing.sql");
pub const MIGRATION_0003: &str = include_str!("../migrations/0003_add_codex_home.sql");
pub const MIGRATION_0004: &str = include_str!("../migrations/0004_pricing_per_1m.sql");
pub const MIGRATION_0005: &str = include_str!("../migrations/0005_add_session_id.sql");
pub const MIGRATION_0006: &str = include_str!("../migrations/0006_add_reasoning_effort.sql");
pub const MIGRATION_0007: &str = include_str!("../migrations/0007_add_usage_limits.sql");
pub const MIGRATION_0008: &str = include_str!("../migrations/0008_add_message_events.sql");
pub const MIGRATION_0009: &str = include_str!("../migrations/0009_add_cursor_state.sql");

pub const MIGRATIONS: &[(&str, &str)] = &[
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

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("time parse error: {0}")]
    TimeParse(#[from] chrono::ParseError),
}

pub type Result<T> = std::result::Result<T, DbError>;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "cache_size", -20_000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn })
    }

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

    pub fn insert_usage_events(
        &mut self,
        codex_home_id: i64,
        events: &[UsageEvent],
    ) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR IGNORE INTO usage_event (
                  id, ts, model, input_tokens, cached_input_tokens, output_tokens,
                  reasoning_output_tokens, total_tokens, context_used, context_window,
                  cost_usd, source, session_id, request_id, raw_json, codex_home_id,
                  reasoning_effort
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
                )
                "#,
            )?;
            for event in events {
                let rows = stmt.execute(params![
                    event.id,
                    event.ts,
                    event.model,
                    event.usage.input_tokens as i64,
                    event.usage.cached_input_tokens as i64,
                    event.usage.output_tokens as i64,
                    event.usage.reasoning_output_tokens as i64,
                    event.usage.total_tokens as i64,
                    event.context.context_used as i64,
                    event.context.context_window as i64,
                    event.cost_usd,
                    event.source,
                    event.session_id,
                    event.request_id,
                    event.raw_json,
                    codex_home_id,
                    event.reasoning_effort,
                ])?;
                if rows > 0 {
                    inserted += 1;
                }
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn insert_message_events(
        &mut self,
        codex_home_id: i64,
        events: &[MessageEvent],
    ) -> Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR IGNORE INTO message_event (
                  id, ts, role, source, session_id, raw_json, codex_home_id
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7
                )
                "#,
            )?;
            for event in events {
                let rows = stmt.execute(params![
                    event.id,
                    event.ts,
                    event.role,
                    event.source,
                    event.session_id,
                    event.raw_json,
                    codex_home_id,
                ])?;
                if rows > 0 {
                    inserted += 1;
                }
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn insert_limit_snapshots(
        &mut self,
        codex_home_id: i64,
        snapshots: &[UsageLimitSnapshot],
    ) -> Result<usize> {
        if snapshots.is_empty() {
            return Ok(0);
        }
        let mut last_by_type: HashMap<String, (f64, String)> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT limit_type, percent_left, reset_at
                FROM usage_limit_snapshot
                WHERE codex_home_id = ?1
                ORDER BY ts DESC
                "#,
            )?;
            let mut rows = stmt.query(params![codex_home_id])?;
            while let Some(row) = rows.next()? {
                let limit_type: String = row.get(0)?;
                if last_by_type.contains_key(&limit_type) {
                    continue;
                }
                last_by_type.insert(limit_type, (row.get::<_, f64>(1)?, row.get(2)?));
            }
        }

        let tx = self.conn.transaction()?;
        let mut inserted = 0usize;
        {
            let mut insert_stmt = tx.prepare(
                r#"
                INSERT INTO usage_limit_snapshot (
                  codex_home_id, ts, limit_type, percent_left, reset_at, source, raw_line
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )?;
            for snapshot in snapshots {
                let limit_type = snapshot.limit_type.clone();
                let should_insert = match last_by_type.get(&limit_type) {
                    Some((percent_left, reset_at)) => {
                        (*percent_left - snapshot.percent_left).abs() > 0.0001
                            || reset_at != &snapshot.reset_at
                    }
                    None => true,
                };
                if !should_insert {
                    continue;
                }
                insert_stmt.execute(params![
                    codex_home_id,
                    snapshot.observed_at,
                    snapshot.limit_type,
                    snapshot.percent_left,
                    snapshot.reset_at,
                    snapshot.source,
                    snapshot.raw_line,
                ])?;
                last_by_type.insert(
                    limit_type,
                    (snapshot.percent_left, snapshot.reset_at.clone()),
                );
                inserted += 1;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn latest_context(&self, codex_home_id: i64) -> Result<Option<ContextStatus>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT context_used, context_window
            FROM usage_event
            WHERE codex_home_id = ?1
            ORDER BY ts DESC
            LIMIT 1
            "#,
        )?;
        let mut rows = stmt.query(params![codex_home_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ContextStatus {
                context_used: row.get::<_, i64>(0)? as u64,
                context_window: row.get::<_, i64>(1)? as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn latest_limit_snapshot(
        &self,
        codex_home_id: i64,
        limit_type: &str,
    ) -> Result<Option<UsageLimitSnapshot>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT limit_type, percent_left, reset_at, ts, source, raw_line
            FROM usage_limit_snapshot
            WHERE codex_home_id = ?1 AND limit_type = ?2
            ORDER BY ts DESC
            LIMIT 1
            "#,
        )?;
        let mut rows = stmt.query(params![codex_home_id, limit_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(UsageLimitSnapshot {
                limit_type: row.get(0)?,
                percent_left: row.get(1)?,
                reset_at: row.get(2)?,
                observed_at: row.get(3)?,
                source: row.get(4)?,
                raw_line: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn latest_limit_snapshot_current(
        &self,
        codex_home_id: i64,
        limit_type: &str,
    ) -> Result<Option<UsageLimitSnapshot>> {
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT limit_type, percent_left, reset_at, ts, source, raw_line
            FROM usage_limit_snapshot
            WHERE codex_home_id = ?1 AND limit_type = ?2 AND reset_at >= ?3
            ORDER BY ts DESC
            LIMIT 1
            "#,
        )?;
        let mut rows = stmt.query(params![codex_home_id, limit_type, now])?;
        if let Some(row) = rows.next()? {
            Ok(Some(UsageLimitSnapshot {
                limit_type: row.get(0)?,
                percent_left: row.get(1)?,
                reset_at: row.get(2)?,
                observed_at: row.get(3)?,
                source: row.get(4)?,
                raw_line: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn limit_windows_7d(
        &self,
        codex_home_id: i64,
        limit: usize,
    ) -> Result<Vec<UsageLimitWindow>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT reset_at
            FROM usage_limit_snapshot
            WHERE codex_home_id = ?1 AND limit_type = '7d'
            ORDER BY reset_at ASC
            "#,
        )?;
        let mut reset_set = BTreeMap::<DateTime<Utc>, ()>::new();
        let reset_rows = stmt
            .query_map(params![codex_home_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for reset_at in reset_rows {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(&reset_at) {
                let normalized = normalize_limit_boundary(parsed.with_timezone(&Utc));
                reset_set.entry(normalized).or_insert(());
            }
        }
        let resets: Vec<DateTime<Utc>> = reset_set.into_keys().collect();
        let mut windows = Vec::new();
        let mut prev: Option<DateTime<Utc>> = None;
        for reset_at in resets {
            let complete = prev.is_some();
            let start = prev.unwrap_or_else(|| reset_at - Duration::days(7));
            let range = TimeRange {
                start: normalize_limit_boundary(start).to_rfc3339_opts(SecondsFormat::Millis, true),
                end: normalize_limit_boundary(reset_at)
                    .to_rfc3339_opts(SecondsFormat::Millis, true),
            };
            let summary = self.summary(&range, codex_home_id)?;
            let message_count = self.message_count_in_range(&range, codex_home_id)?;
            windows.push(UsageLimitWindow {
                window_start: Some(range.start),
                window_end: range.end,
                total_tokens: Some(summary.total_tokens),
                total_cost_usd: summary.total_cost_usd,
                message_count: Some(message_count),
                complete,
            });
            prev = Some(reset_at);
        }
        if limit == 0 || windows.len() <= limit {
            return Ok(windows);
        }
        Ok(windows
            .into_iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect())
    }

    pub fn limit_current_window(
        &self,
        codex_home_id: i64,
        limit_type: &str,
    ) -> Result<Option<UsageLimitCurrentWindow>> {
        let snapshot = self.latest_limit_snapshot_current(codex_home_id, limit_type)?;
        let snapshot = match snapshot {
            Some(snapshot) => snapshot,
            None => return Ok(None),
        };
        let reset_at = DateTime::parse_from_rfc3339(&snapshot.reset_at)?.with_timezone(&Utc);
        let duration = match limit_type {
            "5h" => Duration::hours(5),
            "7d" => Duration::days(7),
            _ => return Ok(None),
        };
        let start = reset_at - duration;
        let range = TimeRange {
            start: normalize_limit_boundary(start).to_rfc3339_opts(SecondsFormat::Millis, true),
            end: normalize_limit_boundary(reset_at).to_rfc3339_opts(SecondsFormat::Millis, true),
        };
        let summary = self.summary(&range, codex_home_id)?;
        let message_count = self.message_count_in_range(&range, codex_home_id)?;
        Ok(Some(UsageLimitCurrentWindow {
            window_start: range.start,
            window_end: range.end,
            total_tokens: Some(summary.total_tokens),
            total_cost_usd: summary.total_cost_usd,
            message_count: Some(message_count),
        }))
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_setting WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get::<_, String>(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO app_setting (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, value],
        )?;
        Ok(())
    }

    pub fn list_homes(&self) -> Result<Vec<CodexHome>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, label, path, created_at, last_seen_at
            FROM codex_home
            ORDER BY created_at ASC, id ASC
            "#,
        )?;
        let rows = stmt
            .query_map([], row_to_codex_home)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_home_by_id(&self, id: i64) -> Result<Option<CodexHome>> {
        self.conn
            .query_row(
                r#"
                SELECT id, label, path, created_at, last_seen_at
                FROM codex_home
                WHERE id = ?1
                "#,
                params![id],
                row_to_codex_home,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn get_home_by_path(&self, path: &str) -> Result<Option<CodexHome>> {
        self.conn
            .query_row(
                r#"
                SELECT id, label, path, created_at, last_seen_at
                FROM codex_home
                WHERE path = ?1
                "#,
                params![path],
                row_to_codex_home,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn add_home(&self, path: &str, label: Option<&str>) -> Result<CodexHome> {
        let now = Utc::now().to_rfc3339();
        let label = label.unwrap_or("Home");
        self.conn.execute(
            r#"
            INSERT INTO codex_home (label, path, created_at, last_seen_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![label, path, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_home_by_id(id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn get_or_create_home(&self, path: &str, label: Option<&str>) -> Result<CodexHome> {
        if let Some(home) = self.get_home_by_path(path)? {
            return Ok(home);
        }
        let inserted = self.add_home(path, label);
        if let Ok(home) = inserted {
            return Ok(home);
        }
        if let Some(home) = self.get_home_by_path(path)? {
            return Ok(home);
        }
        Err(inserted
            .err()
            .unwrap_or(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)))
    }

    pub fn set_active_home(&self, home_id: i64) -> Result<()> {
        self.set_setting("active_codex_home_id", &home_id.to_string())
    }

    pub fn get_active_home(&self) -> Result<Option<CodexHome>> {
        let value = self.get_setting("active_codex_home_id")?;
        let Some(value) = value else {
            return Ok(None);
        };
        let id = value.parse::<i64>().ok();
        let Some(id) = id else {
            return Ok(None);
        };
        self.get_home_by_id(id)
    }

    pub fn ensure_active_home(&mut self) -> Result<CodexHome> {
        if let Some(home) = self.get_active_home()? {
            return Ok(home);
        }
        let path = load_codex_home_path(&self.conn)?;
        let home = self.get_or_create_home(&path, Some("Default"))?;
        self.set_active_home(home.id)?;
        Ok(home)
    }

    pub fn update_home_last_seen(&self, home_id: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE codex_home SET last_seen_at = ?1 WHERE id = ?2",
            params![now, home_id],
        )?;
        Ok(())
    }

    pub fn delete_home(&mut self, home_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM usage_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM message_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM usage_limit_snapshot WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM ingest_cursor WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute("DELETE FROM codex_home WHERE id = ?1", params![home_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn clear_home_data(&mut self, home_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM usage_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM message_event WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM usage_limit_snapshot WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.execute(
            "DELETE FROM ingest_cursor WHERE codex_home_id = ?1",
            params![home_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn count_usage_events(&self, home_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM usage_event WHERE codex_home_id = ?1",
                params![home_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    pub fn count_message_events(&self, home_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM message_event WHERE codex_home_id = ?1",
                params![home_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    pub fn count_ingest_cursors(&self, home_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM ingest_cursor WHERE codex_home_id = ?1",
                params![home_id],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    pub fn list_pricing_rules(&self) -> Result<Vec<PricingRule>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, model_pattern, input_per_1m, cached_input_per_1m, output_per_1m, effective_from, effective_to
            FROM pricing_rule
            ORDER BY effective_from DESC, id DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], Self::row_to_pricing_rule)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn replace_pricing_rules(&mut self, rules: &[PricingRuleInput]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM pricing_rule", [])?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx.prepare(
                r#"
            INSERT INTO pricing_rule (
              model_pattern,
              input_per_1k,
              cached_input_per_1k,
              output_per_1k,
              input_per_1m,
              cached_input_per_1m,
              output_per_1m,
              effective_from,
              effective_to
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            )?;
            for rule in rules {
                stmt.execute(params![
                    rule.model_pattern,
                    rule.input_per_1m / 1000.0,
                    rule.cached_input_per_1m / 1000.0,
                    rule.output_per_1m / 1000.0,
                    rule.input_per_1m,
                    rule.cached_input_per_1m,
                    rule.output_per_1m,
                    rule.effective_from,
                    rule.effective_to
                ])?;
                inserted += 1;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn summary(&self, range: &TimeRange, codex_home_id: i64) -> Result<UsageSummary> {
        let pricing = self.list_pricing_rules()?;
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let (totals, cost, cost_known) = compute_totals(rows, &pricing)?;
        Ok(UsageSummary {
            total_tokens: totals.total_tokens,
            input_tokens: totals.input_tokens,
            cached_input_tokens: totals.cached_input_tokens,
            output_tokens: totals.output_tokens,
            reasoning_output_tokens: totals.reasoning_output_tokens,
            total_cost_usd: if cost_known {
                Some(cost.total_cost_usd)
            } else {
                None
            },
            input_cost_usd: if cost_known {
                Some(cost.input_cost_usd)
            } else {
                None
            },
            cached_input_cost_usd: if cost_known {
                Some(cost.cached_input_cost_usd)
            } else {
                None
            },
            output_cost_usd: if cost_known {
                Some(cost.output_cost_usd)
            } else {
                None
            },
        })
    }

    pub fn message_count_in_range(&self, range: &TimeRange, codex_home_id: i64) -> Result<u64> {
        self.conn
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM message_event
                WHERE codex_home_id = ?1 AND ts >= ?2 AND ts < ?3
                "#,
                params![codex_home_id, range.start, range.end],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value as u64)
            .map_err(DbError::from)
    }

    pub fn timeseries(
        &self,
        range: &TimeRange,
        bucket: Bucket,
        metric: Metric,
        codex_home_id: i64,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let pricing = self.list_pricing_rules()?;
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let mut buckets: BTreeMap<String, f64> = BTreeMap::new();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        for row in rows {
            let prev = prev_by_source.get(&row.source);
            let delta = delta_usage(prev, row.usage);
            prev_by_source.insert(row.source.clone(), row.usage);
            let ts = DateTime::parse_from_rfc3339(&row.ts)?;
            let local = ts.with_timezone(&Local);
            let bucket_start = match bucket {
                Bucket::Hour => local.format("%Y-%m-%dT%H:00:00%:z").to_string(),
                Bucket::Day => local.format("%Y-%m-%dT00:00:00%:z").to_string(),
            };
            let value = match metric {
                Metric::Tokens => delta.total_tokens as f64,
                Metric::Cost => row
                    .cost_usd
                    .unwrap_or_else(|| compute_cost_from_pricing(&pricing, &row, delta)),
            };
            *buckets.entry(bucket_start).or_insert(0.0) += value;
        }
        Ok(buckets
            .into_iter()
            .map(|(bucket_start, value)| TimeSeriesPoint {
                bucket_start,
                value,
            })
            .collect())
    }

    pub fn breakdown_by_model(
        &self,
        range: &TimeRange,
        codex_home_id: i64,
    ) -> Result<Vec<ModelBreakdown>> {
        let pricing = self.list_pricing_rules()?;
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let mut totals: HashMap<String, UsageTotals> = HashMap::new();
        let mut costs: HashMap<String, f64> = HashMap::new();
        let mut cost_known: HashMap<String, bool> = HashMap::new();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        for row in rows {
            let prev = prev_by_source.get(&row.source);
            let delta = delta_usage(prev, row.usage);
            prev_by_source.insert(row.source.clone(), row.usage);
            totals
                .entry(row.model.clone())
                .and_modify(|value| *value = add_usage(*value, delta))
                .or_insert(delta);
            let cost_value = row
                .cost_usd
                .unwrap_or_else(|| compute_cost_from_pricing(&pricing, &row, delta));
            costs
                .entry(row.model.clone())
                .and_modify(|value| *value += cost_value)
                .or_insert(cost_value);
            if row.cost_usd.is_some() || pricing.iter().any(|rule| rule_matches(rule, &row)) {
                cost_known.insert(row.model.clone(), true);
            }
        }
        let mut result: Vec<ModelBreakdown> = totals
            .into_iter()
            .map(|(model, usage)| ModelBreakdown {
                model: model.clone(),
                total_tokens: usage.total_tokens,
                total_cost_usd: cost_known.get(&model).and_then(|known| {
                    if *known {
                        costs.get(&model).copied()
                    } else {
                        None
                    }
                }),
            })
            .collect();
        result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
        Ok(result)
    }

    pub fn breakdown_by_model_tokens(
        &self,
        range: &TimeRange,
        codex_home_id: i64,
    ) -> Result<Vec<ModelTokenBreakdown>> {
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let mut totals: HashMap<String, UsageTotals> = HashMap::new();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        for row in rows {
            let prev = prev_by_source.get(&row.source);
            let delta = delta_usage(prev, row.usage);
            prev_by_source.insert(row.source.clone(), row.usage);
            totals
                .entry(row.model.clone())
                .and_modify(|value| *value = add_usage(*value, delta))
                .or_insert(delta);
        }
        let mut result: Vec<ModelTokenBreakdown> = totals
            .into_iter()
            .map(|(model, usage)| ModelTokenBreakdown {
                model,
                input_tokens: usage.input_tokens,
                cached_input_tokens: usage.cached_input_tokens,
                output_tokens: usage.output_tokens,
                reasoning_output_tokens: usage.reasoning_output_tokens,
                total_tokens: usage.total_tokens,
            })
            .collect();
        result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
        Ok(result)
    }

    pub fn breakdown_by_model_costs(
        &self,
        range: &TimeRange,
        codex_home_id: i64,
    ) -> Result<Vec<ModelCostBreakdown>> {
        let pricing = self.list_pricing_rules()?;
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let mut totals: HashMap<String, UsageTotals> = HashMap::new();
        let mut costs: HashMap<String, CostBreakdown> = HashMap::new();
        let mut cost_known: HashMap<String, bool> = HashMap::new();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        for row in rows {
            let prev = prev_by_source.get(&row.source);
            let delta = delta_usage(prev, row.usage);
            prev_by_source.insert(row.source.clone(), row.usage);
            totals
                .entry(row.model.clone())
                .and_modify(|value| *value = add_usage(*value, delta))
                .or_insert(delta);
            let cost_value = compute_cost_breakdown_from_pricing(&pricing, &row, delta);
            costs
                .entry(row.model.clone())
                .and_modify(|value| {
                    value.input_cost_usd += cost_value.input_cost_usd;
                    value.cached_input_cost_usd += cost_value.cached_input_cost_usd;
                    value.output_cost_usd += cost_value.output_cost_usd;
                    value.total_cost_usd += cost_value.total_cost_usd;
                })
                .or_insert(cost_value);
            if pricing.iter().any(|rule| rule_matches(rule, &row)) {
                cost_known.insert(row.model.clone(), true);
            }
        }
        let mut result: Vec<ModelCostBreakdown> = totals
            .into_iter()
            .map(|(model, usage)| {
                let known = cost_known.get(&model).copied().unwrap_or(false);
                let cost = costs.get(&model).copied().unwrap_or_default();
                ModelCostBreakdown {
                    model,
                    input_tokens: usage.input_tokens,
                    cached_input_tokens: usage.cached_input_tokens,
                    output_tokens: usage.output_tokens,
                    reasoning_output_tokens: usage.reasoning_output_tokens,
                    total_tokens: usage.total_tokens,
                    input_cost_usd: known.then_some(cost.input_cost_usd),
                    cached_input_cost_usd: known.then_some(cost.cached_input_cost_usd),
                    output_cost_usd: known.then_some(cost.output_cost_usd),
                    total_cost_usd: known.then_some(cost.total_cost_usd),
                }
            })
            .collect();
        result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
        Ok(result)
    }

    pub fn breakdown_by_model_effort_tokens(
        &self,
        range: &TimeRange,
        codex_home_id: i64,
    ) -> Result<Vec<ModelEffortTokenBreakdown>> {
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let mut totals: HashMap<(String, Option<String>), UsageTotals> = HashMap::new();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        for row in rows {
            let prev = prev_by_source.get(&row.source);
            let delta = delta_usage(prev, row.usage);
            prev_by_source.insert(row.source.clone(), row.usage);
            totals
                .entry((row.model.clone(), row.reasoning_effort.clone()))
                .and_modify(|value| *value = add_usage(*value, delta))
                .or_insert(delta);
        }
        let mut result: Vec<ModelEffortTokenBreakdown> = totals
            .into_iter()
            .map(|((model, effort), usage)| ModelEffortTokenBreakdown {
                model,
                reasoning_effort: effort,
                input_tokens: usage.input_tokens,
                cached_input_tokens: usage.cached_input_tokens,
                output_tokens: usage.output_tokens,
                reasoning_output_tokens: usage.reasoning_output_tokens,
                total_tokens: usage.total_tokens,
            })
            .collect();
        result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
        Ok(result)
    }

    pub fn breakdown_by_model_effort_costs(
        &self,
        range: &TimeRange,
        codex_home_id: i64,
    ) -> Result<Vec<ModelEffortCostBreakdown>> {
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
        let pricing = self.list_pricing_rules()?;
        let mut totals: HashMap<(String, Option<String>), UsageTotals> = HashMap::new();
        let mut costs: HashMap<(String, Option<String>), CostBreakdown> = HashMap::new();
        let mut cost_known: HashMap<String, bool> = HashMap::new();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        for row in rows {
            let prev = prev_by_source.get(&row.source);
            let delta = delta_usage(prev, row.usage);
            prev_by_source.insert(row.source.clone(), row.usage);
            let key = (row.model.clone(), row.reasoning_effort.clone());
            totals
                .entry(key.clone())
                .and_modify(|value| *value = add_usage(*value, delta))
                .or_insert(delta);
            let cost_value = compute_cost_breakdown_from_pricing(&pricing, &row, delta);
            costs
                .entry(key)
                .and_modify(|value| {
                    value.input_cost_usd += cost_value.input_cost_usd;
                    value.cached_input_cost_usd += cost_value.cached_input_cost_usd;
                    value.output_cost_usd += cost_value.output_cost_usd;
                    value.total_cost_usd += cost_value.total_cost_usd;
                })
                .or_insert(cost_value);
            if pricing.iter().any(|rule| rule_matches(rule, &row)) {
                cost_known.insert(row.model.clone(), true);
            }
        }
        let mut result: Vec<ModelEffortCostBreakdown> = totals
            .into_iter()
            .map(|((model, effort), usage)| {
                let known = cost_known.get(&model).copied().unwrap_or(false);
                let cost = costs
                    .get(&(model.clone(), effort.clone()))
                    .copied()
                    .unwrap_or_default();
                ModelEffortCostBreakdown {
                    model,
                    reasoning_effort: effort,
                    input_tokens: usage.input_tokens,
                    cached_input_tokens: usage.cached_input_tokens,
                    output_tokens: usage.output_tokens,
                    reasoning_output_tokens: usage.reasoning_output_tokens,
                    total_tokens: usage.total_tokens,
                    input_cost_usd: known.then_some(cost.input_cost_usd),
                    cached_input_cost_usd: known.then_some(cost.cached_input_cost_usd),
                    output_cost_usd: known.then_some(cost.output_cost_usd),
                    total_cost_usd: known.then_some(cost.total_cost_usd),
                }
            })
            .collect();
        result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
        Ok(result)
    }

    pub fn list_usage_events(
        &self,
        range: &TimeRange,
        model: Option<&str>,
        limit: u32,
        offset: u32,
        codex_home_id: i64,
    ) -> Result<Vec<UsageEvent>> {
        let mut sql = String::from(
            r#"
            SELECT id, ts, model, input_tokens, cached_input_tokens, output_tokens,
                   reasoning_output_tokens, total_tokens, context_used, context_window,
                   cost_usd, source, session_id, request_id, raw_json, reasoning_effort
            FROM usage_event
            WHERE codex_home_id = ?1 AND ts >= ?2 AND ts < ?3
            "#,
        );
        if model.is_some() {
            sql.push_str(" AND model = ?4 ");
            sql.push_str(" ORDER BY ts DESC LIMIT ?5 OFFSET ?6");
        } else {
            sql.push_str(" ORDER BY ts DESC LIMIT ?4 OFFSET ?5");
        }
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = if let Some(model) = model {
            stmt.query(params![
                codex_home_id,
                range.start,
                range.end,
                model,
                limit,
                offset
            ])?
        } else {
            stmt.query(params![
                codex_home_id,
                range.start,
                range.end,
                limit,
                offset
            ])?
        };
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            events.push(row_to_usage_event(row)?);
        }
        Ok(events)
    }

    pub fn active_sessions(&self, codex_home_id: i64, since: &str) -> Result<Vec<ActiveSession>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT ue.session_id, ue.ts, latest.start_ts, ue.model, ue.context_used, ue.context_window
            FROM usage_event ue
            INNER JOIN (
                SELECT session_id, MAX(ts) AS last_ts, MIN(ts) AS start_ts
                FROM usage_event
                WHERE codex_home_id = ?1 AND ts >= ?2
                GROUP BY session_id
            ) latest
            ON ue.session_id = latest.session_id AND ue.ts = latest.last_ts
            WHERE ue.codex_home_id = ?1
            ORDER BY ue.ts DESC
            "#,
        )?;
        let rows = stmt.query_map(params![codex_home_id, since], |row| {
            Ok(ActiveSession {
                session_id: row.get(0)?,
                last_seen: row.get(1)?,
                session_start: row.get(2)?,
                model: row.get(3)?,
                context_used: row.get::<_, i64>(4)? as u64,
                context_window: row.get::<_, i64>(5)? as u64,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn context_pressure_stats(
        &self,
        range: &TimeRange,
        codex_home_id: i64,
    ) -> Result<ContextPressureStats> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
              COUNT(*) AS sample_count,
              AVG(context_used) AS avg_context_used,
              AVG(context_window) AS avg_context_window,
              AVG((context_used * 1.0) / context_window) AS avg_pressure
            FROM usage_event
            WHERE codex_home_id = ?1
              AND ts >= ?2
              AND ts < ?3
              AND context_window > 0
            "#,
        )?;
        let stats = stmt.query_row(params![codex_home_id, range.start, range.end], |row| {
            let sample_count: i64 = row.get(0)?;
            Ok(ContextPressureStats {
                avg_context_used: row.get::<_, Option<f64>>(1)?,
                avg_context_window: row.get::<_, Option<f64>>(2)?,
                avg_pressure_pct: row.get::<_, Option<f64>>(3)?.map(|value| value * 100.0),
                sample_count: sample_count.max(0) as u64,
            })
        })?;
        Ok(stats)
    }

    pub fn get_context_active_minutes(&self) -> Result<u32> {
        let value = self.get_setting("context_active_minutes")?;
        Ok(value
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(60))
    }

    pub fn set_context_active_minutes(&self, minutes: u32) -> Result<()> {
        self.set_setting("context_active_minutes", &minutes.to_string())
    }

    pub fn update_event_costs(&mut self, codex_home_id: i64) -> Result<usize> {
        let timing_enabled = env::var("CODEX_TRACKER_INGEST_TIMING").is_ok();
        let start = Instant::now();
        let pricing = self.list_pricing_rules()?;
        let load_start = Instant::now();
        let rows = self.load_usage_rows_all(codex_home_id)?;
        let rows_len = rows.len();
        let load_duration = load_start.elapsed();
        let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
        let tx = self.conn.transaction()?;
        let update_start = Instant::now();
        let mut updated = 0usize;
        {
            let mut stmt = tx.prepare(
                "UPDATE usage_event SET cost_usd = ?1 WHERE id = ?2 AND codex_home_id = ?3",
            )?;
            for row in rows {
                let prev = prev_by_source.get(&row.source);
                let delta = delta_usage(prev, row.usage);
                prev_by_source.insert(row.source.clone(), row.usage);
                let cost = if pricing.iter().any(|rule| rule_matches(rule, &row)) {
                    Some(compute_cost_from_pricing(&pricing, &row, delta))
                } else {
                    None
                };
                stmt.execute(params![cost, row.id, codex_home_id])?;
                updated += 1;
            }
        }
        tx.commit()?;
        if timing_enabled {
            eprintln!(
                "update_event_costs: rows={} load={}ms update={}ms total={}ms",
                rows_len,
                load_duration.as_millis(),
                update_start.elapsed().as_millis(),
                start.elapsed().as_millis()
            );
        }
        Ok(updated)
    }

    pub fn get_cursor(&self, codex_home_id: i64, file_path: &str) -> Result<Option<IngestCursor>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT codex_home_id, codex_home, file_path, inode, mtime, byte_offset, last_event_key,
                   updated_at, last_model, last_effort
            FROM ingest_cursor
            WHERE codex_home_id = ?1 AND file_path = ?2
            "#,
        )?;
        let mut rows = stmt.query(params![codex_home_id, file_path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(IngestCursor {
                codex_home_id: row.get(0)?,
                codex_home: row.get(1)?,
                file_path: row.get(2)?,
                inode: row.get::<_, Option<i64>>(3)?.map(|value| value as u64),
                mtime: row.get(4)?,
                byte_offset: row.get::<_, i64>(5)? as u64,
                last_event_key: row.get(6)?,
                updated_at: row.get(7)?,
                last_model: row.get(8)?,
                last_effort: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_cursor(&self, cursor: &IngestCursor) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO ingest_cursor (
              codex_home_id, codex_home, file_path, inode, mtime, byte_offset, last_event_key,
              updated_at, last_model, last_effort
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(codex_home, file_path) DO UPDATE SET
              codex_home_id = excluded.codex_home_id,
              inode = excluded.inode,
              mtime = excluded.mtime,
              byte_offset = excluded.byte_offset,
              last_event_key = excluded.last_event_key,
              updated_at = excluded.updated_at,
              last_model = excluded.last_model,
              last_effort = excluded.last_effort
            "#,
            params![
                cursor.codex_home_id,
                cursor.codex_home,
                cursor.file_path,
                cursor.inode.map(|value| value as i64),
                cursor.mtime,
                cursor.byte_offset as i64,
                cursor.last_event_key,
                cursor.updated_at,
                cursor.last_model,
                cursor.last_effort
            ],
        )?;
        Ok(())
    }

    pub fn load_usage_rows(
        &self,
        range: &TimeRange,
        model: Option<&str>,
        codex_home_id: i64,
    ) -> Result<Vec<RowUsage>> {
        let mut sql = String::from(
            r#"
            SELECT id, ts, model, input_tokens, cached_input_tokens, output_tokens,
                   reasoning_output_tokens, total_tokens, cost_usd, source, reasoning_effort
            FROM usage_event
            WHERE codex_home_id = ?1 AND ts >= ?2 AND ts < ?3
            "#,
        );
        if model.is_some() {
            sql.push_str(" AND model = ?4 ");
        }
        sql.push_str(" ORDER BY source, ts ASC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = if let Some(model) = model {
            stmt.query_map(
                params![codex_home_id, range.start, range.end, model],
                row_to_usage_row,
            )?
        } else {
            stmt.query_map(
                params![codex_home_id, range.start, range.end],
                row_to_usage_row,
            )?
        };
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn load_usage_rows_all(&self, codex_home_id: i64) -> Result<Vec<RowUsage>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, ts, model, input_tokens, cached_input_tokens, output_tokens,
                   reasoning_output_tokens, total_tokens, cost_usd, source, reasoning_effort
            FROM usage_event
            WHERE codex_home_id = ?1
            ORDER BY source, ts ASC
            "#,
        )?;
        let rows = stmt.query_map(params![codex_home_id], row_to_usage_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn row_to_pricing_rule(row: &Row<'_>) -> std::result::Result<PricingRule, rusqlite::Error> {
        Ok(PricingRule {
            id: Some(row.get::<_, i64>(0)?),
            model_pattern: row.get(1)?,
            input_per_1m: row.get(2)?,
            cached_input_per_1m: row.get(3)?,
            output_per_1m: row.get(4)?,
            effective_from: row.get(5)?,
            effective_to: row.get(6)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Bucket {
    Hour,
    Day,
}

#[derive(Debug, Clone, Copy)]
pub enum Metric {
    Tokens,
    Cost,
}

#[derive(Debug, Clone)]
pub struct RowUsage {
    pub id: String,
    pub ts: String,
    pub model: String,
    pub usage: UsageTotals,
    pub cost_usd: Option<f64>,
    pub source: String,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IngestCursor {
    pub codex_home_id: i64,
    pub codex_home: String,
    pub file_path: String,
    pub inode: Option<u64>,
    pub mtime: Option<String>,
    pub byte_offset: u64,
    pub last_event_key: Option<String>,
    pub updated_at: String,
    pub last_model: Option<String>,
    pub last_effort: Option<String>,
}

fn normalize_effort(value: Option<String>) -> Option<String> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Some("low".to_string());
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower == "unknown" || lower == "unknow" {
                return Some("low".to_string());
            }
            Some(trimmed.to_string())
        }
        None => Some("low".to_string()),
    }
}

fn row_to_usage_row(row: &Row<'_>) -> std::result::Result<RowUsage, rusqlite::Error> {
    Ok(RowUsage {
        id: row.get(0)?,
        ts: row.get(1)?,
        model: row.get(2)?,
        usage: UsageTotals {
            input_tokens: row.get::<_, i64>(3)? as u64,
            cached_input_tokens: row.get::<_, i64>(4)? as u64,
            output_tokens: row.get::<_, i64>(5)? as u64,
            reasoning_output_tokens: row.get::<_, i64>(6)? as u64,
            total_tokens: row.get::<_, i64>(7)? as u64,
        },
        cost_usd: row.get(8)?,
        source: row.get(9)?,
        reasoning_effort: normalize_effort(row.get(10)?),
    })
}

fn row_to_usage_event(row: &Row<'_>) -> std::result::Result<UsageEvent, rusqlite::Error> {
    Ok(UsageEvent {
        id: row.get(0)?,
        ts: row.get(1)?,
        model: row.get(2)?,
        usage: UsageTotals {
            input_tokens: row.get::<_, i64>(3)? as u64,
            cached_input_tokens: row.get::<_, i64>(4)? as u64,
            output_tokens: row.get::<_, i64>(5)? as u64,
            reasoning_output_tokens: row.get::<_, i64>(6)? as u64,
            total_tokens: row.get::<_, i64>(7)? as u64,
        },
        context: ContextStatus {
            context_used: row.get::<_, i64>(8)? as u64,
            context_window: row.get::<_, i64>(9)? as u64,
        },
        cost_usd: row.get(10)?,
        source: row.get(11)?,
        session_id: row.get(12)?,
        request_id: row.get(13)?,
        raw_json: row.get(14)?,
        reasoning_effort: normalize_effort(row.get(15)?),
    })
}

fn row_to_codex_home(row: &Row<'_>) -> std::result::Result<CodexHome, rusqlite::Error> {
    Ok(CodexHome {
        id: row.get(0)?,
        label: row.get(1)?,
        path: row.get(2)?,
        created_at: row.get(3)?,
        last_seen_at: row.get(4)?,
    })
}

fn delta_usage(prev: Option<&UsageTotals>, current: UsageTotals) -> UsageTotals {
    if let Some(prev) = prev {
        if current.total_tokens >= prev.total_tokens {
            UsageTotals {
                input_tokens: current.input_tokens.saturating_sub(prev.input_tokens),
                cached_input_tokens: current
                    .cached_input_tokens
                    .saturating_sub(prev.cached_input_tokens),
                output_tokens: current.output_tokens.saturating_sub(prev.output_tokens),
                reasoning_output_tokens: current
                    .reasoning_output_tokens
                    .saturating_sub(prev.reasoning_output_tokens),
                total_tokens: current.total_tokens.saturating_sub(prev.total_tokens),
            }
        } else {
            current
        }
    } else {
        current
    }
}

fn add_usage(a: UsageTotals, b: UsageTotals) -> UsageTotals {
    UsageTotals {
        input_tokens: a.input_tokens.saturating_add(b.input_tokens),
        cached_input_tokens: a.cached_input_tokens.saturating_add(b.cached_input_tokens),
        output_tokens: a.output_tokens.saturating_add(b.output_tokens),
        reasoning_output_tokens: a
            .reasoning_output_tokens
            .saturating_add(b.reasoning_output_tokens),
        total_tokens: a.total_tokens.saturating_add(b.total_tokens),
    }
}

fn normalize_limit_boundary(value: DateTime<Utc>) -> DateTime<Utc> {
    value
        .with_second(0)
        .and_then(|dt| dt.with_nanosecond(0))
        .unwrap_or(value)
}

fn compute_cost_from_pricing(pricing: &[PricingRule], row: &RowUsage, delta: UsageTotals) -> f64 {
    compute_cost_breakdown_from_pricing(pricing, row, delta).total_cost_usd
}

fn compute_cost_breakdown_from_pricing(
    pricing: &[PricingRule],
    row: &RowUsage,
    delta: UsageTotals,
) -> CostBreakdown {
    if let Some(rule) = pricing
        .iter()
        .filter(|rule| rule_matches(rule, row))
        .max_by(|a, b| a.effective_from.cmp(&b.effective_from))
    {
        compute_cost_breakdown(delta, rule)
    } else {
        CostBreakdown::default()
    }
}

fn rule_matches(rule: &PricingRule, row: &RowUsage) -> bool {
    if !model_matches_pattern(&row.model, &rule.model_pattern) {
        return false;
    }
    if rule.effective_from > row.ts {
        return false;
    }
    if let Some(ref end) = rule.effective_to
        && row.ts >= *end
    {
        return false;
    }
    true
}

fn compute_totals(
    rows: Vec<RowUsage>,
    pricing: &[PricingRule],
) -> Result<(UsageTotals, CostBreakdown, bool)> {
    let mut totals = UsageTotals::default();
    let mut total_cost = CostBreakdown::default();
    let mut cost_known = false;
    let mut prev_by_source: HashMap<String, UsageTotals> = HashMap::new();
    for row in rows {
        let prev = prev_by_source.get(&row.source);
        let delta = delta_usage(prev, row.usage);
        prev_by_source.insert(row.source.clone(), row.usage);
        totals = add_usage(totals, delta);
        let cost = compute_cost_breakdown_from_pricing(pricing, &row, delta);
        if pricing.iter().any(|rule| rule_matches(rule, &row)) {
            cost_known = true;
        }
        total_cost.input_cost_usd += cost.input_cost_usd;
        total_cost.cached_input_cost_usd += cost.cached_input_cost_usd;
        total_cost.output_cost_usd += cost.output_cost_usd;
        total_cost.total_cost_usd += cost.total_cost_usd;
    }
    Ok((totals, total_cost, cost_known))
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

fn load_codex_home_path(conn: &Connection) -> Result<String> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM app_setting WHERE key = 'codex_home'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(path) = stored {
        return Ok(path);
    }
    Ok(default_codex_home_path())
}

fn default_codex_home_path() -> String {
    if let Ok(path) = env::var("CODEX_HOME") {
        return path;
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".codex")
            .to_string_lossy()
            .to_string();
    }
    ".codex".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tracker_core::{
        ContextStatus, MessageEvent, PricingRuleInput, UsageEvent, UsageLimitSnapshot,
    };

    fn setup_db() -> Db {
        let mut db = Db::open(":memory:").expect("open db");
        db.migrate().expect("migrate db");
        db
    }

    fn insert_rules(db: &mut Db, rules: Vec<PricingRuleInput>) {
        db.replace_pricing_rules(&rules).expect("replace pricing");
    }

    fn insert_events(db: &mut Db, codex_home_id: i64, events: Vec<UsageEvent>) {
        db.insert_usage_events(codex_home_id, &events)
            .expect("insert events");
    }

    fn setup_home(db: &mut Db) -> CodexHome {
        let home = db
            .get_or_create_home("/tmp/codex-home", Some("Default"))
            .expect("home");
        db.set_active_home(home.id).expect("active");
        home
    }

    fn make_event(id: &str, ts: &str, model: &str, usage: UsageTotals, source: &str) -> UsageEvent {
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

    fn make_limit_snapshot(
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

    fn make_message_event(id: &str, ts: &str, source: &str) -> MessageEvent {
        MessageEvent {
            id: id.to_string(),
            ts: ts.to_string(),
            role: "user".to_string(),
            source: source.to_string(),
            session_id: session_id_from_source(source),
            raw_json: None,
        }
    }

    #[test]
    fn breakdown_by_model_costs_uses_output_only() {
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_rules(
            &mut db,
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
            &mut db,
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
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_events(
            &mut db,
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
    fn context_pressure_stats_averages_known_context_only() {
        let mut db = setup_db();
        let home = setup_home(&mut db);
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
        insert_events(&mut db, home.id, vec![event1, event2, event3, event4]);

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
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_events(
            &mut db,
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
        let mut db = setup_db();
        let home = setup_home(&mut db);
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
        insert_events(&mut db, home.id, vec![event_a, event_b]);

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
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_events(
            &mut db,
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
    fn update_event_costs_keeps_none_without_pricing() {
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_events(
            &mut db,
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
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_rules(
            &mut db,
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
            &mut db,
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

    #[test]
    fn set_active_home_returns_expected_home() {
        let db = setup_db();
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
        let mut db = setup_db();
        let home = setup_home(&mut db);
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
        let count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM usage_limit_snapshot WHERE codex_home_id = ?1",
                params![home.id],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(count, 2);
    }

    #[test]
    fn limit_current_window_ignores_stale_snapshot() {
        let mut db = setup_db();
        let home = setup_home(&mut db);
        let now = Utc::now();
        let reset_at = (now - Duration::hours(1))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let observed_at = (now - Duration::hours(2))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
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
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_events(
            &mut db,
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
        let mut db = setup_db();
        let home = setup_home(&mut db);
        insert_events(
            &mut db,
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
            conn.execute_batch(MIGRATION_0001).expect("migrate 0001");
            conn.execute(
                "INSERT INTO app_setting (key, value) VALUES ('codex_home', ?1)",
                params![codex_home],
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
                params![codex_home],
            )
            .expect("insert cursor");
        }
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");

        let home_id: i64 = db
            .conn
            .query_row("SELECT id FROM codex_home LIMIT 1", [], |row| row.get(0))
            .expect("load home id");
        let stored_path: String = db
            .conn
            .query_row("SELECT path FROM codex_home LIMIT 1", [], |row| row.get(0))
            .expect("load home path");
        assert_eq!(stored_path, codex_home);

        let active_id: String = db
            .conn
            .query_row(
                "SELECT value FROM app_setting WHERE key = 'active_codex_home_id'",
                [],
                |row| row.get(0),
            )
            .expect("active home");
        assert_eq!(active_id, home_id.to_string());

        let event_home_id: i64 = db
            .conn
            .query_row(
                "SELECT codex_home_id FROM usage_event WHERE id = 'e1'",
                [],
                |row| row.get(0),
            )
            .expect("usage home");
        assert_eq!(event_home_id, home_id);

        let cursor_home_id: i64 = db
            .conn
            .query_row(
                "SELECT codex_home_id FROM ingest_cursor WHERE file_path = 'log.ndjson'",
                [],
                |row| row.get(0),
            )
            .expect("cursor home");
        assert_eq!(cursor_home_id, home_id);

        let session_id: String = db
            .conn
            .query_row(
                "SELECT session_id FROM usage_event WHERE id = 'e1'",
                [],
                |row| row.get(0),
            )
            .expect("session id");
        assert_eq!(session_id, "source-a");
    }
}
