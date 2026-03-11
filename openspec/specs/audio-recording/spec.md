# audio-recording Specification

## Purpose
TBD - created by archiving change update-audio-topology-lifecycle-refresh. Update Purpose after archive.
## Requirements
### Requirement: Event-Driven Topology Cache Refresh
The system SHALL refresh cached input topology in the background on meaningful lifecycle and route events without opening the microphone stream.

#### Scenario: App foreground refreshes topology cache
- **WHEN** the app becomes foreground or active after previously being backgrounded
- **THEN** the system SHALL schedule a background topology refresh
- **AND** SHALL update cached device metadata if refresh succeeds
- **AND** SHALL NOT open the recorder stream as part of that refresh

#### Scenario: Wake refreshes topology cache
- **WHEN** the device wakes from sleep
- **AND** the current platform or runtime path supports wake lifecycle notifications
- **THEN** the system SHALL schedule a background topology refresh before the next likely push-to-talk action
- **AND** SHALL keep that refresh independent from recorder stream opening

#### Scenario: Wake hook unavailable on current platform
- **WHEN** the current platform or runtime path does not provide wake lifecycle notifications
- **THEN** the system SHALL skip registering that refresh trigger
- **AND** SHALL continue to use the existing cache and fresh startup fallback rules rather than guessing wake state

#### Scenario: Audio-route change refreshes topology cache
- **WHEN** the app observes an input-route or default-input change event
- **THEN** the system SHALL refresh cached topology in the background
- **AND** SHALL record the route generation associated with the refreshed cache entry

#### Scenario: Duplicate lifecycle refresh requests are coalesced
- **WHEN** multiple supported refresh triggers arrive while a topology refresh is already in flight
- **THEN** the system SHALL coalesce those requests into a single refresh pass
- **AND** SHALL NOT run parallel device enumerations for the same shared cache update

#### Scenario: Startup does not block on in-flight refresh
- **WHEN** a user-triggered recording start begins while a background topology refresh is still in flight
- **THEN** the system SHALL NOT block recording start solely to await that refresh
- **AND** SHALL treat the topology-resolution decision point as the moment immediately before startup would otherwise begin a fresh live device enumeration or commit a cached topology target for stream-open
- **AND** SHALL either reuse refreshed topology only if it is available by that decision point or continue with the existing cache and fresh fallback rules

#### Scenario: Background refresh failure does not block recording
- **WHEN** a background cache refresh fails
- **THEN** the system SHALL preserve the existing conservative startup fallback behavior
- **AND** SHALL NOT block the next user-triggered recording start solely because the background refresh failed

### Requirement: Long-Lived Topology Cache
The system SHALL retain input topology cache entries for up to 24 hours unless a refresh or invalidation occurs first.

#### Scenario: Explicit microphone reuses long-idle cache
- **WHEN** a specific microphone is selected
- **AND** no ready warm stream exists for that trigger
- **AND** the cached topology is less than 24 hours old
- **AND** no startup-path validation rule invalidates that cached entry
- **THEN** the next on-demand start SHALL resolve that microphone using cached topology without a fresh device enumeration

#### Scenario: Cache older than 24 hours forces fresh enumeration
- **WHEN** cached topology is older than 24 hours
- **THEN** the next startup that needs topology SHALL refresh it before trusting the cache

#### Scenario: Background refresh renews cache lifetime without opening stream
- **WHEN** a supported lifecycle or route event refreshes topology successfully
- **THEN** the cache age SHALL be renewed from the time of that refresh
- **AND** SHALL NOT imply that a live microphone stream remains open

### Requirement: Lifecycle Refresh Preserves Default-Route Safety
The system SHALL keep default-route cache reuse gated by route-change confirmation even when background refresh updated topology earlier.

#### Scenario: Lifecycle refresh does not relax default-route safety
- **WHEN** a supported lifecycle event refreshed topology cache earlier
- **AND** the selected microphone is still `Default`
- **THEN** the next start SHALL still require route state confirming the default route is unchanged before cached topology can be reused

### Requirement: Cached Explicit-Device Open Retry
The system SHALL retry explicit-device startup once with fresh topology if opening a cached explicit-device target fails.

#### Scenario: Cached explicit-device open fails
- **WHEN** an explicit microphone appears valid in cached topology
- **AND** opening that cached target fails at startup
- **THEN** the system SHALL retry once with fresh topology before surfacing failure or fallback behavior

#### Scenario: Power-cycled external interface forces live re-enumeration
- **WHEN** an explicitly selected external audio interface disappears and later reappears with the same display name but a different underlying device identity
- **AND** cached topology still points at the stale pre-power-cycle device object
- **THEN** the required fresh retry SHALL perform a full live device enumeration equivalent to a new `list_input_devices()` pass
- **AND** SHALL NOT satisfy that retry by only refreshing cache metadata around the stale cached target

### Requirement: Microphone Selection Guidance
The system SHALL explain the consistency vs flexibility tradeoff between a specific microphone selection and `Default`.

#### Scenario: Onboarding recommends a specific built-in microphone for consistency
- **WHEN** onboarding presents microphone setup guidance
- **AND** a built-in or internal microphone is available to choose explicitly
- **THEN** the UI SHALL explain that selecting that microphone is the most consistent startup-speed option

#### Scenario: Microphone picker explains `Default`
- **WHEN** the user chooses or reviews microphone selection in the microphone picker
- **THEN** the UI SHALL explain that `Default` follows macOS input changes automatically
- **AND** SHALL describe `Default` as the more flexible choice rather than the most consistent low-latency choice
- **AND** SHALL NOT claim that `Default` is always slower than an explicit microphone selection

#### Scenario: Guidance stays accurate when no built-in microphone is available
- **WHEN** onboarding or microphone selection guidance is shown
- **AND** no built-in or internal microphone is available to recommend explicitly
- **THEN** the UI SHALL still explain the consistency vs flexibility tradeoff
- **AND** SHALL NOT recommend an unavailable built-in or internal microphone

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

#### Scenario: Startup optimization uses paused streams instead of teardowns
- **WHEN** the app is idle and no user-triggered on-demand recording start is in progress
- **THEN** this optimization change SHALL pause the microphone stream to release the hardware privacy indicator
- **AND** SHALL NOT completely destroy and rebuild the stream during sequential recorded sessions

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

