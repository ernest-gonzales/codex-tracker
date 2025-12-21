#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT/apps/web"
npm install
npm run build

cd "$ROOT"
cargo build --release -p tracker_server

BUNDLE_DIR="$ROOT/bundle"
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR"

cp "$ROOT/target/release/tracker_server" "$BUNDLE_DIR/codex-tracker"
cp -R "$ROOT/apps/web/dist" "$BUNDLE_DIR/dist"

echo "Bundle created at $BUNDLE_DIR"
