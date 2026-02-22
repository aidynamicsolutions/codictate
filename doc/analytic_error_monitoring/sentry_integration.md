# Sentry Error Monitoring Integration

This document describes the current pre-prod Sentry integration in Codictate.

## Integration Shape

Sentry is initialized in Rust and connected to Tauri via `tauri-plugin-sentry`.

- Rust backend events are captured by the Sentry Rust SDK.
- Frontend (webview) events are forwarded through the Tauri Sentry plugin.
- Browser and backend events share release/environment metadata.

## Runtime Enablement Logic

Location: `src-tauri/src/lib.rs`

Sentry is enabled only when all conditions are true:

1. A DSN is available from either:
   - runtime env `SENTRY_DSN` (highest precedence), or
   - build-time embedded fallback DSN (compiled from build env `SENTRY_DSN`).
2. `HANDY_DISABLE_SENTRY` is not set to a truthy disable value (`1`, `true`, `yes`, `on`).

If disabled, app startup continues and the disable reason is written to unified logs.

## Actual Initialization Pattern

```rust
let (sentry_guard, sentry_status_message) = initialize_sentry();

let mut builder = tauri::Builder::default();

if let Some(client) = sentry_guard.as_ref() {
    builder = builder.plugin(tauri_plugin_sentry::init(client));
}
```

`initialize_sentry()` configures:

- `send_default_pii = false`
- `release` (from `SENTRY_RELEASE` or fallback `codictate@<Cargo version>`)
- `environment` (from `SENTRY_ENVIRONMENT` or fallback `development`)
- `before_send` event scrubbing for obvious email/path/IP/sensitive-key patterns
- DSN resolution precedence:
  1. runtime `SENTRY_DSN`
  2. build-time embedded `SENTRY_DSN` (for installed app builds)

During `.setup(...)`, Codictate also initializes Sentry scope correlation metadata:

- stable pseudonymous install identity: `user.id = anon:<uuid>`
- per-app-run `run_id` in event `context`/`extra` (not tags)

## Privacy Behavior

Current pre-prod privacy posture is intentionally minimal but explicit:

- no default PII collection
- `before_send` redaction only (no `before_breadcrumb` in this phase)
- request URL/cookies removed from outgoing payloads
- obvious secret-like key/value pairs redacted
- pseudonymous `user.id` values prefixed with `anon:` are preserved by scrubber logic

## Handled Errors Captured In This Phase

This phase captures selected high-value handled backend failures (not every logged error):

1. Transcription operation failure in action pipeline.
2. History save failure after transcription.
3. Paste path failures (`paste`, `run_on_main_thread`, main-thread result channel).
4. Background model-load failure in transcription manager.
5. Generic correction failure fallback path.

Handled events include:

- tag `handled=true`
- tag `component=<...>`
- tag `operation=<...>`
- fingerprint `["{{ default }}", component, operation]`

This balances debuggability with low instrumentation overhead.

## Latest Manual Validation Results (2026-02-22)

The latest manual validation run completed successfully for checklist sections `1`, `1.1`, `2`, `3`, `4`, `5`, `6`, `7`, `8`, and `9`.

Verified artifacts in Sentry:

1. Handled transcription failure:
   - issue: `CODICTATE-3` (`7283384317`)
   - tags: `handled=true`, `component=actions`, `operation=transcribe`
2. Handled correction fallback failure:
   - issue: `CODICTATE-4` (`7283386595`)
   - tags: `handled=true`, `component=actions`, `operation=correction`
3. Frontend unhandled exception:
   - issue: `CODICTATE-5` (`7283485666`)
   - title: `Error: intentional frontend unhandled exception (smoke test)`
   - same project/environment context confirmed in Sentry event details
4. Privacy scrub verification:
   - issue: `CODICTATE-6` (`7283509179`)
   - event: `cf81a7f6026e42009c757a940be64932`
   - redacted message observed: `privacy scrub smoke event: [redacted-email] /Users/[redacted-user]/private/file.txt [redacted-ip] api_key=[redacted]`
   - pseudonymous user ID preserved: `anon:192d4c7e-7f88-451e-8604-53df74387e7b`
5. Pseudonymous correlation verification:
   - compared events: `CODICTATE-5` and `CODICTATE-6`
   - `user.id` remained stable across relaunch (`anon:192d4c7e-7f88-451e-8604-53df74387e7b`)
   - `contexts.diagnostics.run_id` changed per run:
     - `a96944b3-7731-4d39-866c-b667017c8af1` -> `b2a4a382-2b2f-4f7f-9091-e93931562f92`
6. Kill-switch verification (`HANDY_DISABLE_SENTRY=1`):
   - startup log confirmed disable reason: `Sentry disabled: HANDY_DISABLE_SENTRY is set to '1'`
   - backend smoke hook was skipped with no active client
   - issue counters/timestamps remained unchanged before vs after disabled test run:
     - `CODICTATE-6` (`7283509179`): `count=1`, `last_seen=2026-02-22T08:11:50Z`
     - `CODICTATE-5` (`7283485666`): `count=1`, `last_seen=2026-02-22T07:51:57Z`
