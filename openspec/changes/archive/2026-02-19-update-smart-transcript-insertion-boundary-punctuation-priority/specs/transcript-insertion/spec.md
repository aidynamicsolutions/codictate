## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using language-profile-aware punctuation normalization and conservative fallbacks when smart insertion is enabled.

#### Scenario: Duplicate sentence punctuation at boundary is collapsed
- **WHEN** inserted text ends with sentence punctuation
- **AND** right boundary punctuation is the same mark
- **THEN** one duplicate terminal mark is removed from inserted text

#### Scenario: Conflicting sentence punctuation at boundary prefers right-boundary mark in whitespace profiles
- **WHEN** insertion runs under `CasedWhitespace` or `UncasedWhitespace`
- **AND** inserted text ends with sentence punctuation
- **AND** right boundary punctuation is sentence punctuation with a different mark
- **THEN** one terminal mark is removed from inserted text
- **AND** the existing right-boundary punctuation remains the effective boundary mark

#### Scenario: Conservative profile conflicting punctuation behavior is unchanged
- **WHEN** insertion runs under `Conservative`
- **AND** inserted text ends with sentence punctuation
- **AND** right boundary punctuation differs
- **THEN** conflicting-mark boundary collapse is not applied

#### Scenario: No-boundary-spacing profile conflicting punctuation behavior is unchanged
- **WHEN** insertion runs under `NoBoundarySpacing`
- **AND** inserted text ends with sentence punctuation
- **AND** right boundary punctuation differs
- **THEN** conflicting-mark boundary collapse is not applied
