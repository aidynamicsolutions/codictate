## ADDED Requirements
### Requirement: On-Demand Microphone Warm Path
The system SHALL reduce on-demand microphone startup latency by beginning a short-lived warm path only after a concrete user recording trigger begins.

#### Scenario: Fn press starts warm path before recording commit
- **WHEN** the user presses `Fn` for push-to-talk transcription
- **AND** the app is in on-demand microphone mode
- **AND** transcription start is otherwise allowed
- **THEN** the system SHALL begin microphone prearm work before recording commit
- **AND** SHALL allow the subsequent recording start to reuse that in-flight or ready work instead of opening a second stream

#### Scenario: Shortcut trigger starts warm path outside Fn flow
- **WHEN** the user starts on-demand transcription through a non-Fn shortcut path
- **AND** that shortcut path does not expose a separate pre-session key-down hook before recording start begins
- **THEN** the system SHALL begin the warm path at the earliest synchronous recording-start boundary available for that trigger
- **AND** SHALL apply the same dedupe and cancellation semantics as push-to-talk

#### Scenario: Fn press during active hands-free session does not start a parallel warm path
- **WHEN** a hands-free recording session is already active
- **AND** the user presses `Fn`
- **THEN** the system SHALL NOT begin a separate push-to-talk warm path for that trigger
- **AND** SHALL NOT open a parallel microphone stream for that `Fn` press

#### Scenario: Always-on mode skips warm path
- **WHEN** the app is in always-on microphone mode
- **THEN** the system SHALL NOT start a separate prearm warm path for a recording trigger

### Requirement: Warm Path Auto-Close and Cancellation Safety
The system SHALL not leave unused warm microphone streams open after a cancelled or abandoned on-demand start.

#### Scenario: Trigger released before recording commit
- **WHEN** a warm path opens the microphone stream
- **AND** the user releases push-to-talk before recording commit
- **THEN** the system SHALL close the warm stream after a bounded grace window of no more than 1 second
- **AND** SHALL return to idle without blocking the next trigger

#### Scenario: Start blocked after warm path begins
- **WHEN** a warm path begins
- **AND** transcription start is later blocked by maintenance mode, permission denial, or state cancellation
- **THEN** the system SHALL synchronously close any warm stream it owns without waiting for the auto-close grace timer
- **AND** SHALL leave no stale ownership that prevents future starts

### Requirement: Dedupe Concurrent Stream-Open Work
The system SHALL serialize and deduplicate microphone stream-open work across warm path and recording start paths.

#### Scenario: Recording start arrives during in-flight prearm
- **WHEN** the warm path is already opening the stream
- **AND** the recording start path begins for the same user trigger
- **THEN** the system SHALL wait on or reuse the same open work
- **AND** SHALL NOT issue a second independent stream-open attempt for that start

#### Scenario: New trigger supersedes previous warm path
- **WHEN** a newer recording trigger supersedes an older in-flight warm path
- **THEN** the older warm path SHALL clean up safely
- **AND** SHALL NOT commit recording state for the newer trigger

### Requirement: Conservative Default-Route Cache Safety
The system SHALL treat system-default input routing more conservatively than explicit-device selection when deciding whether cached topology can be reused.

#### Scenario: Default route unchanged and route state current
- **WHEN** the selected microphone is the system default
- **AND** the app has route state confirming the default input route has not changed since warm or cached topology was prepared
- **THEN** the next on-demand start SHALL reuse that warm or cached topology for device resolution
- **AND** SHALL NOT require a fresh topology enumeration for that start

#### Scenario: Default route changed since cached topology was captured
- **WHEN** the selected microphone is the system default
- **AND** the app detects that input routing or default input device changed after the cached topology was captured
- **THEN** the next on-demand start SHALL resolve devices from fresh topology before opening the stream

#### Scenario: Default route state is unknown
- **WHEN** the selected microphone is the system default
- **AND** route state cannot be confirmed as unchanged
- **THEN** the system SHALL resolve devices from fresh topology before opening the stream

#### Scenario: Route monitoring unsupported or unavailable
- **WHEN** the selected microphone is the system default
- **AND** the current platform or runtime path does not provide route-change state
- **THEN** the system SHALL skip warm topology reuse for that start
- **AND** SHALL resolve devices from fresh topology before opening the stream

### Requirement: Explicit Device Disconnect Recovery
The system SHALL preserve existing microphone recovery behavior when an explicit microphone is missing at start time.

#### Scenario: Explicit microphone disconnected before start
- **WHEN** an explicitly selected microphone is unavailable at trigger time
- **THEN** the system SHALL attempt recovery using fresh topology and existing fallback-device heuristics
- **AND** SHALL continue to avoid virtual or low-quality fallback devices according to current app policy
