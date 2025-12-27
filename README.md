# Codex Tracker

Local-only Codex CLI usage tracker (tokens + cost) with a Rust (Axum) backend and a React dashboard.
Everything runs on `127.0.0.1` and stores data in a local SQLite database.

<!--
Media (recommended):
- Logo: add `docs/assets/logo.png` (transparent, ~512x512) and then uncomment the block below.

<p align="center">
  <img src="docs/assets/logo.png" width="120" alt="Codex Tracker logo" />
</p>
-->

## Features

- Token + cost totals for a selected time range (and all-time).
- Time series charts for tokens and cost.
- Breakdowns by model and by reasoning effort (when available).
- Active sessions + context window pressure.
- Usage limits (5h + 7d) with message counts derived from logs.
- Multiple Codex homes (switch between different `~/.codex` directories).
- Editable pricing rules with cost recomputation.

## Screenshots

<!--
Add images under `docs/assets/` and then uncomment:

![Dashboard](docs/assets/screenshot-dashboard.png)
![Pricing editor](docs/assets/screenshot-pricing.png)
![Limits](docs/assets/screenshot-limits.png)
-->

## Architecture

- Rust workspace:
  - `crates/server/` (`tracker_server`): Axum server + JSON API + static file hosting for the web UI.
  - `crates/ingest/` (`ingest`): discovers Codex logs under the configured Codex home and ingests them incrementally.
  - `crates/db/` (`tracker_db`): SQLite schema/migrations + query layer.
  - `crates/core/` (`tracker_core`): shared types and helpers (ranges, bucketing, pricing math).
- Web UI:
  - `apps/web/`: React + TypeScript + Vite, Tailwind CSS, and Recharts.
  - Built assets live in `apps/web/dist` (or a custom dist directory).

## Quick start (from source)

Requirements:
- Rust stable (this repo uses Rust 2024 edition; use a recent stable toolchain).
- Node.js + npm (recommended: current LTS).

Build the frontend once:

```bash
cd apps/web
npm install
npm run build
```

Run the server:

```bash
cargo run -p tracker_server
```

It binds to `http://127.0.0.1:3030` and will try to open your browser automatically.

## Configuration and data locations

### Codex log source (“Codex home”)

The app ingests Codex CLI logs from the currently selected Codex home:
- Default: `$CODEX_HOME` if set, otherwise `~/.codex`.
- Change it in the UI (Homes / Settings) to point at another directory.

### Database and pricing defaults

The server stores its runtime files next to the server executable:
- SQLite DB: `codex-tracker.sqlite`
- Pricing defaults JSON: `codex-tracker-pricing.json`

Typical paths:
- `cargo run`: `target/debug/codex-tracker.sqlite`
- bundle: `bundle/codex-tracker.sqlite`

### Frontend asset directory

The server resolves the UI asset directory in this order:
1. `$CODEX_TRACKER_DIST` (absolute or relative path)
2. `./dist` next to the server executable (useful for bundles)
3. `apps/web/dist` (dev default)

## Build a runnable bundle

```bash
./scripts/build-bundle.sh
./bundle/codex-tracker
```

This produces a portable `bundle/` directory containing:
- `bundle/codex-tracker` (server binary)
- `bundle/dist/` (built frontend assets)

## API (optional)

The UI uses the JSON API directly; it’s also handy for scripting:

- Health: `GET /api/health`
- Ingest now: `POST /api/ingest/run`
- Summary: `GET /api/summary?range=last7days` (or `range=today|last14days|thismonth|alltime`)
- Pricing: `GET /api/pricing`, `PUT /api/pricing`, `POST /api/pricing/recompute`
- Homes: `GET /api/homes`, `POST /api/homes`, `PUT /api/homes/active`, `DELETE /api/homes/:id`
- Limits: `GET /api/limits`, `GET /api/limits/current`, `GET /api/limits/7d/windows`

Example:

```bash
curl -sS -X POST http://127.0.0.1:3030/api/ingest/run
```

## Development

```bash
cargo test
```

## License

See `LICENSE`.
