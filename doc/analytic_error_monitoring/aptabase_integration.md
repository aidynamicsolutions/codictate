# Aptabase Analytics Integration

This document describes the current Aptabase integration for Codictate.

## Related Docs
- Event catalog (current tracked events + schemas): `doc/analytic_error_monitoring/aptabase_event_catalog.md`
- Vendored patch maintenance guide: `doc/analytic_error_monitoring/aptabase_plugin_patch_maintenance.md`
- Growth signal ops note: `doc/analytic_error_monitoring/aptabase_growth_ops.md`

## Architecture
- **Transport:** `tauri-plugin-aptabase` sends events asynchronously.
- **Patch note:** current integration uses a temporary vendored Aptabase plugin runtime-safety patch. Maintainer guide: `doc/analytic_error_monitoring/aptabase_plugin_patch_maintenance.md`.
- **Delivery semantics:** best-effort queueing/retry from the plugin. We do not claim guaranteed no-loss delivery under crashes, force-kills, or disk/network faults.
- **Graceful shutdown:** on `RunEvent::Exit`, the app sends `app_exited`; Aptabase plugin handles exit flush internally.
- **Ownership model:**
  - backend emits domain/lifecycle events (`app_started`, `app_exited`, `transcription_*`, `model_download_*`)
  - frontend emits UI events only via backend command (`settings_opened`, `onboarding_completed`, `analytics_toggle_changed`)
- **Policy enforcement:** backend validates event names and properties against an allowlist and blocks sensitive keys.

## Runtime Controls
- **Key precedence:** runtime `APTABASE_APP_KEY` overrides build-time embedded key.
- **Distributed builds:** CI must provide `APTABASE_APP_KEY` during build to embed key for installed apps.
- **Kill switch:** `HANDY_DISABLE_ANALYTICS` disables analytics initialization.
- **User control:** `share_usage_analytics` setting (default `true`) disables event sending when toggled off.
- **Failure behavior:** missing/invalid config disables analytics gracefully; app startup must continue.

## Privacy Guardrails (App-Verifiable)
- Do not send transcript text, prompt text, API keys, file paths, or custom user identifiers.
- Keep event properties low-cardinality and allowlisted.
- Use Aptabase default anonymous identity behavior; do not attach app-defined persistent user IDs.

## Manual Validation Checklist
- [ ] Analytics enabled path: events appear in Aptabase dashboard.
- [ ] Settings opt-out path: no events after disabling `share_usage_analytics`.
- [ ] Kill switch path: no events with `HANDY_DISABLE_ANALYTICS=1`.
- [ ] Missing key path: analytics disabled gracefully, app continues running.
- [ ] Offline/reconnect: events created while offline are delivered later on best-effort basis.
- [ ] Graceful quit: `app_exited` is emitted and plugin exit flush executes.
