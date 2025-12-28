use std::collections::HashMap;
use std::env;
use std::time::Instant;

use rusqlite::params;
use tracker_core::{PricingRule, PricingRuleInput, UsageTotals};

use crate::Db;
use crate::error::Result;
use crate::helpers::{compute_cost_from_pricing, delta_usage, row_to_pricing_rule, rule_matches};

impl Db {
    pub fn list_pricing_rules(&self) -> Result<Vec<PricingRule>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, model_pattern, input_per_1m, cached_input_per_1m, output_per_1m, effective_from, effective_to
            FROM pricing_rule
            ORDER BY effective_from DESC, id DESC
            "#,
        )?;
        let rows = stmt
            .query_map([], row_to_pricing_rule)?
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
}
