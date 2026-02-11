## ADDED Requirements

### Requirement: Anonymous Usage Analytics
The system SHALL collect anonymous usage data to understand application adoption and feature usage patterns.

#### Scenario: Anonymous Event Tracking
- **WHEN** a user performs a key action (e.g., completes a transcription)
- **THEN** an event is sent to the analytics provider (Aptabase)
- **AND** the event contains NO personally identifiable information (PII)
- **AND** the user ID is a rotated hash that prevents long-term tracking

### Requirement: Offline Analytics Queueing
The system SHALL prevent data loss for analytics events generated while the device is offline.

#### Scenario: Offline Event Buffering
- **WHEN** an event is tracked while the device is offline
- **THEN** the event is stored locally in a persistent queue
- **AND** the queue is flushed to the analytics provider once connectivity is restored
