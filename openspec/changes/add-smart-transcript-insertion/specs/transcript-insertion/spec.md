## ADDED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using sentence-aware casing and boundary-aware spacing when smart insertion is enabled.

#### Scenario: Mid-sentence title-case correction is de-capitalized
- **WHEN** insertion occurs mid-sentence
- **AND** transcript starts with a title-case alphabetic word
- **THEN** the first alphabetic character is de-capitalized before paste

#### Scenario: Sentence-start insertion is capitalized
- **WHEN** insertion occurs at document start or after sentence terminator punctuation (`.`, `!`, `?`) with optional whitespace
- **AND** transcript starts with a lowercase alphabetic character
- **THEN** the first alphabetic character is capitalized before paste

#### Scenario: Acronym-style uppercase prefix is preserved
- **WHEN** insertion occurs mid-sentence
- **AND** transcript begins with acronym-style uppercase letters
- **THEN** the transcript casing is not de-capitalized

### Requirement: Smart Boundary Spacing
The system SHALL insert spacing around transcript text when boundaries indicate adjacent word-like text.

#### Scenario: Leading boundary requires space
- **WHEN** immediate left boundary character and transcript first significant character are both word-like
- **THEN** one leading space is added unless transcript already starts with whitespace

#### Scenario: Trailing boundary requires space
- **WHEN** immediate right boundary character and transcript last significant character are both word-like
- **THEN** one trailing space is added unless transcript already ends with whitespace

#### Scenario: Punctuation boundary avoids injected spaces
- **WHEN** either insertion boundary is punctuation-only
- **THEN** no word-boundary space is added for that side

### Requirement: Platform Scope and Fallback
The system SHALL use context-aware insertion on macOS and preserve legacy behavior elsewhere or when context is unavailable.

#### Scenario: macOS context-aware insertion
- **WHEN** smart insertion is enabled on macOS
- **AND** insertion context is successfully captured
- **THEN** sentence-aware casing and boundary-aware spacing are applied

#### Scenario: Context unavailable fallback
- **WHEN** smart insertion is enabled
- **AND** insertion context cannot be captured
- **THEN** legacy trailing-space behavior is used (`text + " "`)

#### Scenario: Smart insertion disabled
- **WHEN** smart insertion setting is disabled
- **THEN** transcript text is pasted without smart casing/spacing transformation

### Requirement: Shared Paste Flow Coverage
The system SHALL apply smart insertion formatting to all transcript paste flows that route through the shared paste utility.

#### Scenario: Live transcription paste
- **WHEN** `transcribe` flow pastes transcript output
- **THEN** shared smart insertion formatter is used

#### Scenario: Paste-last-transcript shortcut
- **WHEN** `paste_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Refine-last-transcript shortcut
- **WHEN** `refine_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used
