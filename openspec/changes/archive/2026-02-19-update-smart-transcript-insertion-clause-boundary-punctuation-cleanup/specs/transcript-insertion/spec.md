## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using language-profile-aware punctuation normalization and conservative fallbacks when smart insertion is enabled.

#### Scenario: Whitespace profiles resolve sentence-vs-clause boundary conflicts by preferring right-boundary clause punctuation
- **WHEN** insertion runs under `CasedWhitespace` or `UncasedWhitespace`
- **AND** inserted text ends with sentence punctuation
- **AND** the immediate right boundary is clause punctuation (for example `,`, `;`, `:` or Arabic/fullwidth variants)
- **THEN** one inserted terminal sentence punctuation mark is removed
- **AND** the existing right-boundary clause punctuation remains the effective boundary mark

#### Scenario: Abbreviation-like period endings are preserved before clause punctuation in whitespace profiles
- **WHEN** insertion runs under `CasedWhitespace` or `UncasedWhitespace`
- **AND** inserted text ends with a period-like sentence mark
- **AND** inserted text token is abbreviation-like with internal dots (for example `e.g.` or `U.S.`)
- **AND** the immediate right boundary is clause punctuation
- **THEN** inserted terminal period punctuation is preserved

#### Scenario: Conservative profile clause-boundary behavior is unchanged
- **WHEN** insertion runs under `Conservative`
- **AND** inserted text ends with sentence punctuation
- **AND** the immediate right boundary is clause punctuation
- **THEN** clause-boundary conflict cleanup is not applied

#### Scenario: No-boundary-spacing profile clause-boundary behavior is unchanged
- **WHEN** insertion runs under `NoBoundarySpacing`
- **AND** inserted text ends with sentence punctuation
- **AND** the immediate right boundary is clause punctuation
- **THEN** clause-boundary conflict cleanup is not applied
