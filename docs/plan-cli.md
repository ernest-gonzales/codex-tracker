# CLI Server Mode Plan

This plan adds a CLI-first mode that runs the app as a local web server and opens
the UI in the default browser, while keeping the existing Tauri desktop app.

Architecture must remain clean and modular, matching the current layered design:
- Domain logic stays in `crates/*` and `crates/app`.
- UI remains in `apps/web`.
- Desktop shell stays in `apps/desktop`.
- New CLI and HTTP layer are new, isolated modules with clear boundaries.

## Goals

- New CLI binary: `codex-tracker` (macOS first).
- Desktop app remains: `Codex Tracker.app` (brew cask name: `codex-tracker-desktop`).
- Default port is persisted (config file), but `--port` overrides per run only.
- Local-only HTTP API for the UI; no remote dependencies.
- Embedded UI assets in the CLI binary (single distributable).
- Release artifacts split between CLI and desktop.

## Constraints and conventions

- No shared logic duplication: reuse `tracker_app::AppState` and services.
- Keep module responsibilities narrow and testable.
- Prefer explicit boundaries between CLI / HTTP / UI / app services.
- No runtime web server for desktop; desktop continues to use Tauri IPC.
- macOS only for CLI release (initially).

## Proposed structure

New crate/module additions:
- `crates/http_api/`
  - HTTP router + JSON API handlers mirroring existing Tauri commands.
  - Static file handler serving embedded `apps/web/dist`.
  - Thin translation layer from HTTP requests to `tracker_app` services.
- `apps/cli/`
  - CLI entrypoint for server mode.
  - Loads config, resolves port, launches HTTP server, opens browser.

Existing modules to adjust:
- `apps/web/src/data/client.ts`:
  - Use Tauri invoke when available; otherwise use `fetch` to `/api/*`.
- `README.md`, `docs/release.md`, workflows/scripts:
  - Split release artifacts and installation instructions.

## API design

HTTP endpoints mirror Tauri command names to avoid branching logic in the UI:
- Example: `POST /api/summary`, `POST /api/timeseries`, `POST /api/pricing_replace`.
- Same request/response payload shapes as Tauri commands.

Security constraints for localhost-only:
- Bind to `127.0.0.1` (and optionally `::1`) only.
- No permissive CORS.
- For mutating endpoints, require an anti-CSRF token injected into the served HTML.
- Validate `Origin` header for browser requests.

## Embedded UI assets

- Build `apps/web/dist` before compiling the CLI.
- Embed the contents into `crates/http_api` (e.g., via `include_dir` or similar).
- Serve `/` and SPA routes with `index.html` fallback.
- Serve `/assets/*` with correct content types.

## CLI UX and config

- Config file: `~/Library/Application Support/codex-tracker/config.toml`.
- `port = 3845` by default.
- CLI options:
  - `--port <n>` overrides for current run only.
  - `--no-open` disables auto-open.
- Port conflict behavior:
  - If chosen port is busy, pick a random free port for this run only.
  - Print a warning and the actual URL.

## Release and packaging

- GitHub release artifacts:
  - `codex-tracker_<ver>_<arch>.tar.gz` (CLI).
  - `codex-tracker-desktop_<ver>_<arch>.dmg` + `.zip` (desktop).
- Homebrew:
  - `codex-tracker` formula for CLI.
  - `codex-tracker-desktop` cask for desktop app.
- Desktop app name remains `Codex Tracker.app`.

## Implementation steps

1. Add `crates/http_api` and its router + handlers.
2. Add `apps/cli` binary and config loading.
3. Embed `apps/web/dist` into the HTTP server crate.
4. Update web client transport (Tauri invoke vs HTTP fetch).
5. Update release workflow + local scripts.
6. Update README and release docs.
7. Add HTTP server smoke tests.

## Acceptance criteria

- `codex-tracker` starts a local server and opens a browser.
- Port is persisted; `--port` overrides only per run.
- UI works both in desktop (Tauri) and CLI (browser).
- Desktop artifacts remain available as `codex-tracker-desktop`.
- README + release docs match the new split and installation flow.
