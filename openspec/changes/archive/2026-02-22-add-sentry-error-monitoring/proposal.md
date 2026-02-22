# Change: Add Pre-Prod Sentry Error Monitoring

## Why
Codictate currently has no centralized crash/error visibility. Even in pre-production, we need early signal when backend panics or frontend exceptions happen, with enough context to debug quickly.

The solution should be simple and robust now, and extensible later when public users are onboarded.

## Key Decisions
- Release profile panic strategy for this phase is `panic = "unwind"` to improve panic-report capture reliability.
- Minidump sidecar/native crash process is deferred to a follow-up change.
- Kill switch name remains `HANDY_DISABLE_SENTRY` for compatibility and reduced churn in this phase.
- Scrubbing remains minimal and focused on `before_send` for this phase.
- Capture only selected high-impact handled backend failures (not blanket capture of all logged errors).
- Use pseudonymous install correlation (`user.id = anon:<uuid>`) and per-run correlation in non-indexed metadata (`context`/`extra`), not high-cardinality searchable tags.
- Preserve anonymous `user.id` values in scrubber logic when they start with `anon:`.
- Use runtime-over-build DSN precedence:
  - runtime `SENTRY_DSN` overrides
  - build-time embedded `SENTRY_DSN` fallback supports installed user builds without shell env setup.

## Non-Goals (This Phase)
- Native minidump capture (`sentry-rust-minidump`) and crash sidecar process integration.
- Breadcrumb-level scrubbing (`before_breadcrumb`) beyond default plugin behavior.
- Performance tracing, profiling, session replay, or user feedback widgets.
- `op_sid`/session-level Sentry tags for every captured event.

## What Changes
- Add `sentry` and `tauri-plugin-sentry` for backend + webview error reporting.
- Enable Sentry when a DSN is available from runtime env or build-time embedded fallback, and `HANDY_DISABLE_SENTRY` is not set.
- Use privacy-safe defaults: `send_default_pii = false` and client-side scrubbing via `before_send`.
- Record consistent `release` and `environment` metadata on every event.
- Add explicit handled-error capture helpers and instrument selected backend handled failures.
- Add pseudonymous correlation metadata:
  - stable install id stored locally and attached as `user.id = anon:<uuid>`
  - per-run `run_id` attached via event `context`/`extra` (non-indexed metadata)
- Configure Tauri v2 plugin permission in capability ACL (`src-tauri/capabilities/default.json`).
- Add frontend sourcemap upload support using `@sentry/vite-plugin`, gated by CI secrets.
- Pass `SENTRY_DSN` in CI release builds so distributed binaries can include build-time DSN fallback.
- Add developer-facing setup docs and a manual verification checklist.

## Impact
- **Specs**: updates `observability` spec.
- **Code**:
  - `src-tauri/Cargo.toml`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/sentry_observability.rs`
  - `src-tauri/src/actions.rs`
  - `src-tauri/src/managers/transcription.rs`
  - `src-tauri/capabilities/default.json`
  - `package.json`
  - `vite.config.ts`
  - `.github/workflows/build.yml`
- **Docs**:
  - `doc/analytic_error_monitoring/sentry_prerequisites.md`
  - `doc/analytic_error_monitoring/sentry_integration.md`
  - `doc/analytic_error_monitoring/sentry_dev_setup.md` (new)
  - `doc/prodRelease.md`
  - `doc/test/sentry-error-monitoring-manual-checklist.md` (new)
