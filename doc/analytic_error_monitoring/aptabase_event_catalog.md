# Aptabase Event Catalog (Current)

This document is the source-of-truth inventory for analytics events currently emitted by Codictate.

- Last verified from code: 2026-02-23
- Backend schema and guardrails: `src-tauri/src/analytics.rs`
- Frontend UI wrapper: `src/utils/analytics.ts`

## Runtime Gating (Applies to Every Event)

An event is sent only if all three checks pass:

1. Runtime analytics is enabled (Aptabase key resolved and runtime init succeeded).
2. Kill switch is not active (`HANDY_DISABLE_ANALYTICS` is not truthy).
3. User setting is enabled (`share_usage_analytics = true`).

Delivery is best-effort via Aptabase plugin queue/retry behavior.

## Event Ownership

- Backend owns domain/lifecycle events.
- Frontend owns only UI-intent events and sends them through backend command `track_ui_analytics_event`.

## Events

### Backend Events

#### `app_started`
- Owner: backend
- Emitted from: `src-tauri/src/lib.rs`
- Trigger: app setup completes and analytics startup path runs
- Properties: none

#### `app_exited`
- Owner: backend
- Emitted from: `src-tauri/src/lib.rs`
- Trigger: `RunEvent::Exit` (graceful app exit path)
- Properties: none

#### `transcription_completed`
- Owner: backend
- Emitted from: `src-tauri/src/actions.rs`
- Trigger: transcription pipeline returns `Ok((transcription, filler_words_removed))`
- Properties:
  - `result`: `empty` | `non_empty`
  - `source_action`: `transcribe` | `transcribe_with_post_process`

#### `transcription_failed`
- Owner: backend
- Emitted from: `src-tauri/src/actions.rs`
- Trigger: transcription pipeline returns `Err(...)`
- Properties:
  - `stage`: `transcribe`

#### `model_download_started`
- Owner: backend
- Emitted from: `src-tauri/src/commands/models.rs`
- Trigger: `download_model` command begins
- Properties: none

#### `model_download_completed`
- Owner: backend
- Emitted from: `src-tauri/src/commands/models.rs`
- Trigger: model download returns `Ok(())`
- Properties: none

#### `model_download_failed`
- Owner: backend
- Emitted from: `src-tauri/src/commands/models.rs`
- Trigger: model download returns `Err(...)`
- Properties: none

#### `feature_used`
- Owner: backend
- Emitted from: `src-tauri/src/growth.rs` via feature success instrumentation in `src-tauri/src/actions.rs`, `src-tauri/src/undo.rs`, and correction acceptance flows
- Trigger: a tracked feature completes with user-visible success
- Properties:
  - `feature`: `transcribe` | `transcribe_with_post_process` | `paste_last_transcript` | `undo_last_transcript` | `refine_last_transcript` | `correct_text`
  - `entrypoint`: `shortcut` | `external` | `ui`

#### `aha_moment_reached`
- Owner: backend
- Emitted from: `src-tauri/src/growth.rs`
- Trigger: first transition where cumulative successful feature uses reaches threshold (`5`)
- Properties:
  - `rule`: `v1_5_successes`
  - `scope`: `all_features`

### UI Events (Routed Through Backend)

#### `settings_opened`
- Owner: frontend (validated + sent by backend command)
- Emitted from: `src/App.tsx`
- Trigger: transition into `settings` section (`previousSection !== "settings"` and `currentSection === "settings"`)
- Properties:
  - `source`: `sidebar` | `menu`

#### `onboarding_completed`
- Owner: frontend (validated + sent by backend command)
- Emitted from: `src/components/onboarding/Onboarding.tsx`
- Trigger: onboarding completion handler (`handleReferralComplete`)
- Properties:
  - `source`: `onboarding_flow`

#### `analytics_toggle_changed`
- Owner: frontend (validated + sent by backend command)
- Emitted from: `src/components/settings/ShareUsageAnalytics.tsx`
- Trigger:
  - disabling analytics: event is sent before persisting `share_usage_analytics=false`
  - enabling analytics: setting is persisted first, then event is sent
- Properties:
  - `enabled`: `enabled` | `disabled`
  - `source`: `settings`

#### `upgrade_prompt_shown`
- Owner: frontend via backend command
- Emitted from: `src/App.tsx` (banner visibility path) and handled by `src-tauri/src/growth.rs`
- Trigger: upgrade banner is shown after an eligible aha nudge
- Properties:
  - `trigger`: `aha_moment`
  - `variant`: `v1`

#### `upgrade_prompt_action`
- Owner: frontend via backend command
- Emitted from: `src/App.tsx` (banner CTA/dismiss handlers) and handled by `src-tauri/src/growth.rs`
- Trigger: user interacts with upgrade banner
- Properties:
  - `action`: `cta_clicked` | `dismissed` | `closed`
  - `trigger`: `aha_moment`

#### `upgrade_checkout_result`
- Owner: frontend via backend command
- Emitted from: `src/App.tsx` (banner CTA flow) and handled by `src-tauri/src/growth.rs`
- Trigger: checkout flow state changes from the upgrade banner or settings
- Properties:
  - `result`: `started` | `completed` | `failed`
  - `source`: `aha_prompt` | `settings`

## Global Event/Property Policy

All events and properties are validated against allowlists in `src-tauri/src/analytics.rs`.

- Unknown event names are rejected.
- Unknown property keys are rejected.
- Sensitive keys are blocked (for example keys containing `token`, `password`, `secret`, `api_key`, `path`, `transcript`, `prompt`, `user_id`).
- Property values are sanitized:
  - strings are trimmed and limited to 64 characters
  - booleans are normalized to numeric values before validation
  - non-finite numbers are rejected
- Enum-valued keys must match allowed values exactly.

## Not Tracked by Design

The analytics layer must not send:

- transcript text
- prompt text
- file paths
- API keys or secrets
- custom user identifiers
