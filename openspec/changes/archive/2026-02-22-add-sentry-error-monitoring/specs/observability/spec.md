## ADDED Requirements

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
