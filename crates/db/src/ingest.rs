use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};
use tracker_core::{MessageEvent, UsageEvent, UsageLimitSnapshot, UsageTotals};

use crate::Db;
use crate::error::Result;
use crate::types::IngestCursor;

impl Db {
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
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO usage_limit_snapshot (
                  codex_home_id, ts, limit_type, percent_left, reset_at, source, raw_line
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7
                )
                "#,
            )?;
            for snapshot in snapshots {
                let limit_type = snapshot.limit_type.clone();
                let should_insert = match last_by_type.get(&limit_type) {
                    Some((percent_left, reset_at)) => {
                        *percent_left != snapshot.percent_left || *reset_at != snapshot.reset_at
                    }
                    None => true,
                };
                if !should_insert {
                    continue;
                }
                let rows = stmt.execute(params![
                    codex_home_id,
                    snapshot.observed_at,
                    snapshot.limit_type,
                    snapshot.percent_left,
                    snapshot.reset_at,
                    snapshot.source,
                    snapshot.raw_line
                ])?;
                if rows > 0 {
                    inserted += 1;
                }
                last_by_type.insert(
                    limit_type,
                    (snapshot.percent_left, snapshot.reset_at.clone()),
                );
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn get_cursor(&self, codex_home_id: i64, file_path: &str) -> Result<Option<IngestCursor>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT codex_home_id, codex_home, file_path, inode, mtime, byte_offset,
                   last_event_key, updated_at, last_model, last_effort
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
              codex_home_id, codex_home, file_path, inode, mtime, byte_offset,
              last_event_key, updated_at, last_model, last_effort
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
            )
            ON CONFLICT(codex_home, file_path) DO UPDATE SET
              codex_home = excluded.codex_home,
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

    pub fn last_usage_totals_for_source(
        &self,
        codex_home_id: i64,
        source: &str,
    ) -> Result<Option<UsageTotals>> {
        self.conn
            .query_row(
                r#"
                SELECT input_tokens, cached_input_tokens, output_tokens,
                       reasoning_output_tokens, total_tokens
                FROM usage_event
                WHERE codex_home_id = ?1 AND source = ?2
                ORDER BY ts DESC
                LIMIT 1
                "#,
                params![codex_home_id, source],
                |row| {
                    Ok(UsageTotals {
                        input_tokens: row.get::<_, i64>(0)? as u64,
                        cached_input_tokens: row.get::<_, i64>(1)? as u64,
                        output_tokens: row.get::<_, i64>(2)? as u64,
                        reasoning_output_tokens: row.get::<_, i64>(3)? as u64,
                        total_tokens: row.get::<_, i64>(4)? as u64,
                    })
                },
            )
            .optional()
            .map_err(crate::error::DbError::from)
    }
}
