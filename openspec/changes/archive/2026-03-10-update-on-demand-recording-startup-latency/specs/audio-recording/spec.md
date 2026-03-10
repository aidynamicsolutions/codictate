## ADDED Requirements
### Requirement: Immediate Non-Ready Startup Feedback
The system SHALL be able to acknowledge an accepted on-demand recording trigger before capture-ready without implying that recording has already become ready.

#### Scenario: Accepted Fn trigger shows a neutral pre-ready shell
- **WHEN** the user presses `Fn` for an on-demand push-to-talk recording
- **AND** the app accepts that trigger after permission and maintenance-mode checks
- **AND** overlay feedback is enabled
- **THEN** the system SHALL show an immediate neutral pre-ready overlay shell
- **AND** that shell SHALL remain visually distinct from the recording-bars state

#### Scenario: Accepted non-Fn on-demand trigger shows a neutral pre-ready shell
- **WHEN** the user starts an on-demand recording through another supported shortcut path
- **AND** that start is accepted
- **AND** overlay feedback is enabled
- **THEN** the system SHALL show the same neutral pre-ready shell before capture-ready
- **AND** SHALL preserve the same readiness contract as the Fn path

#### Scenario: Early feedback is not delayed by a fixed readability timer
- **WHEN** the app shows pre-ready startup feedback for an accepted on-demand start
- **THEN** it SHALL NOT delay that feedback behind a fixed readability holdback such as the current `220 ms` non-Bluetooth threshold

#### Scenario: Pre-ready shell replaces readable connecting text on the on-demand path
- **WHEN** the app shows pre-ready startup feedback for an accepted on-demand start
- **THEN** that feedback SHALL use the neutral shell rather than a readable text-based connecting state

#### Scenario: Overlay-disabled setups skip shell presentation only
- **WHEN** overlay feedback is disabled in settings
- **AND** the app accepts an on-demand start
- **THEN** the system SHALL skip presenting the pre-ready shell
- **AND** SHALL preserve the same audio startup and capture-readiness rules

### Requirement: Recording Bars Remain the Ready-To-Speak Signal
The system SHALL preserve recording bars as the first ready-to-speak indicator on the on-demand path.

#### Scenario: Bars wait for capture-ready
- **WHEN** an on-demand recording start is still opening the stream or waiting for capture-ready acknowledgement
- **THEN** the system SHALL NOT show recording bars yet

#### Scenario: Pre-ready shell does not imply readiness
- **WHEN** the neutral pre-ready shell is visible
- **THEN** the user-facing behavior SHALL continue to treat the microphone as not yet ready to speak

#### Scenario: Failed or cancelled start never shows bars
- **WHEN** an accepted on-demand start later fails, is cancelled, or is blocked before capture-ready
- **THEN** the system SHALL clean up any pre-ready feedback
- **AND** SHALL NOT briefly show recording bars for that aborted start

### Requirement: On-Demand Recorder Startup Optimization
The system SHALL optimize the stream-open path separately from topology reuse so cached-topology hits do not still pay unnecessary recorder-start latency.

#### Scenario: Cached-topology hit still optimizes recorder startup
- **WHEN** an on-demand start reuses cached topology and skips fresh device enumeration
- **THEN** the system SHALL still treat recorder startup as an optimization target
- **AND** SHALL NOT consider the startup path fully optimized merely because topology resolution was a cache hit

#### Scenario: Stream-open coordination remains single-flight
- **WHEN** prearm and user-triggered start overlap on the same on-demand startup
- **THEN** the system SHALL preserve single-flight stream-open safety
- **AND** SHALL NOT open a second independent microphone stream for the same startup attempt

#### Scenario: Startup optimization does not move microphone open off the user trigger path
- **WHEN** the app is idle and no user-triggered on-demand recording start is in progress
- **THEN** this optimization change SHALL NOT open or keep open the microphone stream solely for startup-latency improvement

### Requirement: Capture-Ready Handshake Optimization Preserves Safety
The system SHALL reduce or better structure post-open start latency without weakening the capture-ready safety contract.

#### Scenario: Capture-ready optimization preserves ready contract
- **WHEN** the app optimizes the recorder start handshake after stream open
- **THEN** it SHALL still require capture-ready evidence before transitioning into the recording-bars state

#### Scenario: Capture-ready optimization preserves first-speech capture
- **WHEN** the recorder start path is optimized
- **THEN** the system SHALL continue to preserve early speech according to the current pre-roll and start-acknowledgement contract
- **AND** SHALL NOT trade away first-speech capture correctness for lower perceived startup latency

### Requirement: Post-Wake Overlay Fallback
The system SHALL avoid blocking on-demand startup feedback solely because authoritative overlay target lookup is temporarily unstable after wake or unlock.

#### Scenario: Wake-related AX instability uses immediate fallback positioning
- **WHEN** startup overlay presentation occurs within 5 seconds after a wake or unlock event, or authoritative Accessibility lookup fails with a transient error or timeout that makes overlay targeting unreliable
- **THEN** the app SHALL use cached or coarse overlay positioning immediately for startup feedback
- **AND** SHALL refine that positioning asynchronously later when authoritative state becomes available

#### Scenario: Overlay fallback does not change readiness semantics
- **WHEN** overlay startup feedback uses a fallback position
- **THEN** that fallback SHALL affect only where the overlay is shown
- **AND** SHALL NOT relax the rule that recording bars appear only after capture-ready
