## ADDED Requirements
### Requirement: Dictionary Reason-Coded Decision Logs
The system SHALL emit reason-coded structured logs for dictionary matching accept/reject paths.

#### Scenario: Candidate rejected due to length ratio
- **WHEN** a dictionary candidate is rejected by length ratio guard
- **THEN** a debug log is emitted with reason `skip_length_ratio`
- **AND** log fields include `path`, `ngram`, `entry_input`, `entry_alias`, and `n`

#### Scenario: Candidate accepted by split fuzzy
- **WHEN** a split-token candidate is accepted
- **THEN** a structured log is emitted with reason `accept_split_fuzzy`
- **AND** includes `score`, `threshold`, and matching path

### Requirement: Dictionary Session Summary Metrics
The system SHALL emit per-session dictionary summary counters at info level.

#### Scenario: Session completes with dictionary enabled
- **WHEN** custom words are processed for a transcription
- **THEN** an info log reports `candidates_checked`, `exact_hits`, `split_fuzzy_hits`, and `standard_fuzzy_hits`
- **AND** includes reject counts grouped by reason code

## MODIFIED Requirements
### Requirement: Session Correlation
All dictionary matching logs SHALL be session-correlated with the active transcription span.

#### Scenario: Filter dictionary decisions by session
- **WHEN** a developer filters logs by `session=<id>`
- **THEN** candidate checks, acceptance reasons, and summary counters for that session are available together

