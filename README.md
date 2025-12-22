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
- CLI interface (nicely formatted) with brew install
- Desktop app (Tauri)
  - Goal: keep `apps/web` as the UI, but ship it as a native desktop app (no external browser).
  - Storage (fix current “DB next to executable” approach for installed apps)
    - Store SQLite + pricing defaults under the platform app data dir (not inside the app bundle).
    - First-run migration: if a legacy DB exists (next to the old binary / bundle) and the new location is empty, move/copy it (keep a backup and show the resolved paths in the UI).
    - Keep “power user” overrides (env vars and/or settings) for DB/pricing paths and Codex home, but default to safe, platform-correct locations.
  - Lifecycle
    - Single-instance behavior (second launch focuses existing window).
    - Initialization sequence: create dirs → migrate DB → run migrations → sync/apply pricing defaults → (optional) initial ingest → show UI.
    - Background work: manual “Sync now” + optional periodic ingest while the app is running; make shutdown cancel tasks cleanly.
    - Fix layout when window is not full-size (responsive breakpoints + scroll containment).
  - Backend integration (choose one)
    - Preferred: no local HTTP server in desktop mode; expose a small set of native commands and call `tracker_db`/`ingest` directly (keep `tracker_server` for “web mode”).
    - Alternative: spawn the existing Axum server as a child process bound to `127.0.0.1` on a random free port; have the desktop window wait for readiness and then load it (remove `open_browser()`).
  - Packaging / security
    - Strict allowlist of native entry points and input validation (paths, time ranges, model patterns).
    - Lock down navigation and CSP to only bundled assets; open external links in the system browser.
    - Never bind to non-loopback interfaces; if HTTP is used, randomize the port and consider CSRF/origin checks.
    - Replace the current “copy binary + dist” bundling (`./scripts/build-bundle.sh`) with platform installers via Tauri; optionally keep the script as a dev convenience.
    - Release hygiene: codesign/notarize on macOS, sign on Windows, publish checksums, document reproducible build steps.
