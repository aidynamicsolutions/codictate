# Change: Add Right-Sized Aptabase Analytics

## Why
Codictate needs product analytics to understand adoption and feature usage, but the initial proposal over-specified delivery guarantees and introduced unnecessary queueing complexity. This change adopts a pragmatic, privacy-first baseline aligned with plugin-native capabilities.

## What Changes
- Integrate `tauri-plugin-aptabase` with runtime-safe initialization (no panic paths).
- Use plugin-native best-effort delivery semantics and graceful-exit handling (`app_exited` + plugin-managed exit flush).
- Add analytics operational controls:
  - runtime kill switch (`HANDY_DISABLE_ANALYTICS`)
  - runtime/build-time app key precedence (`APTABASE_APP_KEY`)
  - CI-provisioned build-time embedding for distributed installers (`secrets.APTABASE_APP_KEY` -> build env `APTABASE_APP_KEY`)
  - persisted user setting (`share_usage_analytics`, default `true`)
- Establish a strict event ownership and schema model:
  - backend emits domain/lifecycle events (`app_started`, `app_exited`, `transcription_*`, `model_download_*`)
  - frontend emits UI events only through a typed backend command (`settings_opened`, `onboarding_completed`, `analytics_toggle_changed`)
  - event/property allowlist with low-cardinality constraints and blocked sensitive keys
- Add growth analytics and activation flow instrumentation:
  - backend `feature_used` success events across core features
  - one-time `aha_moment_reached` transition at 5 successful feature uses
  - local upgrade-prompt eligibility model (aha + onboarding complete + not-paid + 14-day cooldown)
  - prompt funnel events (`upgrade_prompt_shown`, `upgrade_prompt_action`, `upgrade_checkout_result`)
- Add a settings toggle for analytics opt-out and update i18n/locales.
- Update setup/privacy docs with app-verifiable claims and a manual verification checklist.

## Impact
- **Specs**: updates `observability`.
- **Code**:
  - `src-tauri/Cargo.toml`: add Aptabase dependency.
  - `src-tauri/src/lib.rs`: analytics config resolution, plugin init, startup/exit tracking.
  - `src-tauri/src/analytics.rs` (new): analytics policy, event allowlist validation, runtime gating.
  - `src-tauri/src/growth.rs` (new): growth state machine, aha milestone tracking, and prompt eligibility.
  - `src-tauri/src/commands/mod.rs`: typed UI analytics command + growth prompt commands.
  - `src-tauri/src/shortcut/mod.rs` + `src-tauri/src/settings.rs`: persisted analytics toggle setting and command.
  - `src/components/settings/*` + `src/stores/settingsStore.ts`: analytics settings UI and persistence wiring.
  - `src/components/growth/UpgradePromptBanner.tsx` + `src/App.tsx`: non-blocking prompt UI and eligibility handling.
  - `src/utils/analytics.ts` (new): thin frontend wrapper for UI analytics events.
  - `src-tauri/capabilities/default.json`: keep Aptabase plugin frontend command permission disabled so analytics must flow through backend policy.
  - `.github/workflows/build.yml`: inject `APTABASE_APP_KEY` from CI secret to embed key in distributed builds.
  - `doc/analytic_error_monitoring/*`: updated setup, event catalog, and growth ops validation docs.
  - `doc/prodRelease.md`: add installer-focused Aptabase verification and key provisioning checks.
