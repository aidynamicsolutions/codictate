# Sentry Dev Setup (Manual)

This guide is the manual setup path for Codictate's pre-prod Sentry integration.

## 1. Create Sentry Org + Project

1. Sign in to [Sentry](https://sentry.io/).
2. Create or select your organization.
3. Create a project for this app (example name: `codictate-desktop`).

Use one project for now (pre-prod/dev-first).

## 2. Capture DSN

1. Open Project Settings.
2. Go to Client Keys (DSN).
3. Copy the DSN value.

You will set this as `SENTRY_DSN` locally.

## 3. Create Auth Tokens (CI vs Local Triage)

Use two token types for least-privilege:

### A) CI token (sourcemap upload + release creation)

1. Open Organization Settings -> Developer Settings -> Organization Tokens.
2. Create org token (scope is fixed to `org:ci` in current Sentry UI).
3. Use this token for GitHub Actions secrets only.

### B) Local triage token (for issue/event checks + optional issue management)

Org tokens with `org:ci` are not enough for issue/event triage APIs.

Create a Personal Token (or Internal Integration token) with:

1. `event:read`
2. `project:releases`
3. `org:read`

If you also want to resolve/mute issues from CLI during cleanup, add issue write permission:

4. `event:admin` (or equivalent issue-management write scope in your Sentry plan/UI)

Use this token for local `sentry-cli` triage/debug workflows.

## 4. Configure Local Environment

Create/update local `.env` in project root:

```bash
SENTRY_DSN=https://<key>@o<org-id>.ingest.sentry.io/<project-id>
SENTRY_ENVIRONMENT=development
# Optional: set to 1 to force-disable Sentry
HANDY_DISABLE_SENTRY=0
# Optional but recommended for release alignment
SENTRY_RELEASE=codictate@0.7.6
# Optional convenience defaults for monitoring skill API modes
SENTRY_ORG=aidynamicsolution
SENTRY_PROJECT=codictate
```

Rules:

- missing/empty `SENTRY_DSN` => Sentry disabled
- `HANDY_DISABLE_SENTRY=1` => Sentry disabled even if DSN exists

## 4.1 Confirm Org and Project Slugs

Before running production triage modes, confirm exact slugs:

```bash
sentry-cli organizations list
sentry-cli projects list --org <org-slug>
```

For this repository, expected values are:

1. `SENTRY_ORG=aidynamicsolution`
2. `SENTRY_PROJECT=codictate`

## 5. Configure GitHub Secrets

Repository Settings -> Secrets and variables -> Actions:

- `SENTRY_AUTH_TOKEN`
- `SENTRY_ORG`
- `SENTRY_PROJECT`
- `SENTRY_DSN` (required for embedding DSN into shipped app builds)

CI already computes `SENTRY_RELEASE` from app version and sets environment.

Do not reuse your broader local triage token in CI unless required.

DSN precedence at runtime:

1. `SENTRY_DSN` runtime env (if present) overrides embedded value.
2. Embedded build-time DSN is used when runtime env is absent (normal installed app case).

## 6. Release + Environment Convention

Use these conventions consistently:

- Release: `codictate@<version>`
- Environment:
  - `development` for local debug/dev runs
  - `production` for release CI builds

Runtime events and sourcemap uploads must share the exact same release value.

## 6.1 Correlation Metadata Convention (Pre-Prod)

Codictate attaches two non-PII correlation layers:

1. `user.id = anon:<uuid>` (stable per local install, persisted in `observability_store.json`)
2. `run_id` (new per app launch, stored in event context/extra)

Important:

- `run_id` is intentionally **not** a Sentry tag in this phase (avoids high-cardinality indexed tags).
- anon IDs are pseudonymous and do not include direct personal identity.

## 7. Minimal Alerting Setup

In Sentry Alert Rules, add a minimal rule:

1. Trigger on new issue in this project.
2. Notify developer email or your dev channel.
3. Keep alert volume low in pre-prod (single channel is enough).

## 8. Common Troubleshooting

### No events in Sentry

- Check runtime `SENTRY_DSN` is present and valid (for local runs).
- For installed/release builds, ensure CI provided `SENTRY_DSN` during build so DSN is embedded.
- Verify `HANDY_DISABLE_SENTRY` is not `1`.
- Check app logs for explicit Sentry enabled/disabled reason.
- Note startup timing: events emitted before Tauri `.setup(...)` may not include anon/run correlation metadata.

### Sourcemaps not applied

- Confirm CI has `SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SENTRY_PROJECT`.
- Confirm uploaded release matches runtime event `release` exactly.
- Verify build generated sourcemaps when upload config is enabled.
- Confirm uploaded artifact URL prefix matches runtime frame URL scheme.
  - For Tauri webview in this app, use `app://localhost/assets`.
- If CI/plugin upload is inconclusive, run deterministic manual upload:

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

- Use event payload as the source of truth for symbolication success:
  - expected: readable source file/function/context lines in stack frames
  - not enough: only seeing issue created without readable mapped frames
- Note: release files list APIs may show empty results with artifact bundles. Prefer upload report + event frame inspection.

### Release mismatch

- Runtime fallback release is `codictate@<Cargo version>`.
- CI release is set from `tauri.conf.json` version.
- Keep `Cargo.toml`, `tauri.conf.json`, and `package.json` versions aligned.

### Correlation fields missing on a startup event

- Early startup events can occur before scope identity setup in `.setup(...)`.
- Trigger another test event after app is fully initialized and verify:
  - `user.id` starts with `anon:`
  - event details include `run_id`

### Invalid auth token

- For CI release upload, use org token (`org:ci`) or equivalent release-capable token.
- For local triage (`issues list`, `events list`), use a token with `event:read`.
- Replace repository secret and rerun workflow.

### Triage commands fail with 403 (`You do not have permission`)

- Your token likely lacks `event:read`.
- Organization tokens with fixed `org:ci` scope cannot query issue/event triage endpoints.
- Switch local CLI auth to a personal/internal token with `event:read`.

### Resolve/mute commands fail with 403

- Your token has read scopes only.
- Add `event:admin` (or equivalent issue-management write scope) for CLI `issues resolve` / `issues mute`.
- Keep CI token least-privilege; grant write scope only to local triage token if needed.

### `sentry_monitor.sh` can’t find `.env` or reports missing `SENTRY_DSN`

Run the skill from the target repository root, or pass `--repo-root` explicitly.

Examples:

```bash
cd /path/to/target-repo
bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode local-smoke --send-test-event
```

```bash
bash /path/to/sentry_monitor.sh \
  --mode local-smoke \
  --send-test-event \
  --repo-root /path/to/target-repo
```

### API checks fail with DNS/network errors in agent/sandbox

If the output says the result is inconclusive because DNS/network is unavailable, this is usually an execution-context issue, not a Sentry wiring issue.

Use host-network verification:

```bash
bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh \
  --mode verify-loop \
  --org aidynamicsolution \
  --project codictate \
  --repo-root /Users/tiger/Dev/opensource/speechGen/Handy \
  --require-host-network \
  --json
```

Interpretation:

1. `PASS`: no immediate action.
2. `PARTIAL`: unresolved backlog exists or some non-blocking checks are inconclusive.
3. `FAIL`: blocking auth/slug/config/network contract issue.

### Ongoing On-Demand Verification Loop

Run on demand whenever you want a full operational snapshot:

```bash
bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh \
  --mode verify-loop \
  --org aidynamicsolution \
  --project codictate \
  --repo-root /Users/tiger/Dev/opensource/speechGen/Handy \
  --json
```

For other repositories:

```bash
bash /path/to/sentry_monitor.sh \
  --mode verify-loop \
  --org <org-slug> \
  --project <project-slug> \
  --repo-root /path/to/repo \
  --json
```

## 9. If You Do Not Control the Upstream GitHub Repo

If your local `origin` points to an open-source repo you do not own, set up a fork and use dual remotes first.

Follow:

- `doc/analytic_error_monitoring/fork-upstream-secrets-checklist.md`
