# observability Specification

## Purpose
TBD - created by archiving change add-tracing-instrumentation. Update Purpose after archive.
## Requirements
### Requirement: Unified Log File

All application components SHALL write logs to a single file for easy debugging.

#### Scenario: Single file contains all logs
- **WHEN** a recording session runs
- **THEN** logs from Rust, Python sidecar, and Frontend appear in one log file
- **AND** the file is located in the OS log directory

### Requirement: Logfmt Format

Logs SHALL use Logfmt format for human-readable structured output.

#### Scenario: Log entry format
- **WHEN** an event is logged
- **THEN** the entry follows format: `TIMESTAMP LEVEL session=ID target=COMPONENT msg="MESSAGE"`
- **AND** the entry is on a single line
- **AND** the entry is readable without parsing tools

### Requirement: Session Correlation

All logs from a single recording session SHALL include a session_id.

#### Scenario: Filter by session
- **WHEN** a developer runs `grep 'session=abc123' handy.log`
- **THEN** all logs related to that recording session are returned
- **AND** logs appear in chronological order across components

### Requirement: Log Retention

The system SHALL retain 7 days of log files.

#### Scenario: Log rotation
- **WHEN** logs rotate daily
- **THEN** the 7 most recent log files are kept
- **AND** older files are automatically deleted

### Requirement: Debugging Skill

The system SHALL provide an AI skill for debugging guidance.

#### Scenario: Debugging workflow
- **WHEN** a user asks how to debug a failed recording
- **THEN** the AI provides the log file location
- **AND** the AI shows how to filter by session_id

