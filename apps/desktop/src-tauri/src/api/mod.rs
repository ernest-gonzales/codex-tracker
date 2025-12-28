pub(crate) mod handlers;
mod types;

use tracker_app::RangeParams;
use tracker_core::TimeRange;

pub(crate) fn resolve_range(
    range: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<TimeRange, String> {
    tracker_app::resolve_range(&RangeParams { range, start, end }).map_err(to_error)
}

pub(crate) fn to_error(err: impl std::fmt::Display) -> String {
    err.to_string()
}
