use chrono::{DateTime, Datelike, Duration, Local, SecondsFormat, TimeZone, Utc};

use crate::config::RangeParams;
use crate::error::{AppError, Result};
use tracker_core::TimeRange;

pub fn resolve_range(params: &RangeParams) -> Result<TimeRange> {
    if let (Some(start), Some(end)) = (params.start.clone(), params.end.clone()) {
        let start = normalize_rfc3339_to_utc(&start)?;
        let end = normalize_rfc3339_to_utc(&end)?;
        return Ok(TimeRange { start, end });
    }
    if let Some(start) = params.start.clone() {
        let start = normalize_rfc3339_to_utc(&start)?;
        let end = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        return Ok(TimeRange { start, end });
    }
    let now_local = Local::now();
    let (start_local, end_local) = match params.range.as_deref().unwrap_or("last7days") {
        "today" => {
            let start = Local
                .with_ymd_and_hms(
                    now_local.year(),
                    now_local.month(),
                    now_local.day(),
                    0,
                    0,
                    0,
                )
                .single()
                .ok_or_else(|| AppError::InvalidInput("invalid local date".to_string()))?;
            (start, now_local)
        }
        "last7days" => {
            let start = now_local - Duration::days(7);
            (start, now_local)
        }
        "last14days" => {
            let start = now_local - Duration::days(14);
            (start, now_local)
        }
        "thismonth" => {
            let start = Local
                .with_ymd_and_hms(now_local.year(), now_local.month(), 1, 0, 0, 0)
                .single()
                .ok_or_else(|| AppError::InvalidInput("invalid local date".to_string()))?;
            (start, now_local)
        }
        "alltime" => {
            let start = Local
                .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| AppError::InvalidInput("invalid local date".to_string()))?;
            (start, now_local)
        }
        value => {
            return Err(AppError::InvalidInput(format!(
                "unsupported range {}",
                value
            )));
        }
    };
    let start = start_local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let end = end_local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    Ok(TimeRange { start, end })
}

pub fn normalize_rfc3339_to_utc(value: &str) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|err| AppError::InvalidInput(format!("invalid datetime: {}", err)))?;
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}
