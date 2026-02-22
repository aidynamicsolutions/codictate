# Sentry Error Monitoring Manual Checklist

## Goal

Validate pre-prod Sentry integration end-to-end for backend + frontend error capture with privacy scrubbing, kill-switch behavior, release/environment tagging, and JS symbolication.

## Pass Criteria

All items below must pass:

1. Backend and frontend events appear in Sentry.
2. Events contain correct `release` and `environment` tags.
3. Test PII samples are redacted in captured payloads.
4. `HANDY_DISABLE_SENTRY=1` prevents event delivery.
5. CI-built frontend stack traces are symbolicated.
6. Handled backend failure events include `handled`, `component`, and `operation`.
7. Pseudonymous correlation metadata is present and consistent (`user.id=anon:<uuid>`, `run_id`).

## 1. Local Setup Prechecks

- [x] `.env` contains valid `SENTRY_DSN`.
- [x] `SENTRY_ENVIRONMENT=development` is set.
- [x] `SENTRY_ORG` and `SENTRY_PROJECT` are set (or passed explicitly to the script).
- [x] `HANDY_DISABLE_SENTRY` is unset or `0`.
- [x] App starts normally (`bun run tauri:dev` or equivalent).
- [x] Startup logs indicate whether Sentry is enabled and why.
- [x] Startup logs include DSN source (`runtime_env` or `build_time_embedded`) when enabled.

## 1.1 Monitoring Skill On-Demand Loop

- [x] Run:
  - `bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode verify-loop --org aidynamicsolution --project codictate --repo-root /Users/tiger/Dev/opensource/speechGen/Handy --json`
- [x] If output says DNS/network is unavailable in current execution context, rerun with host-network requirement:
  - `bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode verify-loop --org aidynamicsolution --project codictate --repo-root /Users/tiger/Dev/opensource/speechGen/Handy --require-host-network --json`
- [x] Confirm output includes deterministic fields:
  - `Status`
  - `Evidence`
  - `Risks`
  - `Next commands`
- [x] Confirm verify-loop evidence includes:
  - unresolved count
  - new/resolved delta since last run
  - high-severity snapshot
  - release-regression snapshot

## 2. Backend Error Capture Test

- [x] Trigger a controlled backend error/panic path (test-only flow).
  - Recommended precheck: run local smoke from the monitoring skill:
    - `bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode local-smoke --send-test-event --repo-root /path/to/repo`
  - App-path validation: use section 3 or section 9 to generate a real backend event from the running app.
- [x] Confirm event appears in Sentry issue stream.
- [x] Confirm event has backend context (OS/app metadata).

## 3. Handled Backend Failure Capture Test

- [x] Trigger handled transcription failure path and confirm Sentry event is captured.
  - Suggested deterministic method:
    - Temporarily make the active transcription model unavailable (for example rename/move the local model file), then trigger one transcription action.
    - Restore the model file after the test.
    - Expect handled event tags:
      - `component=actions`
      - `operation=transcribe`
- [x] Trigger handled correction fallback failure path and confirm Sentry event is captured.
  - Suggested deterministic method:
    - Select a correction provider that requires credentials and temporarily use an invalid/missing API key.
    - Trigger one correction action.
    - Expect handled event tags:
      - `component=actions`
      - `operation=correction`
- [x] Verify tags include:
  - [x] `handled=true`
  - [x] `component=<expected>`
  - [x] `operation=<expected>`
- [x] Verify event fingerprint includes default grouping + `component` + `operation`.

## 4. Frontend Unhandled Exception Test

- [x] Trigger an unhandled frontend exception in webview.
- [x] Confirm event appears in Sentry.
- [x] Confirm event is associated with same project + environment.
  - Verified issue: `CODICTATE-5` (`7283485666`)

## 5. Privacy Scrub Verification

Use test payload strings containing obvious samples:

- email sample: `dev@example.com`
- path sample: `/Users/alice/private/file.txt`
- ip sample: `192.168.0.42`
- secret-like sample: `api_key=abc123`

Checks:

- [x] Email is redacted.
- [x] Local user path is redacted.
- [x] IP string is redacted.
- [x] Secret-like key/value is redacted.
- [x] `user.id=anon:<uuid>` remains intact (not scrubbed).
  - Verified issue/event: `CODICTATE-6` (`7283509179`) / `cf81a7f6026e42009c757a940be64932`
  - Verified redacted message: `privacy scrub smoke event: [redacted-email] /Users/[redacted-user]/private/file.txt [redacted-ip] api_key=[redacted]`
  - Verified preserved pseudonymous ID: `anon:192d4c7e-7f88-451e-8604-53df74387e7b`

## 6. Pseudonymous Correlation Verification

- [x] Capture an event and confirm `user.id` starts with `anon:`.
- [x] Quit and relaunch app, capture another event, confirm `user.id` remains the same.
- [x] Confirm `run_id` is visible in event details.
- [x] Relaunch app and capture another event, confirm `run_id` changes.
  - Comparison events:
    - `CODICTATE-5` (`7283485666`) user `anon:192d4c7e-7f88-451e-8604-53df74387e7b`, `run_id=a96944b3-7731-4d39-866c-b667017c8af1`
    - `CODICTATE-6` (`7283509179`) user `anon:192d4c7e-7f88-451e-8604-53df74387e7b`, `run_id=b2a4a382-2b2f-4f7f-9091-e93931562f92`

## 7. Kill-Switch Verification

