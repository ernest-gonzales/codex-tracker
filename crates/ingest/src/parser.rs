use std::fmt::Write;

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracker_core::{
    ContextStatus, MessageEvent, PricingRule, UsageEvent, UsageLimitSnapshot, UsageTotals,
    compute_cost_breakdown, model_matches_pattern, session_id_from_source,
};

use crate::types::TokenTotals;

fn parse_token_totals(value: &Value) -> Option<TokenTotals> {
    let total_tokens = value
        .get("total_token_usage")?
        .get("total_tokens")?
        .as_u64()?;
    let last_tokens = value
        .get("last_token_usage")
        .and_then(|usage| usage.get("total_tokens"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    Some(TokenTotals {
        total_tokens,
        last_tokens,
    })
}

fn parse_usage_totals(value: &Value) -> Option<UsageTotals> {
    let total_usage = value.get("total_token_usage")?;
    Some(UsageTotals {
        input_tokens: total_usage.get("input_tokens")?.as_u64()?,
        cached_input_tokens: total_usage
            .get("cached_input_tokens")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        output_tokens: total_usage.get("output_tokens")?.as_u64()?,
        reasoning_output_tokens: total_usage
            .get("reasoning_output_tokens")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        total_tokens: total_usage.get("total_tokens")?.as_u64()?,
    })
}

fn context_used_from_info(value: &Value) -> Option<u64> {
    let last_usage = value
        .get("last_token_usage")
        .and_then(|usage| usage.get("total_tokens"))
        .and_then(|value| value.as_u64());
    if let Some(last) = last_usage {
        return Some(last);
    }
    value
        .get("total_token_usage")
        .and_then(|usage| usage.get("total_tokens"))
        .and_then(|value| value.as_u64())
}

fn parse_context_status(value: &Value) -> Option<ContextStatus> {
    let context_used = context_used_from_info(value)?;
    let context_window = value.get("model_context_window")?.as_u64()?;
    Some(ContextStatus {
        context_used,
        context_window,
    })
}

fn parse_context_status_optional(value: &Value) -> ContextStatus {
    let context_used = context_used_from_info(value).unwrap_or(0);
    let context_window = value
        .get("model_context_window")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    ContextStatus {
        context_used,
        context_window,
    }
}

fn find_string<'a>(value: &'a Value, paths: &[&[&str]]) -> Option<&'a str> {
    for path in paths {
        let mut current = value;
        let mut ok = true;
        for key in *path {
            if let Some(next) = current.get(*key) {
                current = next;
            } else {
                ok = false;
                break;
            }
        }
        if ok && let Some(found) = current.as_str() {
            return Some(found);
        }
    }
    None
}

fn normalize_timestamp(raw: &str) -> Option<String> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Some(
            parsed
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        );
    }
    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        let dt = DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
        return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
    }
    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        let dt = DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
        return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
    }
    if raw.chars().all(|ch| ch.is_ascii_digit())
        && let Ok(value) = raw.parse::<i64>()
    {
        let (secs, nanos) = if raw.len() > 10 {
            (
                value / 1000,
                (value % 1000).unsigned_abs() as u32 * 1_000_000,
            )
        } else {
            (value, 0)
        };
        if let Some(dt) = DateTime::<Utc>::from_timestamp(secs, nanos) {
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
    }
    None
}

fn extract_timestamp(value: &Value) -> Option<String> {
    find_string(value, &[&["timestamp"], &["ts"], &["time"]]).and_then(normalize_timestamp)
}

pub(crate) fn extract_model(value: &Value) -> Option<String> {
    find_string(
        value,
        &[
            &["model"],
            &["payload", "model"],
            &["payload", "info", "model"],
            &["payload", "info", "model_name"],
            &["payload", "info", "model_id"],
        ],
    )
    .map(str::to_string)
}

