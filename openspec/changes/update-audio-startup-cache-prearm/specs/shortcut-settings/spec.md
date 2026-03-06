## ADDED Requirements
### Requirement: Shortcut Start Cache Correctness
The system SHALL keep shortcut-triggered on-demand starts fast while ensuring input-device topology is refreshed when cached default-route assumptions can be stale.

#### Scenario: macOS route change invalidates cached default topology
- **GIVEN** microphone selection is `default`
- **AND** the app cache currently has a fresh input-device snapshot
- **WHEN** macOS reports an input-route or device-topology change before shortcut start
- **THEN** the next shortcut-triggered start refreshes topology before opening the stream

#### Scenario: Concurrent starts do not lose route-change refresh requirement
- **GIVEN** route-change monitoring indicates a newer route-change generation than the generation already applied by the cache
- **AND** multiple start/pre-arm paths race near-simultaneously
- **WHEN** one path begins stream startup first
- **THEN** other concurrent start paths still treat topology as needing refresh until a successful fresh enumeration records the newer generation as applied

#### Scenario: Explicit selection keeps cached fast path
- **GIVEN** the user explicitly selected a named microphone
- **AND** cached topology is fresh and clean
- **WHEN** shortcut-triggered start runs
- **THEN** the start path MAY use cached topology without forced refresh

#### Scenario: Clamshell active selection is treated as explicit for cache policy
- **GIVEN** clamshell mode is active
- **AND** `clamshell_microphone` is configured to a named device
- **WHEN** start cache policy is evaluated
- **THEN** the clamshell microphone is treated as the active explicit selection
- **AND** default-route-only forced refresh rules are not applied

#### Scenario: Forced refresh is queued while in-flight refresh is active
- **GIVEN** an input-device cache refresh is already in-flight
- **WHEN** a `Force` refresh request arrives
- **THEN** a follow-up refresh is queued
- **AND** a subsequent enumeration runs before in-flight state is cleared

### Requirement: Startup Prewarm Preference Safety
Startup Bluetooth prewarm SHALL NOT persist auto-switched fallback microphone settings.

#### Scenario: Startup prewarm detects disconnected explicit device
- **GIVEN** startup prewarm opens stream context before user-started recording
- **AND** the explicit selected microphone is unavailable
- **WHEN** fallback resolution picks an alternative input device for warmup
- **THEN** the stream open proceeds without writing fallback selection to persisted settings
