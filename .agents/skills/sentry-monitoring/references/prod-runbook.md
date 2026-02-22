# Sentry Production Runbook (Developer-Oriented)

Use this runbook with `scripts/sentry_monitor.sh`.

## Org/Project Slug Discovery (Before First Run)

Use exact slugs from Sentry:

1. `sentry-cli organizations list`
2. `sentry-cli projects list --org <org-slug>`

Codictate defaults:

1. `org=aidynamicsolution`
2. `project=codictate`

## Token Setup Prerequisite

Before running production triage modes, ensure local `sentry-cli` auth has:

1. `event:read` (required for issue/event triage queries)
2. `project:releases` (release visibility checks)
3. `org:read`

Note:

1. Organization tokens with fixed `org:ci` scope are typically CI-focused.
2. Use a personal/internal token for local triage workflows when `org:ci` cannot access issue/event endpoints.

## Host-Network Requirement

Sentry API checks can be blocked in sandboxed environments due to DNS/network restrictions.

Rules:

1. For authoritative API verification, run with host-network access.
2. If a mode returns `PARTIAL` with network/DNS inconclusive message, rerun with:
   - `--require-host-network`
3. Treat sandbox DNS failures as environment-level constraints, not immediate proof of Sentry misconfiguration.
4. In Codex, request escalated host-network execution for API modes; if denied, treat result as inconclusive and rerun later.

## On-Demand Verification Loop (Recommended)

Run this whenever you want a full monitoring snapshot:

```bash
bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh \
  --mode verify-loop \
  --org aidynamicsolution \
  --project codictate \
  --repo-root /Users/tiger/Dev/opensource/speechGen/Handy \
  --json
```

`verify-loop` executes:

1. `setup-preflight`
2. `local-smoke`
3. `release-gate`
4. `prod-health`
5. `issue-triage`

It also stores checkpoint state and reports:

1. unresolved issue count
2. new unresolved since last run
3. resolved since last run
4. high-severity unresolved snapshot
5. release-regression snapshot

## Daily Health Check

1. Run:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode prod-health --org <org> --project <project>
   ```
2. Review:
   - unresolved issue volume
   - high-severity unresolved view
   - release-scoped unresolved view (default release)
3. Action:
   - if unresolved volume increased unexpectedly, run `issue-triage`
   - if high-severity unresolved exists, escalate same day

## Release Readiness Check

Before shipping:

1. Run:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode release-gate --org <org> --project <project>
   ```
2. Confirm:
   - version alignment across `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `package.json`
   - release naming consistency (`codictate@<version>`)
   - release visible in Sentry for target project
3. If failed:
   - fix version mismatch first
   - re-run release gate

## Issue Triage Routine

1. Run:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode issue-triage --org <org> --project <project>
   ```
2. Process in this order:
   - new since last check
   - likely urgent
   - recurring/noisy
3. For each issue:
   - capture reproduction hints
   - link to most likely release window
   - assign owner and target fix version

## Local Smoke After Changes

Use after Sentry-related code/config changes:

1. Run setup checks:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode setup-preflight
   ```
2. Run smoke:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode local-smoke --send-test-event
   ```
3. If network is unavailable, rerun with strict behavior only when needed:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode local-smoke --strict-network
   ```
4. To force host-network requirement:
   ```bash
   bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode local-smoke --require-host-network
   ```

## Incident Checklist

1. Confirm ingestion is healthy (`local-smoke` or direct `sentry-cli info`).
2. Confirm release/environment tags in affected events.
3. Confirm sourcemap symbolication for the impacted release.
4. Confirm privacy scrubbing still applies to sampled payloads.
5. Document:
   - first seen timestamp
   - affected release
   - mitigation and rollback status

## JS Symbolication Incident Recovery (Tauri/Webview)

Use this when production frontend issues show minified stacks.

Rules that must align:

1. event `release` == upload `release`
2. runtime frame URL prefix == upload URL prefix
   - Codictate uses `app://localhost/assets`
3. uploaded artifacts include current hashed bundles (`main-<hash>.js` + `.map`)

Deterministic fallback:

```bash
bunx vite build --sourcemap hidden
sentry-cli sourcemaps upload \
  --org <org> \
  --project <project> \
  --release codictate@<version> \
  --url-prefix "app://localhost/assets" \
  --validate \
  --wait \
  dist/assets
```

Validation:

1. Trigger one controlled frontend event on the same release.
2. Confirm stack frame resolves to readable source file/function/context lines.
3. If still unsymbolicated, re-check release mismatch first, then URL prefix mismatch.

Important note:

1. Artifact-bundle flows may not appear in older release-file listing endpoints.
2. Use upload report + event stack frame mapping as source of truth.

## Anti-Patterns to Avoid

1. Do not enable auto-resolve/mute from automation in this phase.
2. Do not expose DSN/token values in logs or check outputs.
3. Do not assume missing network means broken setup.
