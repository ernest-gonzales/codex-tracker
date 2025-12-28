## Safety guardrails (must-follow)

- Even when running Codex with full access (e.g. `codex -s danger-full-access -a never`), treat any path **outside this repository** as read-only by default.
- Never run **destructive** operations outside this repository (including `rm`, `mv`, `chmod/chown`, `truncate`, shell redirects like `> file`, `git reset --hard`, `git clean -fdx`, or any command that deletes/overwrites data).
- Allowed non-repo exception: you may update `~/.codex/plans/codex-tracker-implementation.md` as part of the project workflow (keep it narrowly scoped to this project; do not delete other files under `~/.codex`).
- If completing a task would require modifying/deleting something outside this repository (other than the plan file above), stop and ask for input.

## Agent execution rules (important)

- Operate in **autopilot mode**.
- Do NOT ask “what should I do next?”.
- Continue working until the **definition of done** is met.
- Always commit with a message describing what was done.
- Always add tests when needed.
- After any frontend task, run `cd apps/web && npm install && npm run build`.
- Before concluding any frontend task, run `cd apps/web && npm audit`.
- Always run npm commands with a 15-second timeout; if they fail, report it.
- Only stop to ask for input if you hit a **hard blocker**:
  - missing credentials or secrets
  - unclear requirement that materially changes behavior
  - destructive action outside this repository
- For server runs and local curl/API checks, you are pre-authorized to request escalation and proceed without an extra prompt.

### Definition of done

- Feature or task fully implemented
- Relevant tests/builds pass
- No obvious TODOs left behind
- Short summary of changes + how to verify

### Execution loop

1. Inspect repository and existing plan
2. Update or refine the plan if needed
3. Implement incrementally
4. Run relevant tests / build commands
5. Fix failures
6. Repeat until green
7. Summarize

### Assumptions

- If something is underspecified, choose the most reasonable default
- Document assumptions in the final summary

### Rust-specific rules

- Use `cargo fmt` and `cargo clippy` when relevant
- Treat warnings as errors; ensure `cargo clippy --workspace --all-targets -- -D warnings` is clean before committing
- If IDE diagnostics disagree, reproduce with `cargo check -p <crate> --tests` (or `cargo clippy -p <crate> --tests -- -D warnings`) and restart rust-analyzer before assuming the warning is real.
- After Rust changes, run `cargo check` for the affected crate(s) (use `-p codex_tracker_desktop` when touching the desktop app) to catch compile errors before committing.
- Prefer explicit error handling (`thiserror`, `anyhow`)
- SQLite migrations must be idempotent
- Avoid breaking public APIs without updating version notes
- Add or update tests for Rust changes

# Codex Tracker - Agent Notes

This repo is for a local-only Codex usage tracker (tokens + cost) with a Rust backend
and a fancy web frontend. Keep decisions consistent across sessions.

## Core requirements

- Local application only (bind to 127.0.0.1); no publishing/cloud sync.
- Backend must be in Rust (expected: Axum + SQLite).
- Frontend should be visually polished; prioritize charts and clear summaries.
- Ingestion source: Codex CLI logs under the Codex home directory (default `~/.codex`,
  configurable in-app). Auto-discover and parse all logs under that directory.
- Analytics focus (v1): model + time only; always show total usage and total cost summaries.

## Planning reference

- Implementation plan lives at `~/.codex/plans/codex-tracker-implementation.md`.
- Always use this plan as the baseline for work on this project and update it when requirements change. Never ask for permission to edit this file.

## Defaults and preferences

- Keep schemas and APIs stable and versionable.
- Prefer incremental ingestion with cursors and dedupe.
- Document setup and dev commands in a README when created.
- Crate names: `tracker_core`, `tracker_db`, `tracker_server`, `ingest`.
