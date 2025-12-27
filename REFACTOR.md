# Backend Review Issues

## High
- Data loss on ingest read errors: `crates/ingest/src/lib.rs:816` breaks the read loop on any `read_line` error, but `crates/ingest/src/lib.rs:856` and `crates/ingest/src/lib.rs:866` still advance the cursor to `file_len`, so the next run skips unread data. Track the last successful byte offset and only advance the cursor when the file is fully processed, or avoid updating the cursor on read errors.

## Medium
- Timestamp normalization is missing: `crates/ingest/src/lib.rs:111` stores raw timestamp strings from logs, while queries in `crates/db/src/lib.rs:1230` rely on lexicographic string comparisons. Mixed formats or timezone offsets will mis-order or exclude rows. Parse and normalize to UTC (or store as integer epoch) on ingest.
- Effort normalization misrepresents data: `crates/db/src/lib.rs:1316` coerces `None`/empty/unknown effort to `"low"` and preserves original casing, which can collapse unknowns into low and split buckets by case. Preserve `None` for unknowns and canonicalize (e.g., lowercase) for known values.

## Low
- `ContextStatus::percent_left` can return negative percentages when `context_used > context_window` (`crates/core/src/lib.rs:20`). Clamp to `0.0` or return `None` when over the window to avoid misleading UI displays.
