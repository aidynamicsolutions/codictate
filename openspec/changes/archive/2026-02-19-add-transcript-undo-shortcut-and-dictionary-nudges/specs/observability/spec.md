## ADDED Requirements
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
