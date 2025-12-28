use rusqlite::params;
use tracker_core::TimeRange;

use crate::Db;
use crate::error::Result;
use crate::helpers::row_to_usage_row;
use crate::types::RowUsage;

impl Db {
    pub(crate) fn load_usage_rows(
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

    pub(crate) fn load_usage_rows_all(&self, codex_home_id: i64) -> Result<Vec<RowUsage>> {
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
}
