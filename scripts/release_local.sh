#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "${ROOT_DIR}/apps/web"
npm ci
npm run build

cd "${ROOT_DIR}/apps/desktop/src-tauri"
cargo tauri build

echo "Local release build complete."
