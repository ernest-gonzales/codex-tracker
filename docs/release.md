# Release Process

This document describes the end-to-end release flow for the Codex Tracker CLI and
desktop apps using macOS artifacts published on GitHub Releases.

## Prerequisites

- Version source of truth: `Cargo.toml` (`[workspace.package].version`; all workspace crates inherit it via `version.workspace = true`).
- Optional UI version alignment: `apps/web/package.json`
- Release notes: `CHANGELOG.md`

## How to cut a release (checklist)

1. Update versions:
   - `Cargo.toml`
2. Update `CHANGELOG.md` with a new release section.
3. Commit the changes.
4. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
5. GitHub Actions `release.yml` builds and publishes artifacts.
6. Update Homebrew formula + cask with the new version + sha256 sums.

Optional local build helper:

The helper bumps versions + updates `CHANGELOG.md`, then runs a local build.

The helper assumes the Tauri CLI is installed:

```bash
cargo install tauri-cli --locked --version "^2.0.0"
```

```bash
bash scripts/release_local.sh X.Y.Z
```

## Gatekeeper note (desktop app)

Because releases are **not notarized**, macOS Gatekeeper will likely show a warning on first launch.
This is expected for unsigned distribution.

If you see “**Codex Tracker.app is damaged and can’t be opened**”, it’s usually Gatekeeper quarantine on an unsigned app:

1. Drag the app into `/Applications` (don’t run it from the mounted DMG).
2. Remove the quarantine attribute:

```bash
xattr -dr com.apple.quarantine "/Applications/Codex Tracker.app"
```

## Homebrew updates

The Homebrew tap repo is `ernest-gonzales/homebrew-tap`.

CLI formula: `Formula/codex-tracker.rb`

Desktop cask: `Casks/codex-tracker-desktop.rb`

1. Update `version`.
2. Update `sha256` values for both `arm` and `intel` artifacts:
   - `codex-tracker_<ver>_<arch>.tar.gz` (CLI)
   - `codex-tracker-desktop_<ver>_<arch>.dmg` (desktop)
3. Commit the update in the Homebrew tap repository.

## Notes

- The CLI embeds the UI and serves it from a local HTTP server.
- The desktop app bundles the UI from `apps/web/dist` with no runtime web server.
- The GitHub release workflow intentionally does not sign or notarize artifacts.
