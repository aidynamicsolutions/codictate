# observability Specification

## Purpose
TBD - created by archiving change add-tracing-instrumentation. Update Purpose after archive.
## Requirements
### Requirement: Unified Log File

All application components SHALL write logs to a single file for easy debugging.

#### Scenario: Single file contains all logs
- **WHEN** a recording session runs
- **THEN** logs from Rust, Python sidecar, and Frontend appear in one log file
- **AND** the file is located in the OS log directory

### Requirement: Logfmt Format

Logs SHALL use Logfmt format for human-readable structured output.

#### Scenario: Log entry format
- **WHEN** an event is logged
- **THEN** the entry follows format: `TIMESTAMP LEVEL session=ID target=COMPONENT msg="MESSAGE"`
- **AND** the entry is on a single line
- **AND** the entry is readable without parsing tools

### Requirement: Session Correlation
All dictionary matching logs SHALL be session-correlated with the active transcription span.

#### Scenario: Filter dictionary decisions by session
- **WHEN** a developer filters logs by `session=<id>`
- **THEN** candidate checks, acceptance reasons, and summary counters for that session are available together

### Requirement: Log Retention

The system SHALL retain 7 days of log files.

#### Scenario: Log rotation
- **WHEN** logs rotate daily
- **THEN** the 7 most recent log files are kept
- **AND** older files are automatically deleted

### Requirement: Debugging Skill

The system SHALL provide an AI skill for debugging guidance.

#### Scenario: Debugging workflow
- **WHEN** a user asks how to debug a failed recording
- **THEN** the AI provides the log file location
- **AND** the AI shows how to filter by session_id

### Requirement: Dictionary Reason-Coded Decision Logs
The system SHALL emit reason-coded structured logs for dictionary matching accept/reject paths.

#### Scenario: Candidate rejected due to length ratio
- **WHEN** a dictionary candidate is rejected by length ratio guard
- **THEN** a debug log is emitted with reason `skip_length_ratio`
- **AND** log fields include `path`, `ngram`, `entry_input`, `entry_alias`, and `n`

#### Scenario: Candidate accepted by split fuzzy
- **WHEN** a split-token candidate is accepted
- **THEN** a structured log is emitted with reason `accept_split_fuzzy`
- **AND** includes `score`, `threshold`, and matching path

### Requirement: Dictionary Session Summary Metrics
The system SHALL emit per-session dictionary summary counters at info level.

#### Scenario: Session completes with dictionary enabled
- **WHEN** custom words are processed for a transcription
- **THEN** an info log reports `candidates_checked`, `exact_hits`, `split_fuzzy_hits`, and `standard_fuzzy_hits`
- **AND** includes reject counts grouped by reason code

### Requirement: Undo Slot Lifecycle Logs
The system SHALL emit structured logs for tracked paste-slot lifecycle transitions used by undo.

#### Scenario: Slot create and overwrite
- **WHEN** a tracked paste slot is created or replaced
- **THEN** structured log `undo_slot_created` or `undo_slot_overwritten` is emitted
- **AND** fields include `paste_id`, `source_action`, and `auto_refined`

#### Scenario: Slot consumed or expired
- **WHEN** a slot is consumed by undo dispatch or invalidated by TTL expiry
- **THEN** structured log `undo_slot_consumed` or `undo_slot_expired` is emitted
- **AND** fields include `paste_id` and slot age metadata

### Requirement: Undo Dispatch Reason Logs
The system SHALL emit reason-coded structured logs for undo dispatch and no-op outcomes.

#### Scenario: No-op undo by reason
- **WHEN** undo is skipped
- **THEN** structured log `undo_dispatch_skipped` is emitted
- **AND** `reason` is one of `missing_slot`, `consumed_slot`, or `expired_slot`

#### Scenario: Undo dispatch attempted
- **WHEN** undo command dispatch is attempted
- **THEN** structured log `undo_dispatch_attempted` is emitted
- **AND** fields include `paste_id`, `source_action`, and platform key-path metadata

### Requirement: Undo Operation-Cancel Logs
The system SHALL emit structured logs when undo triggers Escape-equivalent cancellation as an active-operation short-circuit.

