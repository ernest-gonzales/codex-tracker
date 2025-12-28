# Release Process (macOS first)

This document describes the end-to-end release flow for the Codex Tracker desktop app,
including macOS signing + notarization, GitHub Releases, and Homebrew cask updates.

## Prerequisites

- Apple Developer account with a Developer ID Application certificate.
- App Store Connect API key (recommended) for notarization, or Apple ID + app-specific password.
- GitHub Actions secrets configured (see below).
- Versions updated in:
  - `apps/desktop/src-tauri/Cargo.toml`
  - `apps/desktop/src-tauri/tauri.conf.json`
  - `apps/web/package.json` (optional; keeps UI version aligned)
  - `CHANGELOG.md`

## Required GitHub Actions secrets

Signing:
- `APPLE_CERTIFICATE`: base64-encoded `.p12` Developer ID Application certificate.
- `APPLE_CERTIFICATE_PASSWORD`: password for the `.p12`.
- `APPLE_SIGNING_IDENTITY`: e.g. `Developer ID Application: Example, Inc. (TEAMID)`.
- `APPLE_TEAM_ID`: your Apple Developer Team ID.

Notarization (App Store Connect API key, preferred):
- `APPLE_API_KEY_ID`: Key ID from App Store Connect.
- `APPLE_API_ISSUER_ID`: Issuer ID from App Store Connect.
- `APPLE_API_PRIVATE_KEY`: base64-encoded `.p8` private key.

Alternative notarization (Apple ID):
- `APPLE_ID`: Apple ID email.
- `APPLE_APP_SPECIFIC_PASSWORD`: app-specific password for the Apple ID.

Base64 helpers (macOS):

```bash
base64 -i certificate.p12 | tr -d '\n'
base64 -i AuthKey_ABC123.p8 | tr -d '\n'
```

## How to cut a release (checklist)

1. Update versions:
   - `apps/desktop/src-tauri/Cargo.toml`
   - `apps/desktop/src-tauri/tauri.conf.json`
2. Update `CHANGELOG.md` with a new release section.
3. Commit the changes.
4. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
5. GitHub Actions `release.yml` builds, signs, notarizes, and publishes artifacts.
6. Update the Homebrew cask with the new version + sha256 sums.

Optional local build helper:

The helper assumes the Tauri CLI is installed:

```bash
cargo install tauri-cli --locked --version 2.5.1
```

```bash
bash scripts/release_local.sh
```

## Local notarization testing (optional)

After a local `cargo tauri build` on macOS:

1. Verify the signature:
   - `codesign --verify --deep --strict --verbose=2 path/to/Codex\\ Tracker.app`
2. Notarize with notarytool (API key example):
   - `xcrun notarytool submit path/to/Codex-Tracker.dmg --key AuthKey.p8 --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID" --wait`
3. Staple:
   - `xcrun stapler staple path/to/Codex\\ Tracker.app`
   - `xcrun stapler staple path/to/Codex-Tracker.dmg`
4. Assess Gatekeeper:
   - `spctl --assess --type execute --verbose=4 path/to/Codex\\ Tracker.app`

## Certificate rotation

1. Create a new Developer ID Application certificate in Apple Developer.
2. Export as `.p12`, base64-encode it, and update `APPLE_CERTIFICATE` + `APPLE_CERTIFICATE_PASSWORD`.
3. Verify the `APPLE_SIGNING_IDENTITY` and `APPLE_TEAM_ID` values.
4. Trigger a test release from a draft tag to validate notarization.

## Homebrew cask updates

The cask template lives at `docs/homebrew/Casks/codex-tracker.rb`. For each release:

1. Update `version`.
2. Update `sha256` values for both `arm` and `intel` DMGs.
3. Commit the update in the Homebrew tap repository.

## Notes

- The app is desktop-only; Tauri bundles the UI from `apps/web/dist` with no runtime web server.
- The GitHub release workflow notarizes and staples DMG + app bundle to minimize Gatekeeper prompts.
