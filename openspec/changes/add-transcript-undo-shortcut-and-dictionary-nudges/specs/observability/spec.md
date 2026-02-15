## ADDED Requirements
### Requirement: Undo Slot Lifecycle Logs
The system SHALL emit structured logs for tracked paste-slot lifecycle transitions used by undo.

#### Scenario: Slot create and overwrite
- **WHEN** a tracked paste slot is created or replaced
- **THEN** a structured log is emitted with event code `undo_slot_created` or `undo_slot_overwritten`
- **AND** log fields include `paste_id`, `source_action`, and `auto_refined`

#### Scenario: Slot consumed or expired
- **WHEN** a slot is consumed by undo dispatch or invalidated by TTL expiry
- **THEN** a structured log is emitted with event code `undo_slot_consumed` or `undo_slot_expired`
- **AND** log fields include `paste_id` and slot age metadata

### Requirement: Undo Dispatch Reason Logs
The system SHALL emit reason-coded structured logs for undo dispatch and no-op outcomes.

#### Scenario: No-op undo by reason
- **WHEN** undo is skipped
- **THEN** a structured log is emitted with event code `undo_dispatch_skipped`
- **AND** log field `reason` is one of `missing_slot`, `consumed_slot`, or `expired_slot`

#### Scenario: Undo dispatch attempted
- **WHEN** undo command dispatch is attempted
- **THEN** a structured log is emitted with event code `undo_dispatch_attempted`
- **AND** log fields include `paste_id`, `source_action`, and platform key-path metadata

### Requirement: Undo Operation-Cancel Logs
The system SHALL emit structured logs when undo triggers Escape-equivalent cancellation before undo dispatch.

#### Scenario: Operation cancel requested and completed
- **WHEN** undo is pressed during active recording/transcription/refine/post-process
- **THEN** structured logs are emitted with event codes `undo_operation_cancel_requested` and `undo_operation_cancel_completed`

#### Scenario: Stop-transition cancel requested
- **WHEN** undo is pressed during the recording stop-to-pipeline transition marker window
- **THEN** a structured log is emitted with event code `undo_stop_transition_cancel_requested`

#### Scenario: Operation cancel with no valid slot
- **WHEN** operation cancel completes and no valid tracked paste slot exists
- **THEN** a structured log is emitted with event code `undo_operation_cancel_no_slot`

### Requirement: Undo Heuristic Bridge Logs
The system SHALL emit structured logs for undo heuristic bridge queueing and result handling.

#### Scenario: Request queued and sent
- **WHEN** an undo evaluation request is queued or sent to frontend
- **THEN** a structured log is emitted with event code `undo_eval_enqueued` or `undo_eval_sent`
- **AND** log fields include `paste_id` and `queue_len`

#### Scenario: Queue overflow drop
- **WHEN** pending evaluation queue overflows and oldest request is dropped
- **THEN** a warning log is emitted with event code `undo_eval_dropped_overflow`
- **AND** log fields include dropped `paste_id` and resulting `queue_len`

### Requirement: Undo Nudge Decision Logs
The system SHALL emit structured logs for nudge trigger and suppression decisions.

#### Scenario: Nudge triggered
- **WHEN** alias or unresolved thresholds are met
- **THEN** a structured log is emitted with event code `undo_nudge_triggered`
- **AND** log fields include `identity_key` (when available), `alias_count`, and `unresolved_count`

#### Scenario: Nudge suppressed by gating
- **WHEN** evidence is updated but repeat-gating suppresses a duplicate nudge
- **THEN** a structured log is emitted with event code `undo_nudge_suppressed`
- **AND** log fields include current counts and last-shown snapshots

#### Scenario: Nudge suppressed by user identity suppression
- **WHEN** evidence updates for an identity that user suppressed with `Don't suggest this`
- **THEN** a structured log is emitted with event code `undo_nudge_identity_suppressed`
- **AND** log fields include `identity_key` and current count

### Requirement: Overlay Arbitration Logs
The system SHALL emit structured logs when overlay-event arbitration queues or replaces events.

#### Scenario: Overlay transient event queued
- **WHEN** transient undo feedback is queued because a nudge card is active
- **THEN** a structured log is emitted with event code `undo_overlay_event_queued`
- **AND** log fields include `event_type` and queue state

#### Scenario: Overlay nudge replacement
- **WHEN** a newer nudge replaces an existing visible nudge card
- **THEN** a structured log is emitted with event code `undo_overlay_event_replaced`
- **AND** log fields include previous and new nudge identities when available
