## ADDED Requirements
### Requirement: Cache Refresh Lifecycle Logs
The system SHALL emit structured logs for every background topology refresh that can affect later startup behavior.

#### Scenario: Refresh source and outcome are logged
- **WHEN** the app starts a topology refresh because of app foreground, wake, audio-route change, or another supported cache-refresh source
- **THEN** the log entry SHALL include the refresh source as a structured field
- **AND** SHALL include a structured outcome such as `started`, `completed`, `skipped`, `failed`, or `coalesced`

#### Scenario: Unsupported wake hook skip is logged
- **WHEN** a wake lifecycle refresh hook is unsupported on the current platform or runtime path
- **THEN** the app SHALL emit a structured log with the attempted refresh source
- **AND** SHALL record `outcome="skipped"` and a structured skip reason

#### Scenario: Refresh completion includes cache metadata
- **WHEN** a topology refresh completes successfully
- **THEN** the log entry SHALL include refresh duration, device count, and captured route generation as structured fields
- **AND** SHALL indicate that cached topology was updated without encoding those details only inside free-form message text

#### Scenario: Coalesced refresh requests remain diagnosable
- **WHEN** multiple refresh triggers collapse into a single background refresh pass
- **THEN** the logs SHALL make it clear that later requests were coalesced or skipped
- **AND** SHALL remain filterable by refresh source and event code

#### Scenario: Startup during in-flight refresh is diagnosable
- **WHEN** a user-triggered start occurs while a background topology refresh is still running
- **THEN** the startup and refresh logs SHALL make it possible to tell whether startup reused the completed refresh results or proceeded without waiting for them

### Requirement: Fresh Startup Reason Logs
The system SHALL emit a structured reason code when startup falls back to fresh topology on the startup path.

#### Scenario: Fresh fallback reason is structured
- **WHEN** on-demand startup resolves topology with `resolution_mode="fresh"`
- **THEN** the startup logs SHALL include a structured `fresh_reason` field
- **AND** `fresh_reason` SHALL distinguish at least `cache_expired`, `route_changed`, `route_unknown`, and `cached_open_failed`
- **AND** developers SHALL NOT need to infer the fresh-resolution cause only from surrounding log lines

### Requirement: Post-Idle Audio Startup Verification
The system SHALL support log-based verification of long-idle and lifecycle-refresh startup behavior using the unified log file alone.

#### Scenario: Developer verifies post-idle fast path after background refresh
- **WHEN** a developer performs a long-idle or wake, app-foreground, or idle route-change verification pass
- **THEN** the logs SHALL show whether a background topology refresh happened before the next startup
- **AND** SHALL make it possible to tell whether the following start reused refreshed cached topology or fell back to fresh startup-path enumeration

#### Scenario: Developer verifies manual lifecycle exercise passes
- **WHEN** a developer manually exercises app foreground, macOS sleep and wake, or idle audio-route change before the next push-to-talk run
- **THEN** the logs SHALL contain the lifecycle trigger source, refresh outcome, and the subsequent startup resolution for each exercised pass
- **AND** those logs SHALL be sufficient for an independent reviewer to confirm whether the implementation matched the expected spec behavior
