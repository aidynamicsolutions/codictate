## ADDED Requirements
### Requirement: History Entry Text Provenance
The system SHALL preserve raw ASR transcript content while exposing inserted-result parity through explicit history fields.

#### Scenario: Additive migration preserves existing rows
- **WHEN** an existing user upgrades to a build that includes `inserted_text`
- **THEN** migration only adds nullable column `inserted_text`
- **AND** existing `transcription_text`, `post_processed_text`, and `post_process_prompt` values remain unchanged

#### Scenario: Effective text fallback on legacy rows
- **WHEN** a history row has `inserted_text = NULL`
- **AND** row has `post_processed_text`
- **THEN** row `effective_text` resolves to `post_processed_text`

#### Scenario: Effective text fallback to raw ASR
- **WHEN** a history row has `inserted_text = NULL`
- **AND** row has `post_processed_text = NULL`
- **THEN** row `effective_text` resolves to `transcription_text`

#### Scenario: Raw alias is always present
- **WHEN** history row is returned through API
- **THEN** `raw_text` equals stored `transcription_text`

### Requirement: History Search Coverage
History queries SHALL match both effective text and raw ASR text.

#### Scenario: Match by effective inserted text
- **WHEN** user searches for a token that appears only in `effective_text`
- **THEN** matching row is returned

#### Scenario: Match by raw ASR text
- **WHEN** user searches for a token that appears only in `raw_text`
- **THEN** matching row is returned

### Requirement: History UI Disclosure Behavior
History UI SHALL keep rows compact while exposing raw transcript details on demand.

#### Scenario: Primary row text uses effective text
- **WHEN** history row is rendered
- **THEN** primary visible row text uses `effective_text`

#### Scenario: Original transcript action visibility
- **WHEN** `raw_text` differs from `effective_text`
- **THEN** row shows an `Original transcript` toggle action
- **AND** row hides this action when values are identical

#### Scenario: Inline original transcript disclosure
- **WHEN** user activates `Original transcript`
- **THEN** row expands inline to show raw ASR text
- **AND** disclosure is keyboard operable with explicit expanded state semantics

#### Scenario: Copy action behavior
- **WHEN** user activates row copy action
- **THEN** copied value is row `effective_text`

#### Scenario: Raw-only search hint
- **WHEN** current search query matches row `raw_text` but not row `effective_text`
- **AND** original transcript panel is collapsed
- **THEN** row shows a hint that match exists in original transcript
