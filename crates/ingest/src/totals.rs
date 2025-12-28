use std::io::BufRead;

use tracker_core::{ContextStatus, UsageTotals};

use crate::parser::{
    extract_context_from_line, extract_token_totals_from_line, extract_usage_totals_from_line,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use tracker_core::{ContextStatus, UsageTotals};

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
}
