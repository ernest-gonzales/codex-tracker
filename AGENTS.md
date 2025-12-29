# AGENTS.md

## Purpose

This file defines **project constraints and contribution guidelines** for both
**AI-assisted** and **human** contributors working on this repository.

It exists to ensure:

- architectural consistency
- safe automation
- predictable outcomes when using coding agents (e.g. Codex)

Human contributors may skim or ignore this file unless they are using AI tools.

---

## Safety & repository boundaries (mandatory)

- Treat **all paths outside this repository as read-only**.
- Never run destructive commands that modify or delete data outside the repository.
  This includes (but is not limited to):
  - `rm`, `mv`, `truncate`, shell redirects like `> file`
  - `git reset --hard`, `git clean -fdx`
  - changing permissions or ownership (`chmod`, `chown`)
- If a task would require modifying files outside the repository, **stop and request clarification**.

---

## Contribution execution guidelines

These rules apply equally to human and AI-assisted workflows.

- Work end-to-end on a task until completion.
- Avoid unnecessary clarification questions unless a requirement is genuinely ambiguous.
- Prefer incremental, reviewable changes.
- Make all changes on a custom branch unless already working on a non-main branch.
- Commit changes with clear, descriptive messages.
- Add or update tests when behavior changes.
- Do not leave unfinished TODOs in production code.

---

## Definition of done

A task is considered complete when:

- The feature or change is fully implemented
- Relevant tests pass
- Builds succeed (frontend and/or backend as applicable)
- No obvious follow-up work is required
- A short summary explains:
  - what changed
  - how to verify it

---

## Execution workflow (recommended)

1. Inspect the repository and understand existing structure
2. Identify the smallest safe set of changes
3. Implement incrementally
4. Run relevant builds/tests
5. Fix failures
6. Repeat until green
7. Summarize changes

---

## Assumptions & defaults

- If a requirement is underspecified, choose the most reasonable default
- Document assumptions in commit messages or summaries
- Prefer clarity and maintainability over cleverness

---

## Rust-specific guidelines

- Run `cargo fmt` and keep formatting clean
- After any Rust changes, ensure `cargo fmt --check` passes (run `cargo fmt` to fix)
- After any Rust changes, ensure `cargo clippy --workspace --all-targets -- -D warnings` passes
- Treat warnings as errors:

  ```bash
  cargo clippy --workspace --all-targets -- -D warnings
  ```
