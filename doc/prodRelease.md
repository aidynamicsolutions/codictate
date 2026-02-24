# Production Release Checklist

This document lists all items that must be completed before releasing to production on macOS.

---

## App Configuration

### HTTP Client Headers (`src-tauri/src/llm_client.rs`)

Update the placeholder URLs to your production values:

```rust
// Line ~41: Update REFERER to your actual domain
HeaderValue::from_static("https://codictate.app"),  // ← Change to real domain

// Line ~45: Update USER_AGENT with version info
HeaderValue::from_static("Codictate/1.0"),  // ← Update version as needed
```

### Tauri Configuration (`src-tauri/tauri.conf.json`)

1. **Bundle Identifier**: Update for production
   ```json
   "identifier": "com.yourcompany.codictate"
   ```

2. **Auto-Updater Endpoint**: Add your update server URL
   ```json
   "plugins": {
     "updater": {
       "endpoints": [
         "https://your-update-server.com/latest.json"
       ]
     }
   }
   ```

3. **macOS Code Signing**: Configure in `bundle.macOS`
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name (XXXXXXXXXX)",
     "hardenedRuntime": true,
     "entitlements": "Entitlements.plist"
   }
   ```

---

## App Store / Notarization

### macOS Notarization

Before distributing outside the App Store:

1. **Apple Developer Account**: Ensure you have a valid Developer ID certificate
2. **Notarization**: Run notarization after building:
   ```bash
   xcrun notarytool submit path/to/app.dmg --apple-id YOUR_APPLE_ID --password APP_SPECIFIC_PASSWORD --team-id TEAM_ID
   ```
3. **Stapling**: After notarization succeeds:
   ```bash
   xcrun stapler staple path/to/app.dmg
   ```

---

## Version Management

### Before Each Release

1. **Update version** in these files:
   - `src-tauri/tauri.conf.json` → `"version": "x.y.z"`
   - `src-tauri/Cargo.toml` → `version = "x.y.z"`
   - `package.json` → `"version": "x.y.z"`

2. **Generate changelog** for the release

---

## Build Commands

### Development Build
```bash
bun run tauri dev
```

### Production Build (macOS)
```bash
bun run tauri build
```

Output location: `src-tauri/target/release/bundle/`

---

## Pre-Release Verification

- [ ] All version numbers match across config files
- [ ] HTTP client headers use production URLs
- [ ] Update server endpoint is configured (if using auto-updates)
- [ ] Code signing identity is set correctly
- [ ] App icon and branding are correct
- [ ] Test on clean macOS installation
- [ ] Verify notarization succeeds
- [ ] Test auto-update flow (if applicable)

### Sentry Verification (Pre-Prod to Prod Readiness)

- [ ] `SENTRY_RELEASE` follows `codictate@<version>` and matches runtime release tags
- [ ] `SENTRY_ENVIRONMENT` is explicitly set per build profile
- [ ] CI secrets are configured: `SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SENTRY_PROJECT`, `SENTRY_DSN`
- [ ] Release build startup log confirms Sentry DSN source is valid (`runtime_env` or `build_time_embedded`)
- [ ] Frontend sourcemaps upload successfully in CI
- [ ] At least one backend and one frontend test event are visible in Sentry for the release candidate
- [ ] Kill switch (`HANDY_DISABLE_SENTRY=1`) confirmed to disable ingestion during smoke test
- [ ] Local triage token has read scopes for production debugging (`event:read`, `org:read`, `project:releases`)
- [ ] If CLI issue cleanup is required, local triage token also has issue write scope (`event:admin` or equivalent)

#### Sentry DSN Provisioning for Installed Builds

The app resolves DSN in this order:

1. Runtime `SENTRY_DSN` (override path for local debugging/reroute)
2. Build-time embedded DSN (`SENTRY_DSN` from CI build environment)

For production installers, users typically do not provide runtime env vars.
This means CI **must** provide `SENTRY_DSN` during build so installed apps can send errors.

#### Release Candidate Smoke Check (Backend Error Ingestion)

1. Build/install release candidate as normal.
2. Launch the app normally (no test-only env vars).
3. Trigger one real handled backend error path:
   - Option A (transcription): temporarily make active model file unavailable, run one transcription, then restore file.
   - Option B (correction): use invalid/missing provider credential and run one correction.
4. Verify startup logs show:
   - `Sentry enabled (...)`
   - `dsn_source='build_time_embedded'` (or `runtime_env` when explicitly overriding)
5. In Sentry, confirm a new backend issue/event appears for the release candidate with expected tags:
   - `handled=true`
   - `component=actions`
   - `operation=transcribe` or `operation=correction`
6. Restore any temporary failure setup and relaunch once for normal usage.

### Aptabase Verification (Pre-Prod to Prod Readiness)

- [ ] CI secret is configured: `APTABASE_APP_KEY`
- [ ] Release build startup log confirms analytics key source is valid (`runtime-env` or `build-time-embedded`)
- [ ] At least one analytics event is visible in Aptabase dashboard for the release candidate
- [ ] Kill switch (`HANDY_DISABLE_ANALYTICS=1`) confirmed to disable analytics ingestion during smoke test

#### Aptabase Key Provisioning for Installed Builds

The app resolves analytics key in this order:

1. Runtime `APTABASE_APP_KEY` (override path for local debugging/reroute)
2. Build-time embedded key (`APTABASE_APP_KEY` from CI build environment)

For production installers, users typically do not provide runtime env vars.
This means CI **must** provide `APTABASE_APP_KEY` during build so installed apps can send analytics.

#### Release Candidate Smoke Check (Frontend Symbolication)

1. Trigger one frontend exception on the release candidate.
2. Verify event has expected tags:
   - `release=codictate@<version>`
   - `environment=production` (or intended target env)
3. Verify stack frame is readable (not only minified `main-<hash>.js` frame).
4. If minified-only, run deterministic fallback upload:

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

5. Re-trigger one frontend event and confirm symbolication is now readable.

#### Live Production Error Debug Workflow (Fast Path)

1. Identify affected release and environment in Sentry issue details.
2. Verify same release has sourcemap upload evidence.
3. Confirm stack trace is symbolicated; if not, run fallback upload for that exact release.
4. Run monitoring triage:
   - `bash .agents/skills/sentry-monitoring/scripts/sentry_monitor.sh --mode issue-triage --org <org> --project <project> --require-host-network --json`
5. Prioritize by:
   - fatal/error level
   - first-seen recency
   - release regression signal

---

## Backup Compatibility

Before each release, verify that changes do not break existing user backups:

- [ ] Backup format version has been bumped if archive structure changed
- [ ] Migration pipeline handles previous backup format versions correctly
- [ ] Settings backup inclusion/exclusion lists updated for any new/removed `AppSettings` fields
- [ ] History payload schema changes have corresponding migration in the restore pipeline
- [ ] Dictionary payload schema changes have corresponding migration in the restore pipeline
- [ ] Test: restore a backup created with the previous release on the new build
- [ ] Test: round-trip backup (export + restore) produces identical data

See [backup-restore.md](backup-restore.md) for full feature documentation, ADR, and error catalog.

---

## Files Reference

| Purpose | File |
|---------|------|
| Version | `tauri.conf.json`, `Cargo.toml`, `package.json` |
| HTTP Headers | `src-tauri/src/llm_client.rs` |
| Bundle ID | `tauri.conf.json` → `identifier` |
| macOS Signing | `tauri.conf.json` → `bundle.macOS` |
| Auto-Updater | `tauri.conf.json` → `plugins.updater` |
| Backup Format | `doc/backup-restore.md` |
