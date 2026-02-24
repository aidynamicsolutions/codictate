## ADDED Requirements

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
