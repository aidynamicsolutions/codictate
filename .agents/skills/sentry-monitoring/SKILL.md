---
name: sentry-monitoring
description: Check and operate Sentry error monitoring for repositories using Sentry. Use when validating setup, troubleshooting missing events, running production health checks, verifying release/sourcemap readiness, or triaging unresolved Sentry issues. Trigger phrases include "check sentry", "error monitoring health", "release symbolication check", and "prod sentry triage".
---

# Sentry Monitoring

## Purpose

Use this skill to validate Sentry configuration and run ongoing operational checks without changing application code.

This skill is diagnostic-only. It should read state, run checks, and suggest fixes.

## Mode Selection

Choose a mode for `scripts/sentry_monitor.sh`:

| Mode | Use When |
| --- | --- |
| `setup-preflight` | Validate local repo wiring and env contracts quickly before testing events |
| `local-smoke` | Verify Sentry CLI connectivity and optionally send a synthetic smoke event |
| `prod-health` | Create a daily operational snapshot of unresolved issues and recent risk indicators |
| `release-gate` | Check release naming consistency and release visibility before/after shipping |
| `issue-triage` | Pull unresolved issue views and output a deterministic triage checklist |
| `verify-loop` | Run a full verification pass with checkpoint state and delta metrics for ongoing monitoring |
| `all` | Run all safe checks in sequence (production checks degrade gracefully if org/project missing) |

## Quick Start

Run from the target repo root:

```bash
bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode setup-preflight
```

If you are not in the target repo directory, pass `--repo-root` explicitly:

```bash
bash /path/to/sentry_monitor.sh --mode setup-preflight --repo-root /path/to/target-repo
```

On-demand full verification loop (recommended for this repo):

```bash
bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh \
  --mode verify-loop \
  --org aidynamicsolution \
  --project codictate \
  --repo-root /Users/tiger/Dev/opensource/speechGen/Handy \
  --json
```

Universal on-demand template (any repo):

```bash
bash /path/to/sentry_monitor.sh \
  --mode verify-loop \
  --org <org-slug> \
  --project <project-slug> \
  --repo-root /path/to/repo \
  --json
```

## Script Interface

Runtime dependencies:

1. `sentry-cli`
2. `rg` (preferred) or `grep`
3. `python3` (preferred) or `python` (fallback, used for JSON/state handling)

Core flags:

1. `--mode setup-preflight|local-smoke|prod-health|release-gate|issue-triage|verify-loop|all`
2. `--org <slug>`
3. `--project <slug>`
4. `--repo-root <path>`
5. `--json`

Advanced flags:

1. `--state-file <path>` for verify-loop checkpoint persistence.
2. `--require-host-network` to fail API checks when network/DNS is unavailable.
3. `--auto-slug-discovery` / `--no-auto-slug-discovery` to control missing-slug auto-selection.
4. `--strict-network` to treat network issues as hard failures.

## Token Prerequisites

For best results, separate CI token and local triage token:

1. CI (`org:ci`) token: good for sourcemap upload and release creation.
2. Local triage token: must include `event:read` for `prod-health` and `issue-triage` modes.

If triage commands fail with 403 permission errors, local token scope is insufficient.

## Execution Context

Checks are split into two tiers:

1. Local/repo checks (`setup-preflight`) can run in sandboxed contexts.
2. Sentry API checks (`local-smoke`, `prod-health`, `release-gate`, `issue-triage`, `verify-loop`) should run with host-network access.

When DNS/network is blocked in a sandbox, API modes return `PARTIAL` by default with an explicit host-network rerun command.
Use `--require-host-network` to force hard failure on network/DNS unavailability.

Codex execution pattern:

1. Run repo-local checks in sandbox first.
2. For API modes, request host-network execution permissions.
3. If permission is denied, return `PARTIAL` as inconclusive with exact rerun command.

## Repository Scope

This skill supports two check levels:

1. Generic repository checks:
   - `local-smoke`
   - `prod-health`
   - `issue-triage`
   - `release-gate` (with `--release` or `SENTRY_RELEASE` when release cannot be inferred)
2. Codictate/Tauri-specific checks:
   - deeper wiring validation in `setup-preflight`
   - deeper release-marker validation in `release-gate`

On non-Codictate repositories, repo-specific checks are skipped with a clear note instead of hard failure.

## Output Contract

Always report with this structure:

1. `Status: PASS | PARTIAL | FAIL`
2. `Evidence:`
3. `Risks:`
4. `Next commands:`

Interpretation baseline:

1. `PASS`: no immediate action required.
2. `PARTIAL`: inconclusive network context or unresolved backlog exists.
3. `FAIL`: auth/slug/config/runtime contract issue that blocks trustworthy verification.

## Guardrails

1. Never print full DSN or token values.
2. Never mutate app code/config from this skill.
3. Never auto-resolve/mute issues.
4. Treat network/API unavailability as inconclusive unless `--strict-network` or `--require-host-network` is set.

## Workflow

1. Run `setup-preflight` first.
2. Run `local-smoke` (optionally with `--send-test-event`).
3. If checking production readiness, run `release-gate`.
4. For production operations, run `prod-health` then `issue-triage`.
5. For complete monitoring with deltas, run `verify-loop`.
6. Summarize findings using the output contract and include exact follow-up commands.

## Slug Discovery and Recovery

`sentry_monitor.sh` validates org/project slugs before API modes.

1. If a slug is missing and only one candidate exists, it auto-selects and records that in evidence.
2. If a supplied slug is invalid, it fails and shows candidate slugs.
3. If multiple candidates exist for a missing slug, it fails fast and asks for explicit `--org` / `--project`.
4. You can disable missing-slug auto-selection with `--no-auto-slug-discovery`.

## References

Use these references when generating recommendations:

1. `references/tauri-sentry-practices.md` for implementation and privacy baselines.
2. `references/prod-runbook.md` for daily, release, and incident routines.