#### Scenario: Operation cancel lifecycle
- **WHEN** undo is pressed during active recording/transcription/refine/post-process
- **THEN** logs `undo_operation_cancel_requested`, `undo_operation_cancel_completed`, and `undo_operation_cancel_short_circuit` are emitted

#### Scenario: Stop-transition cancel
- **WHEN** undo is pressed during stop-transition marker window
- **THEN** log `undo_stop_transition_cancel_requested` is emitted

### Requirement: Discoverability Hint Lifecycle Logs
The system SHALL emit structured logs for discoverability-hint scheduling, skip reasons, and emission.

#### Scenario: Scheduled
- **WHEN** hint evaluation is queued
- **THEN** log `undo_discoverability_hint_scheduled` is emitted

#### Scenario: Skipped
- **WHEN** hint is gated
- **THEN** log `undo_discoverability_hint_skipped` is emitted
- **AND** `reason` is one of `has_seen`, `has_used_undo`, or `insufficient_paste_count`

#### Scenario: Emitted
- **WHEN** hint is shown
- **THEN** log `undo_discoverability_hint_emitted` is emitted

### Requirement: Undo Stats Rollback Logs
The system SHALL emit structured logs for rollback request/defer/apply/skip lifecycle tied to undo dispatch.

#### Scenario: Deferred rollback
- **WHEN** rollback is requested before stats contribution metadata is available
- **THEN** logs `undo_stats_rollback_requested` and `undo_stats_rollback_deferred` are emitted

#### Scenario: Applied rollback
- **WHEN** contribution metadata is available for rollback token
- **THEN** log `undo_stats_rollback_applied` is emitted with contribution fields

#### Scenario: Skipped rollback
- **WHEN** rollback cannot be applied
- **THEN** log `undo_stats_rollback_skipped` is emitted with reason code

### Requirement: Restore Stats Import Source Logs
The system SHALL emit structured logs that identify whether restored home stats came from canonical payload or fallback recompute.

#### Scenario: Restore stats import summary emitted
- **WHEN** restore imports history data
- **THEN** log `restore_stats_import_summary` is emitted
- **AND** includes `stats_source`, `history_rows`, `zero_speech_duration_rows`, and final aggregate duration/word fields

#### Scenario: One-time manual stats repair outcome emitted
- **WHEN** one-time guarded manual stats repair is evaluated
- **THEN** log `restore_stats_manual_repair` is emitted with `outcome=applied|skipped`
- **AND** skipped logs include reason code (for example `guard_mismatch` or `history_db_missing`)

### Requirement: Centralized Error Monitoring
The system SHALL capture and report errors from both the Backend (Rust) and Frontend (TypeScript) to a centralized error tracking service (Sentry).

#### Scenario: Rust Panic Reporting
- **WHEN** the Rust backend panics
- **THEN** an error event is sent to Sentry
- **AND** the report includes the application version and OS details
- **AND** native minidump sidecar capture is not required in this pre-prod phase

#### Scenario: Release panic path compatibility
- **WHEN** a release build panic occurs
- **THEN** panic reporting uses the configured unwind-based path for this phase
- **AND** captured events include `release` and `environment` metadata when available

#### Scenario: Frontend Exception Reporting
- **WHEN** an unhandled JavaScript exception occurs
- **THEN** the error is captured and sent to Sentry
- **AND** the report includes the component stack trace

### Requirement: Best-Effort Delivery and Event Metadata
The system SHALL send Sentry events on a best-effort basis and include consistent release metadata for debugging across versions.

#### Scenario: Best-effort network delivery
- **WHEN** an error occurs while the device is offline or intermittently connected
- **THEN** the app continues functioning without blocking or crashing
- **AND** event delivery is best-effort (no guaranteed local durable queue in this phase)

#### Scenario: Release/environment tagging
- **WHEN** an event is sent to Sentry
- **THEN** the event includes `release` and `environment`
- **AND** release naming is consistent between runtime events and uploaded frontend sourcemaps

### Requirement: Privacy and PII Scrubbing
The system SHALL strictly scrub Personally Identifiable Information (PII) from error reports before they leave the user's device.

