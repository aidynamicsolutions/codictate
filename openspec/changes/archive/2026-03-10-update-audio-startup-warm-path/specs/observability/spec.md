## ADDED Requirements
### Requirement: Audio Startup Timeline Logs
The system SHALL emit structured logs that make on-demand audio startup latency diagnosable from the unified log file.

#### Scenario: Session timeline includes warm-path milestones
- **WHEN** an on-demand transcription start is attempted
- **THEN** logs for that startup attempt SHALL include trigger receipt, warm-path requested/skipped/completed state, topology resolution mode, stream-open readiness, capture-ready acknowledgement, connecting overlay readiness, and recording overlay readiness

#### Scenario: Resolution mode is explicit
- **WHEN** the system resolves an input device for on-demand start
- **THEN** it SHALL log whether the chosen resolution mode was `warm`, `cache`, or `fresh`
- **AND** SHALL include any fallback or invalidation reason when applicable

### Requirement: Machine-Readable Audio Startup Log Fields
The system SHALL emit audio startup diagnostics as structured key=value fields that remain directly filterable in the unified log file.

#### Scenario: Correlation and timing fields are structured
- **WHEN** the system emits audio startup diagnostics
- **THEN** correlation keys, timing milestones, event codes, and resolution mode SHALL be logged as separate structured fields
- **AND** SHALL NOT be encoded only inside free-form message text

#### Scenario: Unified log filtering remains straightforward
- **WHEN** a developer filters the unified log by `trigger_id`, `session`, or startup event code
- **THEN** the required startup diagnostics SHALL remain readable and filterable without parsing embedded JSON blobs or ad hoc string fragments

### Requirement: Trigger-To-Session Correlation Logs
The system SHALL emit a correlation key that bridges pre-session trigger logs to later session-bound startup logs.

#### Scenario: Trigger log occurs before session creation
- **WHEN** a user trigger is received before a recording session id exists
- **THEN** the system SHALL emit a structured trigger log containing a `trigger_id`

#### Scenario: Session logs retain trigger correlation
- **WHEN** a recording session id becomes available for the same startup attempt
- **THEN** subsequent startup logs SHALL include both `trigger_id` and `session`
- **AND** the `trigger_id` SHALL match the earlier pre-session trigger log for that startup attempt

### Requirement: Warm Path Cancellation and Auto-Close Logs
The system SHALL emit structured logs that explain why a warm path was reused, cancelled, or auto-closed.

#### Scenario: Warm path cancelled before commit
- **WHEN** a warm path is cancelled before recording commit
- **THEN** the system SHALL log a reason-coded event indicating why the warm path was cancelled
- **AND** SHALL include whether the stream had already been opened

#### Scenario: Warm stream auto-closed
- **WHEN** a warm stream is closed because no recording commit occurred within the grace window
- **THEN** the system SHALL log the auto-close event with ownership and timeout metadata

### Requirement: Audio Startup Verification Contract
The system SHALL support log-based verification of push-to-talk startup improvements using the unified log file alone.

#### Scenario: Developer compares two recent sessions
- **WHEN** a developer filters the unified log for two recent on-demand transcription sessions
- **THEN** the logs SHALL contain enough timing points and trigger-to-session correlation data to compare `fn_press -> connecting_overlay -> stream_open_ready -> recording_overlay`
- **AND** the developer SHALL not need ad hoc instrumentation to reconstruct that timeline
- **AND** SHALL NOT need post-processing of free-form string fields to extract the timing or correlation values