- [x] Set `HANDY_DISABLE_SENTRY=1`.
- [x] Relaunch app.
- [x] Trigger backend/frontend test errors again.
- [x] Confirm no new Sentry events are ingested.
- [x] Confirm log shows Sentry disabled reason.
  - Run command:
    - `HANDY_DISABLE_SENTRY=1 HANDY_SENTRY_TEST_PRIVACY_REDACTION_ON_START=1 bun run tauri:dev`
    - frontend override hooks were temporary test harnesses and are not part of mainline runtime.
  - Verified startup logs:
    - `Sentry disabled: HANDY_DISABLE_SENTRY is set to '1'`
    - `Skipped privacy redaction smoke event because no active Sentry client is bound`
  - Ingestion verification (unchanged before/after run):
    - `CODICTATE-6` (`7283509179`) remained `count=1`, `last_seen=2026-02-22T08:11:50Z`
    - `CODICTATE-5` (`7283485666`) remained `count=1`, `last_seen=2026-02-22T07:51:57Z`

## 8. Release + Environment Verification

- [x] With Sentry enabled, trigger a test event.
- [x] Verify event tag `release` matches expected format `codictate@<version>`.
- [x] Verify event tag `environment` matches local value (`development`).
- [x] Verify installed/release build can send events with runtime `SENTRY_DSN` unset (embedded DSN fallback path).
  - Dev-mode smoke event:
    - event: `792e4ce71a3c46c1b838826ccbc7143b`
    - tags: `release=codictate@0.7.7`, `environment=development`
  - Release-binary fallback run command:
    - `env -u SENTRY_DSN RUST_LOG=codictate_app_lib=info HANDY_DISABLE_SENTRY=0 SENTRY_ENVIRONMENT=development HANDY_SENTRY_TEST_PRIVACY_REDACTION_ON_START=1 /Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/target/release/codictate`
  - Verified runtime log:
    - `Sentry enabled (release='codictate@0.7.7', environment='development', dsn_source='build_time_embedded')`
  - Verified release-binary ingested event:
    - event: `eeb1bb10cce046f88f9d319e37b2bebd`
    - tags: `release=codictate@0.7.7`, `environment=development`

## 9. Release Panic Path Verification (`panic = "unwind"`, Historical Validation)

- [x] Build/run release path used for panic validation.
- [x] Trigger controlled panic path.
- [x] Verify panic event is captured in Sentry.
- [x] Verify panic event includes `release` and `environment`.
  - Build command:
    - `SENTRY_DSN=\"$SENTRY_DSN\" cargo build --release --manifest-path /Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/Cargo.toml`
  - Panic trigger method used during validation:
    - temporary panic smoke harness in a test branch (removed from mainline runtime after verification)
  - Verified issue/event:
    - issue: `CODICTATE-7` (`7283645345`)
    - event: `29224d1189c94805a296c854522676e5`
  - Verified tags:
    - `release=codictate@0.7.7`
    - `environment=development`
    - `operation=startup_panic_smoke`
  - Note:
    - Runtime emitted the intentional panic and then a secondary fatal (`panic in a function that cannot unwind`) at the boundary; Sentry still captured the fatal panic event with expected metadata.

## 10. JS Sourcemap Symbolication (CI Artifact or Equivalent Release Upload)

- [x] Confirm sourcemap upload credentials are available (`SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SENTRY_PROJECT`).
  - For this run, local token + org/project were used because CI ownership is not available.
- [x] Build minified frontend assets with sourcemaps.
  - Command:
    - `bunx vite build --sourcemap hidden`
- [x] Upload sourcemaps for the active release with explicit app URL prefix.
  - Command:
    - `sentry-cli sourcemaps upload --org aidynamicsolution --project codictate --release codictate@0.7.7 --url-prefix "app://localhost/assets" --validate --wait dist/assets`
  - Upload report confirmed:
    - script + map pairs for `main`, `theme`, and `overlay`
    - artifact bundle ID: `b2543041-cb11-5b44-a8ae-25253e3a9bfa`
- [x] Trigger frontend JS smoke event using minified frame coordinates.
  - Event dispatched: `3096ebba-6e8f-47c1-aca0-9badaf71b673`
  - Issue: `CODICTATE-9` (`7283774675`)
- [x] Verify stack trace is symbolicated (non-minified readable frames).
  - Verified frame resolved from minified bundle path to readable source frame:
    - `../../node_modules/react-select/dist/index-641ee5b8.esm.js`
    - function `loadingIndicatorCSS`
    - context lines present in event payload
  - Verified tags:
    - `release=codictate@0.7.7`
    - `environment=development`

## 11. Alert Delivery Smoke Check

- [ ] Trigger one new issue.
- [ ] Confirm alert is received in configured developer email/channel.

## 12. Cleanup / Reset

- [x] Restore any temporary test setup (model files/API keys) used to force failures.
  - Repo-level verification completed:
    - no temporary frontend symbolication override code remains in `src/main.tsx`
    - no `HANDY_SENTRY_TEST_*` variables remain in local `.env`
    - baseline model asset exists at `src-tauri/resources/models/silero_vad_v4.onnx`
- [x] Reset `HANDY_DISABLE_SENTRY` to default (`0` or unset).
  - Verified local `.env` uses `HANDY_DISABLE_SENTRY=0`.
- [x] Keep `SENTRY_DSN` only in intended local/CI environments.
  - Verified `.env` is gitignored (`.gitignore` includes `.env`), so DSN is not committed.
  - CI should continue storing DSN in Actions Secrets only.
- [ ] Close or resolve test issues in Sentry.
  - Automation attempt from `sentry-cli` failed with `403` because active token lacks write scope.
  - Current local token scopes: `event:read`, `org:read`, `project:read`, `project:releases`.
  - To automate resolution, use a local token that also has issue write permission (`event:admin` or equivalent issue-management scope).
  - Manual fallback: in Sentry Issues, filter `project:codictate is:unresolved` and resolve `CODICTATE-1` through `CODICTATE-9`.
