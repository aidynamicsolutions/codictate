## ADDED Requirements

### Requirement: Centralized Error Monitoring
The system SHALL capture and report errors from both the Backend (Rust) and Frontend (TypeScript) to a centralized error tracking service (Sentry).

#### Scenario: Rust Panic Reporting
- **WHEN** the Rust backend panics
- **THEN** a minidump or error report is sent to Sentry
- **AND** the report includes the application version and OS details

#### Scenario: Frontend Exception Reporting
- **WHEN** an unhandled JavaScript exception occurs
- **THEN** the error is captured and sent to Sentry
- **AND** the report includes the component stack trace

#### Scenario: Offline Error Caching
- **WHEN** an error occurs while the device is offline
- **THEN** the error report is cached locally
- **AND** the report is sent to Sentry when connectivity is restored

### Requirement: Privacy and PII Scrubbing
The system SHALL strictly scrub Personally Identifiable Information (PII) from error reports before they leave the user's device.

#### Scenario: PII Scrubbing
- **WHEN** an error report is generated
- **THEN** any potential PII (IP addresses, specific user paths, email patterns) is removed or masked
- **AND** this sanitization happens on the client-side (Rust `before_send` or Frontend equivalent)

#### Scenario: Default Anonymity
- **WHEN** an error is reported
- **THEN** no unique user identifiers are attached unless explicitly opted-in for debugging
