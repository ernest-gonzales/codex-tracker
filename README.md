# Codex Tracker

Local-only Codex CLI usage tracker (tokens + cost) with a Rust backend and a React dashboard,
bundled as a Tauri desktop app. Everything runs on-device and stores data in a local SQLite database.

## Features

- Token + cost totals for a selected time range (and all-time).
- Time series charts for tokens and cost.
- Breakdowns by model and by reasoning effort (when available).
- Active sessions + context window pressure.
- Usage limits (5h + 7d) with message counts derived from logs.
- Multiple Codex homes (switch between different `~/.codex` directories).
- Editable pricing rules with cost recomputation.

## Architecture

- Rust workspace:
  - `crates/app/` (`tracker_app`): shared app layer for DB init, pricing defaults, ingestion, and range parsing.
  - `crates/ingest/` (`ingest`): discovers Codex logs under the configured Codex home and ingests them incrementally.
  - `crates/db/` (`tracker_db`): SQLite schema/migrations + query layer.
  - `crates/core/` (`tracker_core`): shared types and helpers (ranges, bucketing, pricing math).
- Web UI:
  - `apps/web/`: React + TypeScript + Vite, Tailwind CSS, and Recharts.
  - Built assets live in `apps/web/dist` and are loaded by the desktop shell.
- Desktop app:
  - `apps/desktop/src-tauri`: Tauri shell + IPC commands that host the React UI and call the Rust backend.

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

Run the desktop app:

```bash
cargo run -p codex_tracker
```

Single command (build UI + run desktop app):

```bash
./scripts/run-desktop.sh
```

Note: the desktop app loads the built UI from `apps/web/dist` (no dev server).

### Desktop tips

- Cmd+R refreshes the dashboard.
- Cmd+L opens the active Codex home in Finder (same as the Logs button).
- Cmd+, opens Settings.
- Esc closes modals/drawers.
- Last selected range + settings tab persist between launches.

## Configuration and data locations

### Codex log source (“Codex home”)

The app ingests Codex CLI logs from the currently selected Codex home:

- Default: `$CODEX_HOME` if set, otherwise `~/.codex`.
- Change it in the UI (Homes / Settings) to point at another directory.

### Database and pricing defaults

The desktop app stores its runtime files in the OS app data directory (Tauri `AppData`),
surfaced in the Settings modal under Storage.

## Development

```bash
cargo test
```

## License

See `LICENSE`.
