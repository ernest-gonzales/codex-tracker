use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTotals {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextStatus {
    pub context_used: u64,
    pub context_window: u64,
}

impl ContextStatus {
    pub fn percent_left(&self) -> Option<f64> {
        if self.context_window == 0 {
            return None;
        }
        let used = self.context_used as f64;
        let total = self.context_window as f64;
        Some(((total - used) / total) * 100.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextPressureStats {
    pub avg_context_used: Option<f64>,
    pub avg_context_window: Option<f64>,
    pub avg_pressure_pct: Option<f64>,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub model: String,
    pub last_seen: String,
    pub session_start: String,
    pub context_used: u64,
    pub context_window: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_cost_usd: Option<f64>,
    pub input_cost_usd: Option<f64>,
    pub cached_input_cost_usd: Option<f64>,
    pub output_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageLimitSnapshot {
    pub limit_type: String,
    pub percent_left: f64,
    pub reset_at: String,
    pub observed_at: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_line: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageLimitWindow {
    pub window_start: Option<String>,
    pub window_end: String,
    pub total_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub message_count: Option<u64>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageLimitCurrentWindow {
    pub window_start: String,
    pub window_end: String,
    pub total_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub message_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageLimitCurrentResponse {
    pub primary: Option<UsageLimitCurrentWindow>,
    pub secondary: Option<UsageLimitCurrentWindow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: String,
    pub ts: String,
    pub model: String,
    pub usage: UsageTotals,
    pub context: ContextStatus,
    pub cost_usd: Option<f64>,
    pub reasoning_effort: Option<String>,
    pub source: String,
    pub session_id: String,
    pub request_id: Option<String>,
    pub raw_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEvent {
    pub id: String,
    pub ts: String,
    pub role: String,
    pub source: String,
    pub session_id: String,
    pub raw_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRule {
    pub id: Option<i64>,
    pub model_pattern: String,
    pub input_per_1m: f64,
    pub cached_input_per_1m: f64,
    pub output_per_1m: f64,
    pub effective_from: String,
    pub effective_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRuleInput {
    pub model_pattern: String,
    pub input_per_1m: f64,
    pub cached_input_per_1m: f64,
    pub output_per_1m: f64,
    pub effective_from: String,
    pub effective_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexHome {
    pub id: i64,
    pub label: String,
    pub path: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub bucket_start: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBreakdown {
    pub model: String,
    pub total_tokens: u64,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTokenBreakdown {
    pub model: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEffortTokenBreakdown {
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostBreakdown {
    pub model: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
    pub input_cost_usd: Option<f64>,
    pub cached_input_cost_usd: Option<f64>,
    pub output_cost_usd: Option<f64>,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEffortCostBreakdown {
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
    pub input_cost_usd: Option<f64>,
    pub cached_input_cost_usd: Option<f64>,
    pub output_cost_usd: Option<f64>,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub input_cost_usd: f64,
    pub cached_input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub total_cost_usd: f64,
}

pub fn model_matches_pattern(model: &str, pattern: &str) -> bool {
    let model = model.to_ascii_lowercase();
    let pattern = pattern.to_ascii_lowercase();
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return model == pattern;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut remainder = model.as_str();
    let mut first = true;
    for part in parts {
        if part.is_empty() {
            continue;
        }
        if let Some(index) = remainder.find(part) {
            if first && index != 0 {
                return false;
            }
            remainder = &remainder[index + part.len()..];
            first = false;
        } else {
            return false;
        }
    }
    if pattern.ends_with('*') {
        true
    } else {
        remainder.is_empty()
    }
}

pub fn compute_cost_usd(usage: UsageTotals, rule: &PricingRule) -> f64 {
    compute_cost_breakdown(usage, rule).total_cost_usd
}

pub fn compute_cost_breakdown(usage: UsageTotals, rule: &PricingRule) -> CostBreakdown {
    let non_cached_input = usage.input_tokens.saturating_sub(usage.cached_input_tokens) as f64;
    let cached_input = usage.cached_input_tokens as f64;
    // Treat reasoning tokens as a subset of output tokens to avoid double billing.
    let output = usage.output_tokens as f64;
    let input_cost = (non_cached_input / 1_000_000.0) * rule.input_per_1m;
    let cached_input_cost = (cached_input / 1_000_000.0) * rule.cached_input_per_1m;
    let output_cost = (output / 1_000_000.0) * rule.output_per_1m;
    CostBreakdown {
        input_cost_usd: input_cost,
        cached_input_cost_usd: cached_input_cost,
        output_cost_usd: output_cost,
        total_cost_usd: input_cost + cached_input_cost + output_cost,
    }
}

pub fn session_id_from_source(source: &str) -> String {
    let file_name = Path::new(source).file_name().and_then(|name| name.to_str());
    let stem =
        file_name.and_then(|name| Path::new(name).file_stem().and_then(|value| value.to_str()));
    if let Some(rest) = stem.and_then(|value| value.strip_prefix("rollout-"))
        && let Some(split_at) = rest.rfind('-')
    {
        let session_id = &rest[split_at + 1..];
        if !session_id.is_empty() {
            return session_id.to_string();
        }
    }
    source.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_breakdown_does_not_double_count_reasoning() {
        let rule = PricingRule {
            id: None,
            model_pattern: "gpt-5.2".to_string(),
            input_per_1m: 1750.0,
            cached_input_per_1m: 175.0,
            output_per_1m: 14000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        };
        let usage = UsageTotals {
            input_tokens: 10_000,
            cached_input_tokens: 4_000,
            output_tokens: 2_000,
            reasoning_output_tokens: 1_000,
            total_tokens: 12_000,
        };

        let cost = compute_cost_breakdown(usage, &rule);

        let expected_input = (6_000.0 / 1_000_000.0) * 1750.0;
        let expected_cached = (4_000.0 / 1_000_000.0) * 175.0;
        let expected_output = (2_000.0 / 1_000_000.0) * 14000.0;

        assert!((cost.input_cost_usd - expected_input).abs() < 1e-9);
        assert!((cost.cached_input_cost_usd - expected_cached).abs() < 1e-9);
        assert!((cost.output_cost_usd - expected_output).abs() < 1e-9);
        assert!(
            (cost.total_cost_usd - (expected_input + expected_cached + expected_output)).abs()
                < 1e-9
        );
    }

    #[test]
    fn cost_breakdown_uses_output_tokens_when_no_reasoning() {
        let rule = PricingRule {
            id: None,
            model_pattern: "*".to_string(),
            input_per_1m: 1000.0,
            cached_input_per_1m: 100.0,
            output_per_1m: 2000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        };
        let usage = UsageTotals {
            input_tokens: 2_000,
            cached_input_tokens: 1_000,
            output_tokens: 3_000,
            reasoning_output_tokens: 0,
            total_tokens: 5_000,
        };

        let cost = compute_cost_breakdown(usage, &rule);
        let expected_output = (3_000.0 / 1_000_000.0) * 2000.0;

        assert!((cost.output_cost_usd - expected_output).abs() < 1e-9);
    }

    #[test]
    fn session_id_from_source_parses_rollout_name() {
        let source = "/tmp/rollout-2025-12-20T00-00-00Z-abc123.jsonl";
        assert_eq!(session_id_from_source(source), "abc123");
    }

    #[test]
    fn session_id_from_source_falls_back_to_path() {
        let source = "/tmp/codex.log";
        assert_eq!(session_id_from_source(source), source);
    }
}