pub(crate) fn parse_json_line(line: &str) -> Option<Value> {
    serde_json::from_str(line).ok()
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

fn rule_matches_event(rule: &PricingRule, model: &str, ts: &str) -> bool {
    if !model_matches_pattern(model, &rule.model_pattern) {
        return false;
    }
    if rule.effective_from.as_str() > ts {
        return false;
    }
    if let Some(ref end) = rule.effective_to
        && ts >= end.as_str()
    {
        return false;
    }
    true
}

pub(crate) fn compute_cost_for_event(
    pricing: &[PricingRule],
    event: &UsageEvent,
    delta: UsageTotals,
) -> Option<f64> {
    let rule = pricing
        .iter()
        .filter(|rule| rule_matches_event(rule, &event.model, &event.ts))
        .max_by(|a, b| a.effective_from.cmp(&b.effective_from))?;
    Some(compute_cost_breakdown(delta, rule).total_cost_usd)
}

pub(crate) fn extract_effort_if_turn_context(value: &Value) -> Option<String> {
    let payload_type = value
        .get("payload")
        .and_then(|payload| payload.get("type"))
        .and_then(|value| value.as_str());
    let top_type = value.get("type").and_then(|value| value.as_str());
    if payload_type != Some("turn_context") && top_type != Some("turn_context") {
        return None;
    }
    extract_effort(value)
}

fn extract_request_id(value: &Value) -> Option<String> {
    find_string(
        value,
        &[
            &["request_id"],
            &["requestId"],
            &["payload", "request_id"],
            &["payload", "requestId"],
            &["payload", "info", "request_id"],
            &["payload", "info", "requestId"],
        ],
    )
    .map(str::to_string)
}

fn extract_role(value: &Value) -> Option<String> {
    find_string(
        value,
        &[
            &["role"],
            &["info", "role"],
            &["author", "role"],
            &["info", "author", "role"],
        ],
    )
    .map(str::to_string)
}

fn value_to_string(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(number) = value.as_i64() {
        return Some(number.to_string());
    }
    None
}

fn value_to_f64(value: &Value) -> Option<f64> {
    if let Some(value) = value.as_f64() {
        return Some(value);
    }
    if let Some(value) = value.as_i64() {
        return Some(value as f64);
    }
    if let Some(value) = value.as_u64() {
        return Some(value as f64);
    }
    if let Some(value) = value.as_str() {
        return value.parse::<f64>().ok();
    }
    None
}

fn extract_effort(value: &Value) -> Option<String> {
    let effort = find_string(
        value,
        &[
            &["turn_context", "effort"],
            &["payload", "info", "effort"],
            &["payload", "effort"],
            &["effort"],
            &["usage", "effort"],
            &["usage", "reasoning_effort"],
            &["payload", "usage", "effort"],
            &["payload", "usage", "reasoning_effort"],
        ],
    )
    .map(str::to_string);
    if effort.is_some() {
        return effort;
    }
    value
        .get("turn_context")
        .and_then(|value| value.get("effort"))
        .and_then(value_to_string)
}

fn normalize_percent(value: f64) -> f64 {
    let mut percent = if value <= 1.0 { value * 100.0 } else { value };
    if percent.is_nan() {
        return 0.0;
    }
    percent = percent.clamp(0.0, 100.0);
    percent
}

fn normalize_reset_at(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_second(0)
        .and_then(|value| value.with_nanosecond(0))
        .unwrap_or(dt)
}

fn parse_reset_at(raw: &Value, reference_ts: &str) -> Option<String> {
    let reference = DateTime::parse_from_rfc3339(reference_ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))?;
    if let Some(value) = raw.as_str() {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
            let dt = normalize_reset_at(parsed.with_timezone(&Utc));
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
            let dt = normalize_reset_at(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            let time = chrono::NaiveTime::from_hms_opt(0, 0, 0)?;
            let parsed = chrono::NaiveDateTime::new(date, time);
            let dt = normalize_reset_at(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y/%m/%d") {
            let time = chrono::NaiveTime::from_hms_opt(0, 0, 0)?;
            let parsed = chrono::NaiveDateTime::new(date, time);
            let dt = normalize_reset_at(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        if let Ok(time) = chrono::NaiveTime::parse_from_str(value, "%H:%M:%S") {
            let mut parsed = chrono::NaiveDateTime::new(reference.date_naive(), time);
            let mut dt = DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
            if dt <= reference {
                parsed += chrono::Duration::days(1);
                dt = DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
            }
            let dt = normalize_reset_at(dt);
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        if let Ok(time) = chrono::NaiveTime::parse_from_str(value, "%H:%M") {
            let mut parsed = chrono::NaiveDateTime::new(reference.date_naive(), time);
            let mut dt = DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
            if dt <= reference {
                parsed += chrono::Duration::days(1);
                dt = DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc);
            }
            let dt = normalize_reset_at(dt);
            return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        return None;
    }
    let value = value_to_f64(raw)?;
    let seconds = if value > 1_000_000_000_000.0 {
        value / 1000.0
    } else {
        value
    };
    let secs = seconds as i64;
    let nanos = ((seconds - secs as f64) * 1_000_000_000.0) as u32;
    let parsed = DateTime::<Utc>::from_timestamp(secs, nanos)?;
    let dt = normalize_reset_at(parsed);
    Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn extract_rate_limits(value: &Value) -> Option<&Value> {
    value
        .get("payload")
        .and_then(|payload| payload.get("rate_limits"))
        .or_else(|| {
            value
                .get("payload")
                .and_then(|payload| payload.get("info"))
                .and_then(|info| info.get("rate_limits"))
        })
        .or_else(|| value.get("rate_limits"))
}

fn extract_percent_left(limit: &Value) -> Option<f64> {
    for key in [
        "percent_left",
        "remaining_percent",
        "remaining_pct",
        "percent_remaining",
        "remaining",
    ] {
        if let Some(value) = limit.get(key).and_then(value_to_f64) {
            return Some(normalize_percent(value));
        }
    }
    for key in ["used_percent", "used_pct", "percent_used", "used"] {
        if let Some(value) = limit.get(key).and_then(value_to_f64) {
            let used = normalize_percent(value);
            return Some(normalize_percent(100.0 - used));
        }
    }
    None
}

fn extract_reset_at(limit: &Value, reference_ts: &str) -> Option<String> {
    for key in [
        "reset_at",
        "resets_at",
        "resetAt",
        "reset",
        "reset_time",
        "resetTime",
    ] {
        if let Some(value) = limit.get(key)
            && let Some(reset_at) = parse_reset_at(value, reference_ts)
        {
            return Some(reset_at);
        }
    }
    None
}

fn limit_type_label(key: &str) -> Option<&'static str> {
    match key {
        "primary" => Some("5h"),
        "secondary" => Some("7d"),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn extract_limit_snapshots_from_line(
    line: &str,
    source: &str,
) -> Vec<UsageLimitSnapshot> {
    let Some(obj) = parse_json_line(line) else {
        return Vec::new();
    };
    extract_limit_snapshots_from_value(&obj, line, source)
}

pub(crate) fn extract_limit_snapshots_from_value(
    obj: &Value,
    line: &str,
    source: &str,
) -> Vec<UsageLimitSnapshot> {
    let rate_limits = match extract_rate_limits(obj) {
        Some(value) => value,
        None => return Vec::new(),
    };
    let observed_at = match extract_timestamp(obj) {
        Some(value) => value,
        None => return Vec::new(),
    };
    let mut snapshots = Vec::new();
    if let Some(map) = rate_limits.as_object() {
        for (key, value) in map {
            let Some(limit_type) = limit_type_label(key) else {
                continue;
            };
            let percent_left = match extract_percent_left(value) {
                Some(value) => value,
                None => continue,
            };
            let reset_at = match extract_reset_at(value, &observed_at) {
                Some(value) => value,
                None => continue,
            };
            snapshots.push(UsageLimitSnapshot {
                limit_type: limit_type.to_string(),
                percent_left,
                reset_at,
                observed_at: observed_at.clone(),
                source: source.to_string(),
                raw_line: Some(line.to_string()),
            });
        }
    }
    snapshots
}

#[cfg(test)]
pub(crate) fn extract_message_event_from_line(
    line: &str,
    source: &str,
    session_id: &str,
) -> Option<MessageEvent> {
    let obj = parse_json_line(line)?;
    extract_message_event_from_value(&obj, line, source, session_id)
}

pub(crate) fn extract_message_event_from_value(
    obj: &Value,
    line: &str,
    source: &str,
    session_id: &str,
) -> Option<MessageEvent> {
    let top_type = obj.get("type").and_then(|value| value.as_str());
    let (payload_type, info) = if top_type == Some("event_msg") {
        let payload = obj.get("payload")?;
        let payload_type = payload.get("type").and_then(|value| value.as_str());
        let info = payload.get("info").unwrap_or(payload);
        (payload_type, info)
    } else {
        (top_type, obj)
    };
    if payload_type != Some("user_message") && payload_type != Some("message") {
        return None;
    }
    let mut role = extract_role(info);
    if role.is_none() && payload_type == Some("user_message") {
        role = Some("user".to_string());
    }
    let role = role?;
    if !role.eq_ignore_ascii_case("user") {
        return None;
    }
    let ts = extract_timestamp(obj).or_else(|| extract_timestamp(info))?;
    let id = hash_line(source, line);
    Some(MessageEvent {
        id,
        ts,
        role,
        source: source.to_string(),
        session_id: session_id.to_string(),
        raw_json: Some(line.to_string()),
    })
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut out, "{:02x}", byte);
    }
    out
}

fn hash_line(source: &str, line: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(b":");
    hasher.update(line.as_bytes());
    hex_digest(&hasher.finalize())
}

pub fn extract_token_totals_from_line(line: &str) -> Option<TokenTotals> {
    let obj = parse_json_line(line)?;
    if obj.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = obj.get("payload")?;
    if payload.get("type")?.as_str()? != "token_count" {
        return None;
    }
    let info = payload.get("info")?;
    if info.is_null() {
        return None;
    }
    parse_token_totals(info)
}

pub fn extract_usage_totals_from_line(line: &str) -> Option<UsageTotals> {
    let obj = parse_json_line(line)?;
    if obj.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = obj.get("payload")?;
    if payload.get("type")?.as_str()? != "token_count" {
        return None;
    }
    let info = payload.get("info")?;
    if info.is_null() {
        return None;
    }
    parse_usage_totals(info)
}

pub fn extract_context_from_line(line: &str) -> Option<ContextStatus> {
    let obj = parse_json_line(line)?;
    if obj.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = obj.get("payload")?;
    if payload.get("type")?.as_str()? != "token_count" {
        return None;
    }
    let info = payload.get("info")?;
    if info.is_null() {
        return None;
    }
    parse_context_status(info)
}

pub fn extract_usage_event_from_line(
    line: &str,
    source: &str,
    fallback_model: Option<&str>,
    session_id: &str,
    reasoning_effort: Option<&str>,
) -> Option<UsageEvent> {
    let obj = parse_json_line(line)?;
    extract_usage_event_from_value(
        &obj,
        line,
        source,
        fallback_model,
        session_id,
        reasoning_effort,
    )
}

pub(crate) fn extract_usage_event_from_value(
    obj: &Value,
    line: &str,
    source: &str,
    fallback_model: Option<&str>,
    session_id: &str,
    reasoning_effort: Option<&str>,
) -> Option<UsageEvent> {
    if obj.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = obj.get("payload")?;
    if payload.get("type")?.as_str()? != "token_count" {
        return None;
    }
    let info = payload.get("info")?;
    if info.is_null() {
        return None;
    }
    let usage = parse_usage_totals(info)?;
    let ts = extract_timestamp(obj)?;
    let model = extract_model(obj)
        .or_else(|| fallback_model.map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());
    let request_id = extract_request_id(obj);
    let context = parse_context_status_optional(info);
    let id = hash_line(source, line);
    let effort = reasoning_effort
        .map(|value| value.to_string())
        .or_else(|| extract_effort(obj));

    Some(UsageEvent {
        id,
        ts,
        model,
        usage,
        context,
        cost_usd: None,
        reasoning_effort: effort,
        source: source.to_string(),
        session_id: session_id.to_string(),
        request_id,
        raw_json: Some(line.to_string()),
    })
}

pub fn usage_events_from_reader<R: std::io::BufRead>(reader: R, source: &str) -> Vec<UsageEvent> {
    let mut current_model: Option<String> = None;
    let mut current_effort: Option<String> = None;
    let session_id = session_id_from_source(source);
    reader
        .lines()
        .map_while(|line| line.ok())
        .filter_map(|line| {
            let obj = parse_json_line(&line)?;
            if let Some(model) = extract_model(&obj) {
                current_model = Some(model);
            }
            if let Some(effort) = extract_effort_if_turn_context(&obj) {
                current_effort = Some(effort);
            }
            extract_usage_event_from_value(
                &obj,
                &line,
                source,
                current_model.as_deref(),
                &session_id,
                current_effort.as_deref(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_totals() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":5115,"cached_input_tokens":0,"output_tokens":21,"reasoning_output_tokens":0,"total_tokens":5136},"last_token_usage":{"input_tokens":5115,"cached_input_tokens":0,"output_tokens":21,"reasoning_output_tokens":0,"total_tokens":5136},"model_context_window":258400}}}"#;
        let totals = extract_token_totals_from_line(line).expect("totals");
        assert_eq!(
            totals,
            TokenTotals {
                total_tokens: 5136,
                last_tokens: 5136,
            }
        );
    }

    #[test]
    fn ignores_non_token_lines() {
        let line = r#"{"timestamp":"2025-12-19T21:31:32.694Z","type":"session_meta","payload":{"id":"abc"}}"#;
        assert!(extract_token_totals_from_line(line).is_none());
    }

    #[test]
    fn extracts_token_totals_without_last_usage() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":5136}}}}"#;
        let totals = extract_token_totals_from_line(line).expect("totals");
        assert_eq!(
            totals,
            TokenTotals {
                total_tokens: 5136,
                last_tokens: 0,
            }
        );
    }

    #[test]
    fn extracts_usage_totals_with_missing_fields() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"output_tokens":2,"total_tokens":12}}}}"#;
        let totals = extract_usage_totals_from_line(line).expect("totals");
        assert_eq!(
            totals,
            UsageTotals {
                input_tokens: 10,
                cached_input_tokens: 0,
                output_tokens: 2,
                reasoning_output_tokens: 0,
                total_tokens: 12,
            }
        );
    }

    #[test]
    fn extracts_usage_event() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}"#;
        let event = extract_usage_event_from_line(line, "test.log", None, "session-1", None)
            .expect("event");
        assert_eq!(event.ts, "2025-12-19T21:31:36.168Z");
        assert_eq!(event.model, "gpt-test");
        assert_eq!(event.context.context_used, 12);
        assert_eq!(event.context.context_window, 100);
        assert_eq!(event.usage.total_tokens, 12);
        assert_eq!(event.source, "test.log");
        assert_eq!(event.session_id, "session-1");
        assert!(event.raw_json.is_some());
    }

    #[test]
    fn normalizes_timestamp_to_utc() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36+02:00","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}"#;
        let event = extract_usage_event_from_line(line, "test.log", None, "session-1", None)
            .expect("event");
        assert_eq!(event.ts, "2025-12-19T19:31:36.000Z");
    }

    #[test]
    fn extracts_usage_event_with_fallback_model() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}"#;
        let event =
            extract_usage_event_from_line(line, "test.log", Some("gpt-5.2"), "session-1", None)
                .expect("event");
        assert_eq!(event.model, "gpt-5.2");
    }

    #[test]
    fn usage_events_from_reader_uses_session_model() {
        let input = r#"
{"type":"session_meta","payload":{"info":{"model":"gpt-5.2"}}}
{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}
"#;
        let events = usage_events_from_reader(input.trim().as_bytes(), "test.log");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model, "gpt-5.2");
    }

    #[test]
    fn context_uses_last_token_usage_when_present() {
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":90,"output_tokens":10,"total_tokens":100},"last_token_usage":{"total_tokens":12},"model_context_window":200}}}"#;
        let event = extract_usage_event_from_line(line, "test.log", None, "session-1", None)
            .expect("event");
        assert_eq!(event.context.context_used, 12);
        assert_eq!(event.context.context_window, 200);
    }

    #[test]
    fn usage_events_capture_latest_effort() {
        let input = r#"
{"type":"event_msg","payload":{"type":"turn_context","info":{"effort":"high"}}}
{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}
"#;
        let events = usage_events_from_reader(input.trim().as_bytes(), "test.log");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].reasoning_effort.as_deref(), Some("high"));
    }

    #[test]
    fn usage_events_fallback_effort_from_token_line() {
        let input = r#"
{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"effort":"low","total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}
"#;
        let events = usage_events_from_reader(input.trim().as_bytes(), "test.log");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].reasoning_effort.as_deref(), Some("low"));
    }

    #[test]
    fn extracts_user_message_event() {
        let line = r#"{"timestamp":"2025-01-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","info":{"role":"user","content":"Hello"}}}"#;
        let event =
            extract_message_event_from_line(line, "test.log", "session-1").expect("message event");
        assert_eq!(event.ts, "2025-01-01T00:00:00.000Z");
        assert_eq!(event.role, "user");
        assert_eq!(event.source, "test.log");
        assert_eq!(event.session_id, "session-1");
    }

    #[test]
    fn extracts_user_message_without_role() {
        let line = r#"{"timestamp":"2025-01-01T00:00:00Z","type":"event_msg","payload":{"type":"user_message","info":{"content":"Hello"}}}"#;
        let event =
            extract_message_event_from_line(line, "test.log", "session-1").expect("message event");
        assert_eq!(event.role, "user");
    }

    #[test]
    fn ignores_non_user_message_role() {
        let line = r#"{"timestamp":"2025-01-01T00:00:00Z","type":"event_msg","payload":{"type":"message","info":{"role":"assistant","content":"Hi"}}}"#;
        assert!(extract_message_event_from_line(line, "test.log", "session-1").is_none());
    }

    #[test]
    fn extracts_limit_snapshots_from_rate_limits() {
        let line = r#"{"timestamp":"2025-01-01T00:00:00Z","type":"event_msg","payload":{"rate_limits":{"primary":{"used_percent":0.25,"resets_at":"2025-01-01T05:00:00Z"},"secondary":{"remaining_percent":40,"resets_at":"2025-01-08T00:00:00Z"}}}}"#;
        let snapshots = extract_limit_snapshots_from_line(line, "test.log");
        assert_eq!(snapshots.len(), 2);
        let primary = snapshots
            .iter()
            .find(|snap| snap.limit_type == "5h")
            .expect("primary");
        assert!((primary.percent_left - 75.0).abs() < 1e-6);
        assert_eq!(primary.reset_at, "2025-01-01T05:00:00.000Z");
        let secondary = snapshots
            .iter()
            .find(|snap| snap.limit_type == "7d")
            .expect("secondary");
        assert!((secondary.percent_left - 40.0).abs() < 1e-6);
        assert_eq!(secondary.reset_at, "2025-01-08T00:00:00.000Z");
    }

    #[test]
    fn limit_snapshot_parses_time_only_reset() {
        let line = r#"{"timestamp":"2025-01-01T04:00:00Z","type":"event_msg","payload":{"rate_limits":{"primary":{"remaining":0.5,"resets_at":"05:30"}}}}"#;
        let snapshots = extract_limit_snapshots_from_line(line, "test.log");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].reset_at, "2025-01-01T05:30:00.000Z");
        assert!((snapshots[0].percent_left - 50.0).abs() < 1e-6);
    }
}
