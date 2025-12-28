# Release Process

This document describes the end-to-end release flow for the Codex Tracker desktop app
using macOS artifacts published on GitHub Releases.

## Prerequisites

- Versions updated in:
  - `apps/desktop/src-tauri/Cargo.toml`
  - `apps/desktop/src-tauri/tauri.conf.json`
  - `apps/web/package.json` (optional; keeps UI version aligned)
  - `CHANGELOG.md`

## How to cut a release (checklist)

1. Update versions:
   - `apps/desktop/src-tauri/Cargo.toml`
   - `apps/desktop/src-tauri/tauri.conf.json`
2. Update `CHANGELOG.md` with a new release section.
3. Commit the changes.
4. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
5. GitHub Actions `release.yml` builds and publishes artifacts.
6. Update the Homebrew cask with the new version + sha256 sums.

Optional local build helper:

The helper bumps versions + updates `CHANGELOG.md`, then runs a local build.

The helper assumes the Tauri CLI is installed:

```bash
cargo install tauri-cli --locked --version "^2.0.0"
```

```bash
bash scripts/release_local.sh X.Y.Z
```

## Gatekeeper note

Because releases are **not notarized**, macOS Gatekeeper will likely show a warning on first launch.
This is expected for unsigned distribution.

## Homebrew cask updates

The cask lives in the Homebrew tap repo at `ernest-gonzales/homebrew-tap` under `Casks/codex-tracker.rb`. For each release:

1. Update `version`.
2. Update `sha256` values for both `arm` and `intel` DMGs.
3. Commit the update in the Homebrew tap repository.

## Notes

- The app is desktop-only; Tauri bundles the UI from `apps/web/dist` with no runtime web server.
- The GitHub release workflow intentionally does not sign or notarize artifacts.
