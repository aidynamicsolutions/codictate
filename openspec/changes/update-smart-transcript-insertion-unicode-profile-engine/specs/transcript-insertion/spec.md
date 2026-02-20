## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL resolve smart insertion behavior from canonicalized language tags into deterministic script-aware profiles for supported transcription languages, while preserving conservative fallback behavior for unresolved or ambiguous cases.

#### Scenario: Canonical language variant resolves to deterministic profile
- **WHEN** selected language includes region or script variants (for example `pt-BR`, `zh-Hans`, `zh-TW`)
- **THEN** smart insertion canonicalizes and resolves to a deterministic profile
- **AND** equivalent variants map to the same profile behavior

#### Scenario: Ambiguous or unknown language remains conservative
- **WHEN** selected language cannot be confidently mapped to a safe profile
- **THEN** smart insertion uses conservative fallback behavior
- **AND** conservative mode avoids aggressive spacing/casing changes

#### Scenario: Conservative fallback remains context-safe
- **WHEN** conservative profile is active and insertion context is available
- **THEN** trailing-space behavior is applied only when boundary conditions indicate word continuation
- **AND** no extra space is injected before punctuation boundaries

### Requirement: Smart Boundary Spacing
The system SHALL apply boundary spacing and punctuation normalization using Unicode-aware heuristics appropriate to each resolved profile.

#### Scenario: Whitespace profiles use Unicode boundary-aware adjacency rules
- **WHEN** resolved profile enables boundary spacing
- **THEN** spacing decisions use Unicode-aware word/token boundary logic rather than ASCII-only checks

#### Scenario: No-boundary-spacing profile preserves script-native flow
- **WHEN** resolved profile is no-boundary-spacing
- **THEN** leading and trailing word-boundary spaces are not injected
- **AND** punctuation cleanup still follows profile-safe rules

#### Scenario: Heuristics preserve grapheme-safe output
- **WHEN** smart insertion transforms text containing combining characters or multi-codepoint graphemes
- **THEN** output remains grapheme-safe and valid Unicode text

## ADDED Requirements
### Requirement: Data-Driven Smart Insertion Profile Rules
The system SHALL load language/profile routing and punctuation rule metadata from a versioned data-driven profile table.

#### Scenario: Profile behavior updates from table data
- **WHEN** a supported language mapping is updated in the profile table
- **THEN** smart insertion behavior reflects the updated mapping without changing core branching logic

#### Scenario: Invalid profile table fails closed
- **WHEN** profile table data is invalid or incomplete
- **THEN** smart insertion falls back to conservative behavior
- **AND** emits diagnostic reason codes for troubleshooting

### Requirement: Smart Insertion Regression and Golden Coverage
The system SHALL maintain representative regression and golden fixtures for smart insertion profiles.

#### Scenario: Representative profile regression coverage
- **WHEN** smart insertion tests run
- **THEN** each profile has representative language cases covering spacing, casing, punctuation cleanup, and fallback decisions

#### Scenario: Golden fixtures detect unintended behavior drift
- **WHEN** golden fixture output changes
- **THEN** test failures require explicit fixture update and reviewer confirmation of intended behavior change

### Requirement: Smart Insertion Tuning Telemetry
The system SHALL emit privacy-safe, low-cardinality telemetry for smart insertion profile resolution and heuristic decisions.

#### Scenario: Decision telemetry excludes transcript content
- **WHEN** smart insertion emits telemetry
- **THEN** events include only reason codes and bounded categorical attributes
- **AND** transcript payload text is not included

#### Scenario: Fallback and profile trends are observable
- **WHEN** telemetry is analyzed over time
- **THEN** profile distribution and conservative fallback rates can be monitored for tuning priorities
