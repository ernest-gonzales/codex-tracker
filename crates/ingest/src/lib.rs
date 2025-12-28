use std::fmt::Write;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration as StdDuration, Instant};
use std::{env, fs};

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracker_core::{
    ContextStatus, MessageEvent, PricingRule, UsageEvent, UsageLimitSnapshot, UsageTotals,
    compute_cost_breakdown, model_matches_pattern, session_id_from_source,
};
use tracker_db::{Db, IngestCursor};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenTotals {
    pub total_tokens: u64,
    pub last_tokens: u64,
}

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
    if raw.chars().all(|ch| ch.is_ascii_digit()) {
        if let Ok(value) = raw.parse::<i64>() {
            let (secs, nanos) = if raw.len() > 10 {
                (value / 1000, ((value % 1000).abs() as u32) * 1_000_000)
            } else {
                (value, 0)
            };
            if let Some(dt) = DateTime::<Utc>::from_timestamp(secs, nanos) {
                return Some(dt.to_rfc3339_opts(SecondsFormat::Millis, true));
            }
        }
    }
    None
}

fn extract_timestamp(value: &Value) -> Option<String> {
    find_string(value, &[&["timestamp"], &["ts"], &["time"]])
        .and_then(|raw| normalize_timestamp(raw))
}

fn extract_model(value: &Value) -> Option<String> {
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

fn parse_json_line(line: &str) -> Option<Value> {
    serde_json::from_str(line).ok()
}

fn delta_usage(prev: Option<&UsageTotals>, current: UsageTotals) -> UsageTotals {
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

fn compute_cost_for_event(
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

fn extract_effort_if_turn_context(value: &Value) -> Option<String> {
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
    if let Some(number) = value.as_u64() {
        return Some(number.to_string());
    }
    if let Some(number) = value.as_f64() {
        return Some(number.to_string());
    }
    None
}

fn extract_effort(value: &Value) -> Option<String> {
    for path in [
        &["payload", "info", "effort"][..],
        &["payload", "info", "reasoning_effort"],
        &["payload", "info", "reasoning", "effort"],
        &["payload", "effort"],
        &["payload", "turn_context", "effort"],
        &["turn_context", "effort"],
    ] {
        let mut current = value;
        let mut ok = true;
        for key in path {
            if let Some(next) = current.get(*key) {
                current = next;
            } else {
                ok = false;
                break;
            }
        }
        if ok && let Some(value) = value_to_string(current) {
            return Some(value);
        }
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
fn extract_limit_snapshots_from_line(line: &str, source: &str) -> Vec<UsageLimitSnapshot> {
    let Some(obj) = parse_json_line(line) else {
        return Vec::new();
    };
    extract_limit_snapshots_from_value(&obj, line, source)
}

fn extract_limit_snapshots_from_value(
    obj: &Value,
    line: &str,
    source: &str,
) -> Vec<UsageLimitSnapshot> {
    let rate_limits = match extract_rate_limits(&obj) {
        Some(value) => value,
        None => return Vec::new(),
    };
    let observed_at = match extract_timestamp(&obj) {
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
fn extract_message_event_from_line(
    line: &str,
    source: &str,
    session_id: &str,
) -> Option<MessageEvent> {
    let obj = parse_json_line(line)?;
    extract_message_event_from_value(&obj, line, source, session_id)
}

fn extract_message_event_from_value(
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
    let ts = extract_timestamp(&obj).or_else(|| extract_timestamp(info))?;
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

fn extract_usage_event_from_value(
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

pub fn usage_events_from_reader<R: BufRead>(reader: R, source: &str) -> Vec<UsageEvent> {
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

pub fn total_from_totals<I>(totals: I) -> Option<u64>
where
    I: IntoIterator<Item = u64>,
{
    let mut iter = totals.into_iter();
    let first = iter.next()?;
    let mut segment_max = first;
    let mut sum = 0u64;

    for total in iter {
        if total >= segment_max {
            segment_max = total;
        } else {
            sum = sum.saturating_add(segment_max);
            segment_max = total;
        }
    }

    Some(sum.saturating_add(segment_max))
}

pub fn total_from_reader<R: BufRead>(reader: R) -> Option<u64> {
    let totals = reader
        .lines()
        .map_while(|line| line.ok())
        .filter_map(|line| extract_token_totals_from_line(&line))
        .map(|totals| totals.total_tokens);
    total_from_totals(totals)
}

fn max_usage(a: UsageTotals, b: UsageTotals) -> UsageTotals {
    UsageTotals {
        input_tokens: a.input_tokens.max(b.input_tokens),
        cached_input_tokens: a.cached_input_tokens.max(b.cached_input_tokens),
        output_tokens: a.output_tokens.max(b.output_tokens),
        reasoning_output_tokens: a.reasoning_output_tokens.max(b.reasoning_output_tokens),
        total_tokens: a.total_tokens.max(b.total_tokens),
    }
}

fn add_usage(a: UsageTotals, b: UsageTotals) -> UsageTotals {
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

pub fn totals_from_usage<I>(totals: I) -> Option<UsageTotals>
where
    I: IntoIterator<Item = UsageTotals>,
{
    let mut iter = totals.into_iter();
    let first = iter.next()?;
    let mut segment_max = first;
    let mut sum = UsageTotals::default();

    for usage in iter {
        if usage.total_tokens >= segment_max.total_tokens {
            segment_max = max_usage(segment_max, usage);
        } else {
            sum = add_usage(sum, segment_max);
            segment_max = usage;
        }
    }

    Some(add_usage(sum, segment_max))
}

pub fn usage_totals_from_reader<R: BufRead>(reader: R) -> Option<UsageTotals> {
    let totals = reader
        .lines()
        .map_while(|line| line.ok())
        .filter_map(|line| extract_usage_totals_from_line(&line));
    totals_from_usage(totals)
}

pub fn latest_context_from_reader<R: BufRead>(reader: R) -> Option<ContextStatus> {
    let mut last = None;
    for line in reader.lines().map_while(|line| line.ok()) {
        if let Some(context) = extract_context_from_line(&line) {
            last = Some(context);
        }
    }
    last
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IngestStats {
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub events_inserted: usize,
    pub bytes_read: u64,
    pub issues: Vec<IngestIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestIssue {
    pub file_path: String,
    pub message: String,
}

#[derive(Debug)]
pub enum IngestError {
    Io(io::Error),
    Db(tracker_db::DbError),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {}", err),
            Self::Db(err) => write!(f, "db error: {}", err),
        }
    }
}

impl From<io::Error> for IngestError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<tracker_db::DbError> for IngestError {
    fn from(err: tracker_db::DbError) -> Self {
        Self::Db(err)
    }
}

pub type Result<T> = std::result::Result<T, IngestError>;

pub fn default_codex_home() -> PathBuf {
    if let Ok(path) = env::var("CODEX_HOME") {
        return PathBuf::from(path);
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".codex");
    }
    PathBuf::from(".codex")
}

fn is_log_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("log") | Some("jsonl") | Some("ndjson")
    )
}

fn is_plain_log(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("log")
    )
}

fn looks_like_jsonl(file: &mut File) -> io::Result<bool> {
    file.seek(SeekFrom::Start(0))?;
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    for _ in 0..5 {
        buf.clear();
        if reader.read_line(&mut buf)? == 0 {
            break;
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        return Ok(line.starts_with('{'));
    }
    Ok(false)
}

pub fn ingest_codex_home(db: &mut Db, codex_home: &Path) -> Result<IngestStats> {
    let mut stats = IngestStats::default();
    let pricing = db.list_pricing_rules()?;
    let has_pricing = !pricing.is_empty();
    let timing_enabled = env::var("CODEX_TRACKER_INGEST_TIMING").is_ok();
    let ingest_start = Instant::now();
    let mut parse_total = StdDuration::ZERO;
    let mut db_total = StdDuration::ZERO;
    let codex_home_str = codex_home.to_string_lossy().to_string();
    let home = db.get_or_create_home(&codex_home_str, Some("Default"))?;
    db.update_home_last_seen(home.id)?;
    for entry in WalkDir::new(codex_home).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let file_path = err
                    .path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| "<unknown>".to_string());
                stats.issues.push(IngestIssue {
                    file_path: file_path.clone(),
                    message: err.to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        if !entry.file_type().is_file() || !is_log_path(path) {
            continue;
        }
        stats.files_scanned += 1;
        let file_path = path.to_string_lossy().to_string();
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(err) => {
                stats.files_skipped += 1;
                stats.issues.push(IngestIssue {
                    file_path: file_path.clone(),
                    message: err.to_string(),
                });
                continue;
            }
        };
        let file_len = metadata.len();
        let inode = inode_from_metadata(&metadata);
        let mtime = metadata
            .modified()
            .ok()
            .map(|time| DateTime::<Utc>::from(time).to_rfc3339());
        let cursor = db.get_cursor(home.id, &file_path)?;
        let can_resume = matches!(
            cursor.as_ref(),
            Some(cursor) if cursor.byte_offset <= file_len && inode == cursor.inode
        );
        let (start_offset, seed_model, seed_effort) = match cursor.as_ref() {
            Some(cursor) if can_resume => (
                cursor.byte_offset,
                cursor.last_model.clone(),
                cursor.last_effort.clone(),
            ),
            _ => (0, None, None),
        };
        if start_offset >= file_len {
            stats.files_skipped += 1;
            continue;
        }
        let mut prev_usage = if can_resume {
            db.last_usage_totals_for_source(home.id, &file_path)?
        } else {
            None
        };
        let file_start = Instant::now();
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(err) => {
                stats.files_skipped += 1;
                stats.issues.push(IngestIssue {
                    file_path: file_path.clone(),
                    message: err.to_string(),
                });
                continue;
            }
        };
        if is_plain_log(path) {
            match looks_like_jsonl(&mut file) {
                Ok(true) => {}
                Ok(false) => {
                    stats.files_skipped += 1;
                    continue;
                }
                Err(err) => {
                    stats.files_skipped += 1;
                    stats.issues.push(IngestIssue {
                        file_path: file_path.clone(),
                        message: err.to_string(),
                    });
                    continue;
                }
            }
        }
        if let Err(err) = file.seek(SeekFrom::Start(start_offset)) {
            stats.files_skipped += 1;
            stats.issues.push(IngestIssue {
                file_path: file_path.clone(),
                message: err.to_string(),
            });
            continue;
        }
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        let mut bytes_read = 0u64;
        let mut events = Vec::new();
        let mut limit_snapshots = Vec::new();
        let mut message_events = Vec::new();
        let mut current_model: Option<String> = seed_model;
        let mut current_effort: Option<String> = seed_effort;
        let session_id = session_id_from_source(&file_path);
        loop {
            match reader.read_line(&mut buf) {
                Ok(0) => break,
                Ok(bytes) => {
                    bytes_read = bytes_read.saturating_add(bytes as u64);
                    let line = buf.trim_end_matches(&['\n', '\r'][..]);
                    let Some(obj) = parse_json_line(line) else {
                        buf.clear();
                        continue;
                    };
                    if let Some(model) = extract_model(&obj) {
                        current_model = Some(model);
                    }
                    if let Some(effort) = extract_effort_if_turn_context(&obj) {
                        current_effort = Some(effort);
                    }
                    if let Some(mut event) = extract_usage_event_from_value(
                        &obj,
                        line,
                        &file_path,
                        current_model.as_deref(),
                        &session_id,
                        current_effort.as_deref(),
                    ) {
                        let delta = delta_usage(prev_usage.as_ref(), event.usage);
                        if has_pricing {
                            if let Some(cost) = compute_cost_for_event(&pricing, &event, delta) {
                                event.cost_usd = Some(cost);
                            }
                        }
                        prev_usage = Some(event.usage);
                        events.push(event);
                    }
                    if let Some(event) =
                        extract_message_event_from_value(&obj, line, &file_path, &session_id)
                    {
                        message_events.push(event);
                    }
                    let mut snapshots =
                        extract_limit_snapshots_from_value(&obj, line, &file_path);
                    if !snapshots.is_empty() {
                        limit_snapshots.append(&mut snapshots);
                    }
                    buf.clear();
                }
                Err(err) => {
                    stats.issues.push(IngestIssue {
                        file_path: file_path.clone(),
                        message: err.to_string(),
                    });
                    break;
                }
            }
        }
        let parse_done = Instant::now();
        stats.bytes_read += bytes_read;
        if !events.is_empty() {
            stats.events_inserted += db.insert_usage_events(home.id, &events)?;
        }
        if !message_events.is_empty() {
            let _ = db.insert_message_events(home.id, &message_events)?;
        }
        if !limit_snapshots.is_empty() {
            let _ = db.insert_limit_snapshots(home.id, &limit_snapshots)?;
        }
        let new_cursor = IngestCursor {
            codex_home_id: home.id,
            codex_home: codex_home_str.clone(),
            file_path,
            inode,
            mtime,
            byte_offset: start_offset.saturating_add(bytes_read),
            last_event_key: events.last().map(|event| event.id.clone()),
            updated_at: Utc::now().to_rfc3339(),
            last_model: current_model,
            last_effort: current_effort,
        };
        db.upsert_cursor(&new_cursor)?;
        let file_done = Instant::now();
        parse_total += parse_done.saturating_duration_since(file_start);
        db_total += file_done.saturating_duration_since(parse_done);
        if timing_enabled {
            eprintln!(
                "ingest file: {} read={}ms db={}ms events={} bytes={}",
                new_cursor.file_path,
                parse_done.duration_since(file_start).as_millis(),
                file_done.duration_since(parse_done).as_millis(),
                events.len(),
                bytes_read
            );
        }
    }
    if timing_enabled {
        eprintln!(
            "ingest total: files={} scanned={} skipped={} events={} read={}ms db={}ms total={}ms",
            stats.files_scanned + stats.files_skipped,
            stats.files_scanned,
            stats.files_skipped,
            stats.events_inserted,
            parse_total.as_millis(),
            db_total.as_millis(),
            ingest_start.elapsed().as_millis()
        );
    }
    Ok(stats)
}

fn inode_from_metadata(metadata: &fs::Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(metadata.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;
    use tracker_core::TimeRange;
    use tracker_db::Db;

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
    fn total_from_totals_monotonic() {
        let totals = vec![100, 200, 350];
        assert_eq!(total_from_totals(totals), Some(350));
    }

    #[test]
    fn totals_from_usage_monotonic() {
        let totals = vec![
            UsageTotals {
                input_tokens: 10,
                cached_input_tokens: 0,
                output_tokens: 2,
                reasoning_output_tokens: 0,
                total_tokens: 12,
            },
            UsageTotals {
                input_tokens: 20,
                cached_input_tokens: 5,
                output_tokens: 4,
                reasoning_output_tokens: 0,
                total_tokens: 24,
            },
        ];
        let result = totals_from_usage(totals).expect("totals");
        assert_eq!(result.total_tokens, 24);
        assert_eq!(result.input_tokens, 20);
        assert_eq!(result.cached_input_tokens, 5);
        assert_eq!(result.output_tokens, 4);
    }

    #[test]
    fn total_from_totals_with_reset() {
        let totals = vec![100, 200, 50, 80, 40];
        assert_eq!(total_from_totals(totals), Some(200 + 80 + 40));
    }

    #[test]
    fn totals_from_usage_with_reset() {
        let totals = vec![
            UsageTotals {
                input_tokens: 10,
                cached_input_tokens: 0,
                output_tokens: 2,
                reasoning_output_tokens: 0,
                total_tokens: 12,
            },
            UsageTotals {
                input_tokens: 30,
                cached_input_tokens: 0,
                output_tokens: 3,
                reasoning_output_tokens: 0,
                total_tokens: 33,
            },
            UsageTotals {
                input_tokens: 5,
                cached_input_tokens: 0,
                output_tokens: 1,
                reasoning_output_tokens: 0,
                total_tokens: 6,
            },
        ];
        let result = totals_from_usage(totals).expect("totals");
        assert_eq!(result.total_tokens, 33 + 6);
        assert_eq!(result.input_tokens, 30 + 5);
        assert_eq!(result.output_tokens, 3 + 1);
    }

    #[test]
    fn total_from_reader_works() {
        let input = r#"
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":10},"last_token_usage":{"total_tokens":10}}}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":12},"last_token_usage":{"total_tokens":2}}}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":5},"last_token_usage":{"total_tokens":5}}}}
"#;
        let total = total_from_reader(input.trim().as_bytes()).expect("total");
        assert_eq!(total, 12 + 5);
    }

    #[test]
    fn usage_totals_from_reader_works() {
        let input = r#"
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"last_token_usage":{"total_tokens":12}}}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":12,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":0,"total_tokens":15},"last_token_usage":{"total_tokens":3}}}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":5,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":6},"last_token_usage":{"total_tokens":6}}}}
"#;
        let totals = usage_totals_from_reader(input.trim().as_bytes()).expect("totals");
        assert_eq!(totals.total_tokens, 15 + 6);
        assert_eq!(totals.input_tokens, 12 + 5);
        assert_eq!(totals.cached_input_tokens, 2);
        assert_eq!(totals.output_tokens, 3 + 1);
    }

    #[test]
    fn latest_context_from_reader_works() {
        let input = r#"
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":12},"model_context_window":100}}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":25},"model_context_window":100}}}
"#;
        let context = latest_context_from_reader(input.trim().as_bytes()).expect("context");
        assert_eq!(
            context,
            ContextStatus {
                context_used: 25,
                context_window: 100,
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
    fn ingest_resume_seeds_model_and_effort_from_cursor() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join("ingest.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");

        let log_dir = dir.path().join("sessions/2025/01/01");
        fs::create_dir_all(&log_dir).expect("create log dir");
        let log_path = log_dir.join("rollout-2025-01-01T00-00-00-1234.jsonl");
        let initial = r#"
{"type":"session_meta","payload":{"info":{"model":"gpt-5.2-codex"}}}
{"type":"event_msg","payload":{"type":"turn_context","info":{"effort":"high"}}}
{"timestamp":"2025-01-01T00:00:10Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":2},"model_context_window":100}}}
"#;
        fs::write(&log_path, initial.trim()).expect("write log");

        let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest");
        assert_eq!(stats.events_inserted, 1);

        let appended = r#"
{"timestamp":"2025-01-01T00:00:20Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":2,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":3},"model_context_window":100}}}
"#;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&log_path)
            .expect("open log");
        writeln!(file, "{}", appended.trim()).expect("append log");

        let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest again");
        assert_eq!(stats.events_inserted, 1);

        let home = db
            .get_home_by_path(&dir.path().to_string_lossy())
            .expect("home lookup")
            .expect("home");
        assert_eq!(db.count_usage_events(home.id).expect("count"), 2);
        let range = TimeRange {
            start: "0000-01-01T00:00:00Z".to_string(),
            end: "9999-12-31T23:59:59Z".to_string(),
        };
        let events = db
            .list_usage_events(&range, None, 10, 0, home.id)
            .expect("events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].model, "gpt-5.2-codex");
        assert_eq!(events[0].reasoning_effort.as_deref(), Some("high"));
    }

    #[test]
    fn ingest_does_not_advance_cursor_on_invalid_utf8() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join("ingest.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate db");

        let log_path = dir.path().join("bad.log");
        let line = r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0,"total_tokens":2},"model_context_window":100}}}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        bytes.push(0xff);
        fs::write(&log_path, bytes).expect("write log");

        let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest");
        assert_eq!(stats.events_inserted, 1);
        assert_eq!(stats.issues.len(), 1);

        let home = db
            .get_home_by_path(&dir.path().to_string_lossy())
            .expect("home lookup")
            .expect("home");
        let cursor = db
            .get_cursor(home.id, &log_path.to_string_lossy())
            .expect("cursor lookup")
            .expect("cursor");
        let expected_offset = (line.len() + 1) as u64;
        assert_eq!(cursor.byte_offset, expected_offset);
    }

    #[test]
    fn ingest_skips_plain_log_files() {
        let dir = tempdir().expect("tempdir");
        let log_path = dir.path().join("codex-tui.log");
        let json_path = dir.path().join("rollout-2025-12-19T21-31-36.jsonl");
        let db_path = dir.path().join("ingest.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate");
        fs::write(&log_path, "not json\nmore text\n").expect("write log");
        fs::write(
            &json_path,
            r#"{"timestamp":"2025-12-19T21:31:36.168Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":10,"cached_input_tokens":0,"output_tokens":2,"reasoning_output_tokens":0,"total_tokens":12},"model_context_window":100}}}"#,
        )
        .expect("write json");
        let stats = ingest_codex_home(&mut db, dir.path()).expect("ingest");
        assert_eq!(stats.events_inserted, 1);
        assert!(stats.files_skipped >= 1);
    }

    #[test]
    fn ingest_sets_cost_on_insert() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("ingest.sqlite");
        let mut db = Db::open(&db_path).expect("open db");
        db.migrate().expect("migrate");
        db.replace_pricing_rules(&[tracker_core::PricingRuleInput {
            model_pattern: "gpt-test".to_string(),
            input_per_1m: 1750.0,
            cached_input_per_1m: 175.0,
            output_per_1m: 14000.0,
            effective_from: "2025-01-01T00:00:00Z".to_string(),
            effective_to: None,
        }])
        .expect("pricing");

        let log_path = dir.path().join("rollout-2025-12-19T21-31-36.jsonl");
        fs::write(
            &log_path,
            r#"{"timestamp":"2025-12-19T19:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"model":"gpt-test","total_token_usage":{"input_tokens":1000,"cached_input_tokens":200,"output_tokens":300,"reasoning_output_tokens":0,"total_tokens":1500},"model_context_window":100}}}"#,
        )
        .expect("write json");

        ingest_codex_home(&mut db, dir.path()).expect("ingest");
        let home = db
            .get_home_by_path(dir.path().to_string_lossy().as_ref())
            .expect("get home")
            .expect("home");
        let range = TimeRange {
            start: "2025-12-19T18:40:00Z".to_string(),
            end: "2025-12-19T20:00:00Z".to_string(),
        };
        let events = db
            .list_usage_events(&range, None, 10, 0, home.id)
            .expect("list events");
        assert_eq!(events.len(), 1);
        let cost = events[0].cost_usd.expect("cost");
        let expected_input = (800.0 / 1_000_000.0) * 1750.0;
        let expected_cached = (200.0 / 1_000_000.0) * 175.0;
        let expected_output = (300.0 / 1_000_000.0) * 14000.0;
        let expected_total = expected_input + expected_cached + expected_output;
        assert!((cost - expected_total).abs() < 1e-9);
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
