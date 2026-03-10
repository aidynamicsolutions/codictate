## ADDED Requirements
### Requirement: On-Demand Startup Phase Logs
The system SHALL emit structured logs that separate perceived-latency milestones from actual audio-readiness milestones on the on-demand startup path.

#### Scenario: Earliest input boundary is measurable
- **WHEN** an on-demand recording is initiated through the Fn push-to-talk path or another supported trigger
- **THEN** the logs SHALL include the earliest practical input-handling milestone before `startup_trigger_received`
- **AND** developers SHALL be able to measure any gap before `startup_trigger_received` from logs alone

#### Scenario: Trigger correlation survives into later startup logs
- **WHEN** early-input and accepted-trigger milestones are logged before a session id exists
- **THEN** those logs SHALL still carry a trigger-correlation key that can be matched to later session-bound startup logs

#### Scenario: Perceived and actual milestones remain distinct
- **WHEN** an on-demand startup attempt progresses through early feedback, stream open, capture-ready acknowledgement, and recording bars
- **THEN** the logs SHALL make it possible to distinguish:
  - accepted trigger to first visible feedback
  - accepted trigger to stream-open ready
  - accepted trigger to capture-ready acknowledgement
  - capture-ready acknowledgement to recording bars shown

#### Scenario: Cache-hit startup still exposes later bottlenecks
- **WHEN** an on-demand start resolves topology through cache reuse
- **THEN** the logs SHALL still expose later startup phases clearly enough to show whether the remaining bottleneck was stream open, recorder start acknowledgement, or overlay presentation

### Requirement: Stream-Open and Start-Acknowledgement Subphase Logs
The system SHALL emit structured subphase logs for recorder startup so the actual latency source is attributable without ad hoc instrumentation.

#### Scenario: Stream-open subphases are logged
- **WHEN** the app opens the recorder stream for an on-demand start
- **THEN** the logs SHALL make recorder creation, device open, stream-play readiness, first-packet readiness, and any held startup gate duration independently diagnosable

#### Scenario: Start-acknowledgement subphases are logged
- **WHEN** the app transitions from an open stream into capture-ready recording
- **THEN** the logs SHALL make the start command dispatch, worker-side application, first readiness evidence, and acknowledgement emission independently diagnosable

### Requirement: Startup Feedback and Overlay Fallback Logs
The system SHALL emit structured logs for pre-ready startup feedback and wake-related overlay fallback behavior.

#### Scenario: Pre-ready shell presentation is logged
- **WHEN** the app shows the neutral pre-ready shell for an accepted on-demand start
- **THEN** the logs SHALL record that event as a separate startup milestone from recording bars shown

#### Scenario: Wake-related fallback is logged
- **WHEN** overlay startup feedback or recording overlay presentation uses cached or coarse positioning because authoritative AX lookup was slow, unstable, or unavailable
- **THEN** the logs SHALL include the fallback reason and the chosen fallback strategy as structured fields

#### Scenario: Later overlay refinement is logged
- **WHEN** the overlay later refines its position after an earlier fallback presentation
- **THEN** the logs SHALL make that refinement visible as a separate structured event

### Requirement: On-Demand Startup Verification Contract
The system SHALL support manual and independent verification of startup-latency improvements from the unified log file alone.

#### Scenario: Developer verifies perceived-latency improvement
- **WHEN** a developer compares before and after logs for an on-demand startup optimization pass
- **THEN** the logs SHALL show whether first visible feedback moved earlier independently from any capture-ready change

#### Scenario: Developer verifies actual readiness improvement
- **WHEN** a developer compares before and after logs for the same startup path
- **THEN** the logs SHALL show whether capture-ready acknowledgement also improved independently from earlier visual feedback

#### Scenario: Wake-path verification remains attributable
- **WHEN** a developer validates app foreground, wake, or long-idle on-demand starts after topology cache refresh is already working
- **THEN** the logs SHALL make it possible to attribute the remaining startup cost to input delivery, stream open, recorder acknowledgement, or overlay fallback
- **AND** SHALL NOT require the reviewer to infer those causes only from free-form message text

#### Scenario: Repeated-run verification is supported
- **WHEN** a developer runs the same startup scenario multiple times to smooth out variance
- **THEN** the logs SHALL remain structured enough to compare repeated runs by trigger or session correlation
- **AND** SHALL support reporting median and slowest-pass timing for that scenario
