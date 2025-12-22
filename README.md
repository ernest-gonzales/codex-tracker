# Codex Tracker

Local-only Codex CLI usage tracker (tokens + cost) with a Rust backend and SQLite storage.

## Requirements

- Rust toolchain (stable)

## Quick start

```bash
cargo run -p tracker_server
```

The server binds to `127.0.0.1:3030` and stores data in
`~/.codex/codex-tracker.sqlite` by default.

## Frontend

```bash
cd apps/web
npm install
npm run build
```

Then run the server (`cargo run -p tracker_server`) and open `http://127.0.0.1:3030`.

## Build a runnable bundle

```bash
./scripts/build-bundle.sh
```

This creates `bundle/codex-tracker` plus `bundle/dist/` for the frontend assets.
Run the server with:

```bash
./bundle/codex-tracker
```

To override the frontend path, set `CODEX_TRACKER_DIST` to a custom directory.

## API notes

- Trigger ingestion: `POST /api/ingest/run`
- Summary: `GET /api/summary?range=last7days` or `GET /api/summary?start=<rfc3339>` (includes token and cost breakdowns)
- Timeseries: `GET /api/timeseries?range=last7days&bucket=day&metric=tokens`
- Model breakdown: `GET /api/breakdown?range=last7days`
- Token breakdown by model: `GET /api/breakdown/tokens?range=last7days`
- Cost breakdown by model + token type: `GET /api/breakdown/costs?range=last7days`
- Events: `GET /api/events?range=last7days&limit=200`
- All time range: `range=alltime`
- Settings: `GET/PUT /api/settings` (`codex_home`)
- Pricing rules: `GET/PUT /api/pricing` (includes cached input rate, prices are per 1M tokens)
- Recompute costs: `POST /api/pricing/recompute`

## Pricing assumptions

- `reasoning_output_tokens` are treated as a subset of `output_tokens` for cost calculation (no double billing).
- Pricing rules are expressed in USD per 1M tokens (input, cached input, output).
- If you update pricing rules or cost logic, run `POST /api/pricing/recompute` to refresh stored costs.

## Development

```bash
cargo test
```

TODO:

- proper release and binary signing
- auto fetch pricing on startup from https://github.com/BerriAI/litellm/blob/main/model_prices_and_context_window.json
- wrap app in tauri to create a proper desktop app, see if you could fix frontend layout when not on full window
