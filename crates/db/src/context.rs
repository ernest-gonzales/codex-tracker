use rusqlite::params;
use tracker_core::{ActiveSession, ContextPressureStats, ContextStatus, TimeRange};

use crate::Db;
use crate::error::Result;

impl Db {
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
}
