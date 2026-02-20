# transcript-insertion Specification

## Purpose
TBD - created by archiving change add-smart-transcript-insertion. Update Purpose after archive.
## Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using sentence-aware casing and conservative punctuation normalization when smart insertion is enabled.

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

#### Scenario: Selection-replace strips trailing sentence punctuation during lowercase/digit continuation
- **WHEN** insertion replaces a non-empty selection
- **AND** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the next non-whitespace character to the right is a lowercase letter or digit
- **THEN** one trailing sentence punctuation mark is removed before paste

#### Scenario: No-selection insert strips trailing sentence punctuation during lowercase/digit continuation
- **WHEN** insertion does not replace a selection
- **AND** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the next non-whitespace character to the right is a lowercase letter or digit
- **THEN** one trailing sentence punctuation mark is removed before paste

#### Scenario: Trailing punctuation is preserved when continuation is uppercase
- **WHEN** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the next non-whitespace character to the right is uppercase
- **THEN** trailing punctuation is preserved

#### Scenario: Abbreviation-like token preserves trailing punctuation
- **WHEN** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the transcript token is abbreviation-like with internal dots (for example `e.g.` or `U.S.`)
- **THEN** trailing punctuation is preserved

#### Scenario: Duplicate sentence punctuation at boundary is collapsed
- **WHEN** transcript ends with sentence punctuation (`.`, `?`, or `!`)
- **AND** the immediate right boundary character is the same punctuation mark
- **THEN** one duplicate punctuation mark is removed from the inserted transcript text

### Requirement: Smart Boundary Spacing
The system SHALL insert spacing around transcript text when boundaries indicate adjacent word-like text, including sentence-punctuation boundaries.

#### Scenario: Leading boundary requires space for word adjacency
- **WHEN** immediate left boundary character and transcript first significant character are both word-like
- **THEN** one leading space is added unless transcript already starts with whitespace

#### Scenario: Leading boundary requires space after sentence punctuation
- **WHEN** immediate left boundary is sentence punctuation (`.`, `?`, or `!`) with no whitespace between cursor and punctuation
- **AND** transcript first significant character is word-like
- **THEN** one leading space is added unless transcript already starts with whitespace

#### Scenario: Trailing boundary requires space
- **WHEN** immediate right boundary character and transcript last significant character are both word-like
- **THEN** one trailing space is added unless transcript already ends with whitespace

#### Scenario: Punctuation boundary avoids injected spaces for non-word starts
- **WHEN** transcript first significant character is punctuation-only
- **THEN** no leading word-boundary space is added

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
The system SHALL apply smart insertion formatting to transcript paste flows that use adaptive preparation, while supporting literal replay for `paste_last_transcript` by default, and SHALL register the exact pasted payload for undo.

#### Scenario: Live transcription paste remains adaptive
- **WHEN** `transcribe` flow pastes transcript output
- **THEN** shared smart insertion formatter is used

#### Scenario: Paste-last-transcript default is literal replay
- **WHEN** `paste_last_transcript` is triggered
- **AND** `paste_last_use_smart_insertion` is disabled or missing
- **THEN** shared paste utility uses literal preparation for paste-last
- **AND** pasted text equals History primary text (`effective_text`) exactly

#### Scenario: Paste-last-transcript adaptive mode is opt-in
- **WHEN** `paste_last_transcript` is triggered
- **AND** `paste_last_use_smart_insertion` is enabled
- **THEN** shared smart insertion formatter is used for paste-last

#### Scenario: Refine-last-transcript remains adaptive
- **WHEN** `refine_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Repeated refine uses latest refined history text
- **WHEN** `refine_last_transcript` is triggered for the latest history row
- **AND** that row has a non-empty `post_processed_text`
- **THEN** refine input uses that `post_processed_text`
- **AND** if no non-empty `post_processed_text` exists, refine input falls back to `raw_text`

#### Scenario: macOS refine replacement re-selection is best-effort
- **WHEN** `refine_last_transcript` runs on macOS
- **AND** latest row has non-empty `inserted_text`
- **THEN** system attempts AX re-selection of that `inserted_text` before paste
- **AND** if AX re-selection fails (or `inserted_text` is unavailable), refine paste continues at current cursor/selection with user feedback

#### Scenario: Refine history commit requires successful paste
- **WHEN** `refine_last_transcript` produces refined output
- **AND** paste is skipped or fails (`did_paste = false` or paste error)
- **THEN** latest row `post_processed_text` and `inserted_text` are not updated by that refine attempt

#### Scenario: Undo capture stores actual pasted text
- **WHEN** a transcript paste succeeds through the shared paste utility
- **THEN** the registered undo payload uses the `pasted_text` returned by that paste operation
- **AND** triggering configured `undo_last_transcript` reverses that exact paste as a single operation

### Requirement: Paste-Last Formatting Mode Setting
The system SHALL provide a user setting to control whether `paste_last_transcript` uses literal replay or adaptive smart insertion formatting.

#### Scenario: New and existing users default to literal mode
- **WHEN** settings are initialized or loaded from older versions where this setting does not exist
- **THEN** `paste_last_use_smart_insertion` defaults to `false`

#### Scenario: User enables adaptive mode
- **WHEN** user enables the paste-last smart insertion setting
- **THEN** subsequent `paste_last_transcript` actions use adaptive smart insertion preparation
