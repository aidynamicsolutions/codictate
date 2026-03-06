## ADDED Requirements
### Requirement: Input Route Change Cache Logs
The system SHALL emit structured logs when system input-route changes force cache-bypass refresh behavior.

#### Scenario: System route change forces refresh
- **WHEN** a newer route-change generation is detected before stream start
- **THEN** a structured log is emitted indicating route-change detection and forced refresh
- **AND** refresh logs include reason code `system_route_changed`

#### Scenario: Default-route fallback refresh when monitor unavailable
- **WHEN** microphone selection is `default` and native route monitoring is unavailable
- **THEN** a structured log is emitted indicating cached topology bypass for safety

#### Scenario: Forced refresh queued behind in-flight refresh
- **GIVEN** a cache refresh is already in-flight
- **WHEN** a `Force` refresh request arrives
- **THEN** a structured log is emitted with event code `cache_refresh_queued_force`
- **AND** a follow-up refresh lifecycle log is emitted before in-flight refresh is cleared

### Requirement: Prewarm Persistence Guard Logs
The system SHALL preserve prewarm lifecycle observability while preventing startup prewarm from mutating persisted microphone selection.

#### Scenario: Startup prewarm fallback without persistence
- **WHEN** startup prewarm resolves a fallback microphone
- **THEN** prewarm lifecycle logs are emitted
- **AND** no auto-switch persistence event is emitted from the prewarm context
