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

