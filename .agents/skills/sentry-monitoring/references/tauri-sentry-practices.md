# Tauri + Sentry Practices (Current Repo Baseline)

This reference describes the baseline expected by the `sentry-monitoring` skill.

## 1) Runtime Initialization

1. Initialize Sentry in Rust and keep the init guard alive for app lifetime.
2. Initialize `tauri-plugin-sentry` only when Sentry client is enabled.
3. Keep startup resilient when Sentry is disabled.

Repo markers:

1. `src-tauri/src/lib.rs` contains `initialize_sentry()`.
2. `src-tauri/src/lib.rs` gates plugin registration using `if let Some(client) = sentry_guard.as_ref()`.

## 2) Enable/Disable Contract

1. `SENTRY_DSN` present and non-empty enables Sentry.
2. `HANDY_DISABLE_SENTRY=1|true|yes|on` disables Sentry regardless of DSN.
3. Disable reason should be logged for diagnostics.

## 3) Release + Environment Tagging

1. Runtime events must include `release` and `environment`.
2. Fallback release format is `codictate@<version>`.
3. CI sourcemap upload release must match runtime release exactly.

Repo markers:

1. `src-tauri/src/lib.rs` release fallback uses Cargo package version.
2. `.github/workflows/build.yml` exports `SENTRY_RELEASE`.
3. `vite.config.ts` uses the same release naming shape for `@sentry/vite-plugin`.

## 4) Privacy Baseline

1. `send_default_pii = false`.
2. `before_send` scrub hook is present.
3. Obvious sensitive fields (emails, paths, IP-like tokens, secret-like keys) are redacted.

This phase intentionally avoids advanced privacy layers (for example `before_breadcrumb`) unless leakage is demonstrated.

## 5) Tauri v2 Capability Model

1. Capabilities live in `src-tauri/capabilities/*.json`.
2. `sentry:default` permission must be present in active capability config.

## 6) Frontend Symbolication

1. `@sentry/vite-plugin` handles sourcemap upload.
2. Upload is gated by secrets (`SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SENTRY_PROJECT`).
3. Local/fork builds should remain non-blocking when secrets are absent.

## 7) Production Scope Boundaries

1. Focus on error monitoring and release alignment.
2. Defer minidump sidecar, tracing tuning, replay tuning, and auto-remediation.
3. Keep workflows deterministic and diagnosable with explicit commands.
