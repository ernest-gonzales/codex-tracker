use std::collections::BTreeMap;

use chrono::{DateTime, Duration, SecondsFormat, Timelike, Utc};
use rusqlite::params;
use tracker_core::{TimeRange, UsageLimitCurrentWindow, UsageLimitSnapshot, UsageLimitWindow};

use crate::Db;
use crate::error::Result;

impl Db {
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
        let mut rows = stmt.query(params![codex_home_id])?;
        while let Some(row) = rows.next()? {
            let reset_at = row.get::<_, String>(0)?;
            if let Ok(parsed) = DateTime::parse_from_rfc3339(&reset_at) {
                let normalized = normalize_limit_boundary(parsed.with_timezone(&Utc));
                reset_set.insert(normalized, ());
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
            Some(value) => value,
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
}

fn normalize_limit_boundary(value: DateTime<Utc>) -> DateTime<Utc> {
    value
        .with_second(0)
        .and_then(|dt| dt.with_nanosecond(0))
        .unwrap_or(value)
}