#### Scenario: PII Scrubbing
- **WHEN** an error report is generated
- **THEN** any potential PII (IP addresses, specific user paths, email patterns) is removed or masked
- **AND** this sanitization happens on the client-side (Rust `before_send` or Frontend equivalent)

#### Scenario: Default Anonymity
- **WHEN** an error is reported
- **THEN** default SDK behavior does not send default PII
- **AND** direct personal identifiers are not attached by application code
- **AND** a pseudonymous random install identifier MAY be attached for diagnostics as `user.id = anon:<uuid>`

#### Scenario: Preserve pseudonymous install ID during scrubbing
- **WHEN** event scrubbing runs in `before_send`
- **THEN** `user.id` values that start with `anon:` are preserved
- **AND** non-anon user identifiers continue to be scrubbed

### Requirement: Selected Handled Backend Error Capture
The system SHALL explicitly capture selected high-impact handled backend failures to improve debugging without over-instrumenting all logged errors.

#### Scenario: Capture handled transcription pipeline failures
- **WHEN** a transcription operation fails but is handled by application control flow
- **THEN** a Sentry event is captured with tags `handled=true`, `component`, and `operation`
- **AND** grouping uses fingerprint `["{{ default }}", component, operation]`

#### Scenario: Capture background model-load failure
- **WHEN** background model loading fails in the transcription manager
- **THEN** a handled Sentry event is captured from the manager layer
- **AND** this manager-level capture is limited to background/internal failures that do not bubble to a user-impact boundary

### Requirement: Pseudonymous Correlation Metadata
The system SHALL attach stable, non-PII correlation metadata to improve issue triage across app launches.

#### Scenario: Stable install-level pseudonymous correlation
- **WHEN** Sentry scope identity is initialized
- **THEN** the app loads or creates a stable random install id in local store
- **AND** attaches it as `user.id = anon:<uuid>`

#### Scenario: Per-run correlation in non-indexed metadata
- **WHEN** an event is captured
- **THEN** the event includes a per-launch `run_id` in context/extra metadata
- **AND** `run_id` is not emitted as a high-cardinality searchable tag in this phase

#### Scenario: Setup timing boundary documented
- **WHEN** startup occurs before `.setup(...)` identity initialization
- **THEN** early events MAY be missing anon install/run metadata
- **AND** this boundary is documented for developers

### Requirement: Operational Disable Controls
The system SHALL provide explicit controls to disable Sentry reporting without code changes.

#### Scenario: Kill switch disables reporting
- **WHEN** runtime environment variable `HANDY_DISABLE_SENTRY` is set to a truthy disable value
- **THEN** Sentry initialization is skipped
- **AND** the app logs the disable reason in unified logs

#### Scenario: Runtime DSN overrides embedded DSN
- **WHEN** both runtime `SENTRY_DSN` and build-time embedded DSN are available
- **THEN** runtime `SENTRY_DSN` is used for Sentry initialization
- **AND** this allows environment-level reroute without rebuilding the app

#### Scenario: Build-time DSN supports distributed installs
- **WHEN** runtime `SENTRY_DSN` is absent in an installed app
- **THEN** Sentry initialization uses build-time embedded DSN when present
- **AND** event reporting works without requiring end-user shell env setup

#### Scenario: Missing runtime and embedded DSN disables reporting
- **WHEN** runtime `SENTRY_DSN` is missing or empty and no build-time embedded DSN is available
- **THEN** Sentry initialization is skipped
- **AND** the app runs normally without telemetry failures

### Requirement: Developer Setup and Manual Validation Docs
The project SHALL include explicit developer documentation for setup and manual verification of Sentry integration.

#### Scenario: Setup guide available
- **WHEN** a developer configures Sentry for local or CI builds
- **THEN** docs enumerate required credentials, env vars, and release naming convention
- **AND** docs include minimal alert/notification setup guidance

#### Scenario: Manual checklist available
- **WHEN** a developer validates the integration
- **THEN** a checklist exists covering backend and frontend capture, privacy scrubbing, kill-switch behavior, release/environment tags, and sourcemap symbolication

### Requirement: Anonymous Analytics Event Collection
The system SHALL collect anonymous analytics events through Aptabase without attaching application-defined user identifiers.

