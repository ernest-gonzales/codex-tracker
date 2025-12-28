# Codex Tracker

Local-only Codex CLI usage tracker (tokens + cost) with a Rust backend and a React dashboard,
bundled as a Tauri desktop app. Everything runs on-device and stores data in a local SQLite database.

## Privacy and scope

- Local-only by design: no account, no cloud sync, no backend services required.
- Reads Codex CLI logs from your configured Codex home (default `~/.codex`) and stores derived analytics in a local SQLite DB.
- The UI is bundled into the desktop app; there is no web deployment.

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
- UI bundle (desktop-only):
  - `apps/web/`: React + TypeScript + Vite, Tailwind CSS, and Recharts.
  - Built assets live in `apps/web/dist` and are loaded by the Tauri shell (no web deployment).
- Desktop app:
  - `apps/desktop/src-tauri`: Tauri shell + IPC commands that host the React UI and call the Rust backend.

## Install (macOS)

### GitHub Releases

Download the latest notarized DMG from GitHub Releases:

`https://github.com/ernest-gonzales/codex-tracker/releases`

### Homebrew (cask)

```bash
brew tap ernest-gonzales/homebrew-tap
brew install --cask codex-tracker
```

Note: update the tap name if you host the cask elsewhere.

### Gatekeeper note

The release artifacts are signed and notarized. If macOS still blocks the app,
use Finder to open it once (Control-click → Open) and the warning should not reappear.

## Quick start (from source)

Requirements:

- Rust stable 1.85+ (workspace uses a mix of Rust 2021 and Rust 2024 editions).
- Node.js + npm (recommended: current LTS).
  - macOS: Xcode Command Line Tools are required for Tauri builds.
  - Linux: Tauri depends on `webkit2gtk` and related system packages.
  - Windows: install the Visual Studio C++ build tools (MSVC).

Build the UI bundle once:

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

Note: the desktop app loads the built UI from `apps/web/dist` (no web server).

## Build from source (release builds)

### macOS

```bash
cargo install tauri-cli --locked --version "^2.0.0"

cd apps/web
npm ci
npm run build

cd ../desktop/src-tauri
cargo tauri build
```

### Windows (high level)

1. Install Rust with the MSVC toolchain and Visual Studio Build Tools.
2. Install Node.js LTS.
3. Install the Tauri CLI: `cargo install tauri-cli --locked --version "^2.0.0"`.
4. Run:
   - `npm ci && npm run build` in `apps/web`
   - `cargo tauri build` in `apps/desktop/src-tauri`

### Linux (high level)

1. Install Rust + Node.js LTS.
2. Install Tauri system deps (example for Debian/Ubuntu):
   - `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
3. Install the Tauri CLI: `cargo install tauri-cli --locked --version "^2.0.0"`.
4. Run:
   - `npm ci && npm run build` in `apps/web`
   - `cargo tauri build` in `apps/desktop/src-tauri`

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

UI unit tests:

```bash
cd apps/web
npm test
```

## Release process

See `docs/release.md` for the macOS release, signing/notarization, and Homebrew cask flow.

## Contributing

- Repo automation: `AGENTS.md` contains guidelines used by AI coding agents when working in this repository.
- PRs/issues: bug reports and feature requests are welcome via GitHub Issues.

## License

See `LICENSE`.
