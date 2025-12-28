use std::collections::HashMap;

use rusqlite::Row;
use tracker_core::{
    CodexHome, ContextStatus, CostBreakdown, PricingRule, UsageEvent, UsageTotals,
    compute_cost_breakdown, model_matches_pattern,
};

use crate::error::Result;
use crate::types::RowUsage;

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

pub(crate) fn row_to_usage_row(row: &Row<'_>) -> std::result::Result<RowUsage, rusqlite::Error> {
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

pub(crate) fn row_to_usage_event(
    row: &Row<'_>,
) -> std::result::Result<UsageEvent, rusqlite::Error> {
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

pub(crate) fn row_to_codex_home(row: &Row<'_>) -> std::result::Result<CodexHome, rusqlite::Error> {
    Ok(CodexHome {
        id: row.get(0)?,
        label: row.get(1)?,
        path: row.get(2)?,
        created_at: row.get(3)?,
        last_seen_at: row.get(4)?,
    })
}

pub(crate) fn row_to_pricing_rule(
    row: &Row<'_>,
) -> std::result::Result<PricingRule, rusqlite::Error> {
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

pub(crate) fn delta_usage(prev: Option<&UsageTotals>, current: UsageTotals) -> UsageTotals {
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

pub(crate) fn add_usage(a: UsageTotals, b: UsageTotals) -> UsageTotals {
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

pub(crate) fn compute_cost_from_pricing(
    pricing: &[PricingRule],
    row: &RowUsage,
    delta: UsageTotals,
) -> f64 {
    compute_cost_breakdown_from_pricing(pricing, row, delta).total_cost_usd
}

pub(crate) fn compute_cost_breakdown_from_pricing(
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

pub(crate) fn rule_matches(rule: &PricingRule, row: &RowUsage) -> bool {
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

pub(crate) fn compute_totals(
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