7. Release/environment verification:
   - dev event: `792e4ce71a3c46c1b838826ccbc7143b` with tags `release=codictate@0.7.7`, `environment=development`
   - release-binary run with runtime DSN removed (`env -u SENTRY_DSN`) logged:
     - `Sentry enabled (release='codictate@0.7.7', environment='development', dsn_source='build_time_embedded')`
   - release-binary ingested event: `eeb1bb10cce046f88f9d319e37b2bebd` with tags `release=codictate@0.7.7`, `environment=development`
8. Release panic-path verification:
   - controlled panic run using a temporary test harness (removed from mainline runtime) created issue `CODICTATE-7` (`7283645345`)
   - verified event: `29224d1189c94805a296c854522676e5`
   - tags included: `release=codictate@0.7.7`, `environment=development`, `operation=startup_panic_smoke`
   - runtime also emitted a secondary boundary panic (`panic in a function that cannot unwind`), and Sentry captured the fatal panic event with expected metadata
9. JS sourcemap symbolication verification:
   - built minified assets with sourcemaps (`bunx vite build --sourcemap hidden`)
   - uploaded sourcemaps for release `codictate@0.7.7` with `url-prefix=app://localhost/assets`
   - upload report bundle ID: `b2543041-cb11-5b44-a8ae-25253e3a9bfa`
   - triggered JS smoke event: `3096ebba6e8f47c1aca09badaf71b673` (issue `CODICTATE-9` / `7283774675`)
   - verified symbolicated readable frame in event payload:
     - `../../node_modules/react-select/dist/index-641ee5b8.esm.js`
     - function `loadingIndicatorCSS`
   - verified tags remained consistent: `release=codictate@0.7.7`, `environment=development`
10. Cleanup/reset status:
   - temporary test override code removed from runtime paths
   - local defaults reset (`HANDY_DISABLE_SENTRY=0`; no `HANDY_SENTRY_TEST_*` vars in `.env`)
   - smoke issues identified (`CODICTATE-1`..`CODICTATE-9`)
   - automated issue resolution was blocked by token scope (missing write permission)
   - documented manual close flow and required scope update for future automation

Fingerprint verification note:

- Sentry event payload exposes processed grouping hashes in `fingerprints` (for example `950e2ba97ac1b6d0950bacef6308a0e8`), not the raw template string.
- Presence of non-empty `fingerprints` together with expected `component/operation` tags confirms grouping behavior is applied for the handled test events.

## Pseudonymous Correlation Model

Correlation strategy in this phase:

1. Install-level correlation:
   - locally persisted random UUID in `observability_store.json`
   - attached as `user.id = anon:<uuid>`
2. Run-level correlation:
   - generated once per app launch as `run_id`
   - attached in `context`/`extra`

No personal identity fields are attached by app code.

## Why `run_id` Is In Context/Extra (Not Tag)

`run_id` is intentionally **not** stored as a searchable tag to avoid high-cardinality indexed tag growth.

- searchable tags are best for bounded value sets
- per-launch random IDs are unbounded
- context/extra keeps run correlation available in event details without cardinality pressure

## Startup Timing Gap

Sentry initialization (`sentry::init(...)`) occurs before Tauri `.setup(...)`.

- anon install identity and `run_id` are attached in `.setup(...)`
- very early startup events before `.setup(...)` may not include this correlation metadata

This is acceptable for current pre-prod scope and documented for troubleshooting.

## Release and Environment

Runtime and CI must agree on release naming:

- format: `codictate@<version>`

`environment` should be set explicitly:

- local dev: `development`
- CI release build: typically `production`

## Production DSN Provisioning

For installed user builds, the app may not have runtime shell env vars.

To ensure Sentry ingestion works after install:

1. Set `SENTRY_DSN` in CI build environment (for example, GitHub Actions secret).
2. Build the app so `SENTRY_DSN` is embedded at compile time.
3. Keep runtime `SENTRY_DSN` support as an override for local/dev or emergency reroute.

## Frontend Symbolication

Frontend sourcemaps are uploaded through `@sentry/vite-plugin` when these env vars are present:

- `SENTRY_AUTH_TOKEN`
- `SENTRY_ORG`
- `SENTRY_PROJECT`

If missing, builds stay non-blocking and skip upload.

### Production-Useful Symbolication Rules (Learned)

1. `release` must match exactly between:
   - runtime event tags
   - sourcemap upload target
2. Frame URL path must match uploaded artifact URL prefix.
   - Codictate/Tauri webview runtime frames use `app://localhost/assets/...`
   - upload with `--url-prefix "app://localhost/assets"` when manually uploading
3. Hash changes require fresh upload.
   - each new `main-<hash>.js` requires matching `.map` upload for that release
4. “Issue exists” is not enough.
   - success means stack frames become readable (source file/function/context), not just ingested.

### Deterministic Recovery Path When Symbolication Looks Broken

Use this when CI/plugin upload is uncertain or production traces are minified:

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

Then trigger one controlled JS event on the same release and verify event payload stack frames are symbolicated.

### Debugging Notes for Future Maintainers

- Artifact-bundle uploads can succeed even if legacy release-file listings look empty.
- Trust these checks first:
  1. upload report shows script+map pairs for actual built hashes
  2. event stack frame now maps to readable source file/function/context
- Keep a low-noise smoke tag for verification events and resolve those issues after validation.

## Sampling Defaults (Informational)

This phase does not enforce explicit `sample_rate` or `traces_sample_rate` settings.

- error monitoring uses SDK defaults
- performance tracing/replay is out of scope for this change

Sampling can be tuned in a later rollout change.
