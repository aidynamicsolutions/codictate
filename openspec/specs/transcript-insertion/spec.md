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
The system SHALL apply smart insertion formatting to all transcript paste flows that route through the shared paste utility, register transformed paste payloads for undo, and persist transformed inserted text for the exact associated history row when applicable.

#### Scenario: Live transcription paste
- **WHEN** `transcribe` flow pastes transcript output
- **THEN** shared smart insertion formatter is used

#### Scenario: Paste-last-transcript shortcut
- **WHEN** `paste_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Refine-last-transcript shortcut
- **WHEN** `refine_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Undo capture stores transformed pasted text
- **WHEN** a transcript paste succeeds through the shared paste utility
- **THEN** the registered undo payload uses the transformed `pasted_text` value returned by the paste operation
- **AND** triggering configured `undo_last_transcript` reverses that transformed paste as a single operation

#### Scenario: Transcribe persists inserted text by exact row id
- **WHEN** `transcribe` saves a history entry and paste succeeds (`did_paste = true`)
- **THEN** transformed `PasteResult.pasted_text` is stored in `inserted_text` for that exact saved entry id
- **AND** implementation does not rely on latest-row lookup heuristics

#### Scenario: Refine-last keeps raw input and updates same row
- **WHEN** `refine_last_transcript` runs on latest history entry
- **THEN** refine input is latest row raw ASR text
- **AND** refine output updates `post_processed_text` for that same row id

#### Scenario: Refine-last inserted text update is paste-success-only
- **WHEN** `refine_last_transcript` paste succeeds (`did_paste = true`)
- **THEN** transformed `PasteResult.pasted_text` updates `inserted_text` for the same row id
- **AND** if paste is skipped or fails, `inserted_text` is not overwritten