#### Scenario: App-verifiable privacy boundaries
- **WHEN** an analytics event is emitted
- **THEN** the payload excludes transcript text, prompt text, API keys, file paths, and other direct identifiers
- **AND** event properties are limited to an allowlisted low-cardinality schema
- **AND** the application does not attach a custom persistent user ID

### Requirement: Best-Effort Delivery Semantics
The system SHALL use best-effort analytics delivery via Aptabase plugin queueing and retries, without claiming guaranteed no-loss delivery.

#### Scenario: Offline-to-online recovery
- **WHEN** analytics events are generated while connectivity is unavailable
- **THEN** events are queued by the Aptabase plugin and retried when delivery becomes possible
- **AND** this behavior is documented as best-effort rather than guaranteed

#### Scenario: Graceful exit flushing
- **WHEN** the app receives a graceful exit event
- **THEN** the app emits `app_exited`
- **AND** Aptabase plugin exit handling flushes queued events before process termination

### Requirement: Operational Analytics Controls
The system SHALL provide explicit non-crashing controls to enable or disable analytics behavior at runtime.

#### Scenario: Configuration precedence
- **WHEN** analytics initialization resolves the Aptabase app key
- **THEN** runtime `APTABASE_APP_KEY` takes precedence over build-time embedded key
- **AND** analytics is disabled gracefully when neither key source is available

#### Scenario: Distributed installer key provisioning
- **WHEN** a distributed/installed build runs without runtime `APTABASE_APP_KEY`
- **THEN** analytics uses build-time embedded `APTABASE_APP_KEY` when provided by CI at build time
- **AND** documentation states CI-provided embedding is required for installer analytics by default

#### Scenario: Kill switch disable
- **WHEN** `HANDY_DISABLE_ANALYTICS` is set to a truthy value
- **THEN** analytics initialization is skipped
- **AND** the app logs the disable reason without impacting normal operation

#### Scenario: User opt-out disable
- **WHEN** `share_usage_analytics` is disabled in app settings
- **THEN** subsequent analytics events are not sent
- **AND** app functionality remains unchanged

### Requirement: Analytics Event Ownership and Contract
The system SHALL define a single source of truth for event ownership and enforce allowlisted event/property contracts.

#### Scenario: Backend owns domain and lifecycle events
- **WHEN** domain or lifecycle milestones occur (`app_started`, `app_exited`, `transcription_*`, `model_download_*`)
- **THEN** events are emitted by backend Rust instrumentation only
- **AND** frontend does not emit duplicate copies of those events

#### Scenario: Frontend UI events route through typed backend command
- **WHEN** UI intent events occur (`settings_opened`, `onboarding_completed`, `analytics_toggle_changed`)
- **THEN** frontend submits them through a typed backend analytics command
- **AND** backend validates event/property allowlist before dispatching to Aptabase

### Requirement: Growth Signal and Prompt Eligibility
The system SHALL capture a high-signal activation milestone and a low-noise upgrade prompt funnel with deterministic local eligibility logic.

#### Scenario: Feature success tracking
- **WHEN** a core feature completes with user-visible success (`transcribe`, `transcribe_with_post_process`, `paste_last_transcript`, `undo_last_transcript`, `refine_last_transcript`, `correct_text`)
- **THEN** backend emits `feature_used` with allowlisted `feature` and `entrypoint` properties
- **AND** events are not emitted on key press/release churn without success

#### Scenario: Aha milestone transition
- **WHEN** cumulative successful feature count transitions from below 5 to 5 or more
- **THEN** backend emits `aha_moment_reached` exactly once per local profile
- **AND** growth state persists the milestone transition timestamp

#### Scenario: Upgrade prompt eligibility and cooldown
- **WHEN** eligibility is evaluated
- **THEN** prompt is eligible only if aha has been reached, onboarding is complete, user is not paid, and the last prompt timestamp is older than 14 days
- **AND** backend emits `upgrade-prompt-eligible` only when eligible

#### Scenario: Prompt funnel tracking
- **WHEN** the upgrade prompt is shown and the user interacts with it
- **THEN** frontend records `upgrade_prompt_shown`, `upgrade_prompt_action`, and `upgrade_checkout_result` through backend commands
- **AND** all prompt funnel properties remain low-cardinality and allowlisted
