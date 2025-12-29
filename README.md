# Codex Tracker

**Codex Tracker** is a **local-only analytics app** for **Codex CLI usage**
(tokens and cost). It runs entirely on your machine, stores data in a local SQLite database,
and does **not** require any account, cloud service, or remote backend.

It ships as two macOS deliverables:

- **codex-tracker** (CLI)  
  Starts a local web server and opens the UI in your default browser.

- **codex-tracker-desktop** (desktop app)  
  Tauri desktop shell for the same UI and backend.

The application is built with:

- **Rust** (backend, ingestion, analytics)
- **React + TypeScript** (UI)
- **Tauri** (desktop shell)

---

## Key principles

- **Local-only by design**  
  Everything runs on your device. No telemetry, no cloud sync, no external services.

- **CLI-first on macOS**  
  The preferred install is the CLI mode to avoid Gatekeeper friction.

- **Developer-focused**  
  Optimized for long-running sessions, dense information, and predictable behavior.

---

## Features

- Token and cost totals for a selected time range (and all-time)
- Time-series charts for tokens and cost
- Breakdown by model and reasoning effort (when available)
- Active sessions with context window pressure
- Usage limits (5h / 7d) derived from Codex logs
- Multiple Codex homes (switch between different log directories)
- Editable pricing rules with automatic cost recomputation

---

## Screenshots

### Dashboard overview

![Codex Tracker dashboard](docs/screenshots/dashboard.png)

### Cost breakdown

![Codex Tracker cost breakdown](docs/screenshots/cost.png)

### Token and cost trends

![Codex Tracker token and cost trends](docs/screenshots/trends.png)

---

## Architecture overview

### Rust workspace

- `crates/core/`  
  Shared domain types, ranges, bucketing, and pricing math

- `crates/db/`  
  SQLite schema, migrations, and query layer

- `crates/ingest/`  
  Incremental discovery and ingestion of Codex CLI logs

- `crates/app/`  
  Application services: ingestion orchestration, analytics, defaults

- `crates/app_api/`  
  Shared API surface (requests/responses + handlers) used by both desktop and CLI

- `crates/http_api/`  
  Local HTTP server + embedded UI assets for CLI mode

### Desktop application

- `apps/web/`  
  React + TypeScript UI (Vite, Tailwind, Recharts)

- `apps/cli/`  
  CLI entrypoint that serves the UI over localhost and opens a browser

- `apps/desktop/src-tauri/`  
  Tauri shell and IPC commands bridging UI and Rust backend

The UI is built once:
- Desktop loads it directly in the Tauri shell
- CLI embeds it and serves it via a local HTTP server

---

## Installation (macOS)

### Option 1: Homebrew (CLI, recommended)

```bash
brew tap ernest-gonzales/homebrew-tap
brew install codex-tracker
```

Run it:

```bash
codex-tracker
```

Optional flags:

```bash
codex-tracker --port 4567
codex-tracker --no-open
```

Config file (default port is saved here):

```
~/Library/Application Support/codex-tracker/config.toml
```

Data directory:

- Reuses the desktop app data directory if present
- Otherwise defaults to `~/Library/Application Support/codex-tracker`

### Option 2: Homebrew (desktop app)

```bash
brew tap ernest-gonzales/homebrew-tap
brew install --cask codex-tracker-desktop
```

### Option 3: GitHub Releases (desktop app)

Download the latest DMG from GitHub Releases:

https://github.com/ernest-gonzales/codex-tracker/releases

Open the DMG and drag **Codex Tracker** to Applications.

### Gatekeeper note

Because desktop releases are **not notarized** (yet), macOS Gatekeeper will likely
show a warning on first launch. This is expected for unsigned distribution.

If you see “**Codex Tracker.app is damaged and can’t be opened**”:

1. Drag the app into `/Applications` (don’t run it from the mounted DMG).
2. Remove the quarantine attribute:

```bash
xattr -dr com.apple.quarantine "/Applications/Codex Tracker.app"
```

If you install via Homebrew, you can also use
`brew install --cask --no-quarantine codex-tracker-desktop`.
