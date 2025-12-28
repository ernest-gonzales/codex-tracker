use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Local};
use rusqlite::params;
use tracker_core::{TimeRange, TimeSeriesPoint, UsageEvent, UsageSummary};

use crate::Db;
use crate::error::Result;
use crate::helpers::{compute_cost_from_pricing, compute_totals, delta_usage, row_to_usage_event};
use crate::types::{Bucket, Metric};

impl Db {
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
            .map_err(crate::error::DbError::from)
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
        let mut prev_by_source: HashMap<String, tracker_core::UsageTotals> = HashMap::new();
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
}
