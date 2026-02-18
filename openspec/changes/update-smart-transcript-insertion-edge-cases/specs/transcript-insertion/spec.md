## MODIFIED Requirements
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

### Requirement: Shared Paste Flow Coverage
The system SHALL apply smart insertion formatting to all transcript paste flows that route through the shared paste utility and register transformed paste payloads for undo.

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
