#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "${root_dir}/apps/web"
npm install
npm run build

cd "${root_dir}"
# run with CODEX_TRACKER_INGEST_TIMING=1 for ingestion time logs
cargo run -p codex_tracker
