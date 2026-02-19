## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using language-profile-aware punctuation normalization and conservative fallbacks when smart insertion is enabled.

#### Scenario: Language profile is resolved from selected language
- **WHEN** smart insertion formatting runs
- **THEN** the selected language is normalized from BCP47-style tags to a canonical internal key
- **AND** a deterministic smart insertion profile is selected from that normalized key

#### Scenario: Conservative profile uses boundary-safe trailing-space fallback
- **WHEN** the resolved profile is conservative (such as `auto`, `tr`, or unknown language)
- **AND** insertion context is available
- **THEN** smart insertion appends a trailing space only when the inserted text ends with a word-like character and the right boundary is word-like
- **AND** smart insertion does not add a trailing space before punctuation boundaries

#### Scenario: Unicode numeric continuation strips trailing sentence punctuation
- **WHEN** transcript ends with supported sentence punctuation
- **AND** right continuation is a Unicode numeric character
- **THEN** one trailing sentence punctuation mark is removed before paste

#### Scenario: Uncased profile preserves casing
- **WHEN** insertion runs under an uncased language profile
- **THEN** capitalization/de-capitalization transforms are skipped

#### Scenario: CJK punctuation can be normalized under no-boundary-spacing profile
- **WHEN** transcript and right boundary contain duplicate CJK terminal punctuation
- **THEN** one duplicate punctuation mark is removed from inserted text

### Requirement: Smart Boundary Spacing
The system SHALL apply boundary spacing only for profiles that enable word-boundary spacing.

#### Scenario: Cased and uncased whitespace profiles can add boundary spacing
- **WHEN** the resolved profile is `CasedWhitespace` or `UncasedWhitespace`
- **THEN** leading/trailing boundary spacing may be applied based on adjacent word-like boundaries

#### Scenario: No-boundary-spacing profile skips boundary spacing
- **WHEN** the resolved profile is `NoBoundarySpacing`
- **THEN** leading and trailing boundary spacing is not injected

#### Scenario: Context unavailable still uses legacy fallback
- **WHEN** insertion context is unavailable while smart insertion is enabled
- **THEN** the formatter uses legacy trailing-space behavior (`text + " "`)

### Requirement: Shared Paste Flow Coverage
The system SHALL apply language-profile-aware smart insertion to all transcript paste flows that route through the shared paste utility and preserve transformed undo payload behavior.

#### Scenario: Live transcription paste
- **WHEN** `transcribe` flow pastes transcript output
- **THEN** shared profile-aware smart insertion formatter is used

#### Scenario: Paste-last-transcript shortcut
- **WHEN** `paste_last_transcript` flow pastes text
- **THEN** shared profile-aware smart insertion formatter is used

#### Scenario: Refine-last-transcript shortcut
- **WHEN** `refine_last_transcript` flow pastes text
- **THEN** shared profile-aware smart insertion formatter is used

#### Scenario: Undo capture stores transformed pasted text
- **WHEN** a transcript paste succeeds through the shared paste utility
- **THEN** undo payload stores transformed `pasted_text` as the value to reverse
