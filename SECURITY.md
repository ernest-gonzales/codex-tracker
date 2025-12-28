# Security Policy

Codex Tracker is a **local-only desktop application**. Even so, we treat security issues seriously—especially anything that could lead to unintended code execution, data exposure, or unsafe file system access when processing local logs and data.

## Supported Versions

Security fixes are provided only for the **latest released version**.

If you are running an older release, please upgrade and re-test before reporting.

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for suspected security vulnerabilities.

Instead, report privately via GitHub Security Advisories:

1. Go to this repository on GitHub.
2. Open the **Security** tab.
3. Choose **Advisories** → **New draft security advisory**.

If you cannot use GitHub advisories for some reason, open a minimal issue (no exploit details, no sensitive data) asking for a private channel to continue the report.

## What to Include

To help us triage quickly, include:

- A clear description of the impact and affected component(s)
- Steps to reproduce (proof-of-concept if available)
- Version(s) affected and your OS version
- Any relevant logs (redact tokens, secrets, and personal data)
- Proposed mitigation or patch, if you have one

## Disclosure Process

After receiving a report, we aim to:

- Acknowledge receipt within **7 days**
- Provide a status update within **14 days** (or sooner for critical issues)

We may request additional details to reproduce and validate the issue. Please coordinate with us before public disclosure so we can ship a fix and publish appropriate release notes.

## Scope

In scope:

- The desktop app (Tauri shell, IPC surface, UI, and Rust backend)
- Release artifacts published under GitHub Releases
- Data ingestion and parsing of local Codex CLI logs

Out of scope:

- Vulnerabilities in upstream dependencies without a practical impact on Codex Tracker
- Social engineering, phishing, or physical attacks
- Issues requiring a compromised host environment

