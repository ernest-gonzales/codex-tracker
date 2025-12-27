#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "${root_dir}/apps/web"
npm install
npm run build

cd "${root_dir}"
cargo run -p codex_tracker_desktop
