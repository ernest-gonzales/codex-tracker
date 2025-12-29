# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added a CLI server mode (`codex-tracker`) that embeds the UI and opens a browser.
- Added shared `app_api` and `http_api` crates for the HTTP server entrypoint.

### Changed

- Split release artifacts into `codex-tracker` (CLI) and `codex-tracker-desktop` (Tauri app).

### Fixed

### Removed

## [0.2.0] - 2025-12-28

### Added

- Introduced a Tauri desktop app shell with local API handlers for analytics, ingest, settings, pricing, limits, and logs.
- Built new dashboard and settings UI modules with charts, panels, modals, and keyboard-driven interactions.
- Added macOS release workflows, release docs, and scripts for desktop build/run.
- Added desktop app icons and generated Tauri schemas.
- Expanded ingest and database test coverage.

### Changed

- Refactored backend into a new `crates/app` service layer and reorganized ingest parsing/pipelines.
- Optimized ingest performance with batching, parallel parsing, per-event cost computation, and SQLite tuning.
- Refined desktop UI density, spacing, and window sizing for small-screen layouts.

### Fixed

- Corrected ingest cursor offsets and timestamp normalization; skip non-JSONL files and parse lines once.
- Fixed Tauri runtime detection, startup ingest event emission, and a desktop bundle crash from hook ordering.
- Improved logs path handling and limits table responsiveness.

### Removed

- Removed the standalone server crate, export actions, and the ingest CLI binary.

## [0.1.0] - 2025-12-28

- Initial release.
