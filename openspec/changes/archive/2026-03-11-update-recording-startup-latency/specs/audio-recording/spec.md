## MODIFIED Requirements
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

#### Scenario: Startup optimization uses paused streams instead of teardowns
- **WHEN** the app is idle and no user-triggered on-demand recording start is in progress
- **THEN** this optimization change SHALL pause the microphone stream to release the hardware privacy indicator
- **AND** SHALL NOT completely destroy and rebuild the stream during sequential recorded sessions
