## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using sentence-aware casing, conservative selection-replace punctuation sanitation, and boundary-aware spacing when smart insertion is enabled.

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

#### Scenario: Selection-replace strips trailing sentence punctuation during lowercase continuation
- **WHEN** insertion replaces a non-empty selection
- **AND** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the next non-whitespace character to the right is a lowercase letter or digit
- **THEN** one trailing sentence punctuation mark is removed before paste

#### Scenario: Selection-replace preserves trailing punctuation when continuation is uppercase
- **WHEN** insertion replaces a non-empty selection
- **AND** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the next non-whitespace character to the right is uppercase
- **THEN** trailing punctuation is preserved

#### Scenario: Abbreviation-like token preserves trailing punctuation
- **WHEN** insertion replaces a non-empty selection
- **AND** transcript ends with trailing sentence punctuation (`.`, `?`, or `!`)
- **AND** the transcript token is abbreviation-like with internal dots (for example `e.g.` or `U.S.`)
- **THEN** trailing punctuation is preserved

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
