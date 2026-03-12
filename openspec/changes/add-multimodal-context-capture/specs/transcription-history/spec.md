## MODIFIED Requirements

### Requirement: History Entry Text Provenance
The system SHALL preserve raw ASR transcript content, refined spoken text, exact replay payloads, and optional multimodal metadata through explicit history fields.

#### Scenario: Additive migration preserves existing rows
- **WHEN** an existing user upgrades to a build that includes multimodal history metadata
- **THEN** migration only adds nullable multimodal sidecar storage
- **AND** existing `transcription_text`, `post_processed_text`, `post_process_prompt`, and `inserted_text` values remain unchanged

#### Scenario: Effective text fallback on legacy rows
- **WHEN** a history row has `inserted_text = NULL`
- **AND** row has `post_processed_text`
- **THEN** row `effective_text` resolves to `post_processed_text`

#### Scenario: Effective text fallback to raw ASR
- **WHEN** a history row has `inserted_text = NULL`
- **AND** row has `post_processed_text = NULL`
- **THEN** row `effective_text` resolves to `transcription_text`

#### Scenario: Multimodal sidecar does not replace spoken transcript fields
- **WHEN** a history row includes multimodal sidecar metadata
- **THEN** `transcription_text` remains the spoken transcript text
- **AND** `post_processed_text` remains the refined spoken transcript text if present
- **AND** `inserted_text` remains the exact replay payload if the entry was pasted

#### Scenario: Raw alias is always present
- **WHEN** history row is returned through API
- **THEN** `raw_text` equals stored `transcription_text`

### Requirement: History Search Coverage
History queries SHALL match replay payload text, spoken preview text, and raw ASR text.

#### Scenario: Match by exact replay payload
- **WHEN** user searches for a token that appears only in the exact replay payload
- **THEN** matching row is returned

#### Scenario: Match by spoken preview text
- **WHEN** user searches for a token that appears in the spoken or refined transcript text
- **THEN** matching row is returned

#### Scenario: Match by raw ASR text
- **WHEN** user searches for a token that appears only in `raw_text`
- **THEN** matching row is returned

### Requirement: History UI Disclosure Behavior
History UI SHALL keep rows compact while exposing exact replay and raw transcript details on demand.

#### Scenario: Primary row text uses spoken preview
- **WHEN** history row is rendered
- **THEN** the primary visible row text uses spoken or refined transcript text
- **AND** does not dump the rendered context block or figure footer into the main preview

#### Scenario: Multimodal badge visibility
- **WHEN** history row includes multimodal sidecar metadata
- **THEN** the row shows compact multimodal disclosure such as badges or figure counts

#### Scenario: Original transcript action visibility
- **WHEN** `raw_text` differs from the spoken preview text
- **THEN** row shows an `Original transcript` toggle action
- **AND** row hides this action when values are identical

#### Scenario: Inline original transcript disclosure
- **WHEN** user activates `Original transcript`
- **THEN** row expands inline to show raw ASR text
- **AND** disclosure is keyboard operable with explicit expanded state semantics

#### Scenario: Copy action uses replay payload
- **WHEN** user activates row copy action
- **THEN** copied value is the exact replay payload when available
- **AND** otherwise copied value falls back to the deterministically rendered payload or spoken preview text

#### Scenario: Raw-only search hint
- **WHEN** current search query matches row `raw_text` but not the spoken preview text
- **AND** original transcript panel is collapsed
- **THEN** row shows a hint that match exists in original transcript
