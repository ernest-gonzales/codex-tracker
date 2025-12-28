use std::collections::HashMap;

use tracker_core::{
    CostBreakdown, ModelBreakdown, ModelCostBreakdown, ModelEffortCostBreakdown,
    ModelEffortTokenBreakdown, ModelTokenBreakdown, TimeRange, UsageTotals,
};

use crate::Db;
use crate::error::Result;
use crate::helpers::{
    add_usage, compute_cost_breakdown_from_pricing, compute_cost_from_pricing, delta_usage,
    rule_matches,
};

impl Db {
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
            let key = (row.model.clone(), row.reasoning_effort.clone());
            totals
                .entry(key)
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
        let pricing = self.list_pricing_rules()?;
        let rows = self.load_usage_rows(range, None, codex_home_id)?;
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
}
