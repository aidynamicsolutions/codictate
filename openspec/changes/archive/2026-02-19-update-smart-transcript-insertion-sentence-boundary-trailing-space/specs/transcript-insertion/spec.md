## MODIFIED Requirements
### Requirement: Smart Boundary Spacing
The system SHALL apply boundary spacing only for profiles that enable word-boundary spacing.

#### Scenario: Whitespace profiles add trailing sentence-boundary spacing after preserved punctuation
- **WHEN** the resolved profile is `CasedWhitespace` or `UncasedWhitespace`
- **AND** inserted text ends with supported sentence punctuation that is preserved
- **AND** inserted text starts with a word-like character
- **AND** the immediate right boundary is word-like
- **THEN** one trailing space is added after inserted text

#### Scenario: Punctuation-only insert does not force sentence-boundary trailing spacing
- **WHEN** the resolved profile is `CasedWhitespace` or `UncasedWhitespace`
- **AND** inserted text is punctuation-only
- **AND** the immediate right boundary is word-like
- **THEN** sentence-boundary trailing spacing is not forced

#### Scenario: No-boundary-spacing profile behavior is unchanged
- **WHEN** insertion runs under `NoBoundarySpacing`
- **THEN** leading and trailing boundary spacing is not injected

#### Scenario: Conservative profile behavior is unchanged
- **WHEN** insertion runs under `Conservative`
- **THEN** conservative fallback spacing rules remain in effect
