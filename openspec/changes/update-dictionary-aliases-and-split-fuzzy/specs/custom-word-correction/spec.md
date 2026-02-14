## ADDED Requirements
### Requirement: Canonical Alias Support
The system SHALL support dictionary entries with one canonical input and multiple spoken aliases.

#### Scenario: Exact alias match
- **WHEN** a dictionary entry has canonical input `shadcn` and alias `shad cn`
- **AND** transcription contains `shad cn`
- **THEN** the system applies the entry replacement output
- **AND** the exact alias path is used before fuzzy paths

#### Scenario: Alias match with punctuation
- **WHEN** a dictionary entry has alias `shad c n`
- **AND** transcription contains `shad c n?`
- **THEN** the system matches the alias
- **AND** preserves trailing punctuation in the output

### Requirement: Split-Token Fuzzy for Single-Token Targets
The system SHALL support constrained split-token fuzzy matching for 2-3 word n-grams mapped to single-token dictionary terms.

#### Scenario: Split-token fuzzy success
- **WHEN** the dictionary contains canonical input `shadcn`
- **AND** transcription contains `Shat CN component`
- **THEN** the system maps the split term to `shadcn`
- **AND** keeps unrelated surrounding words unchanged

#### Scenario: Split-token fuzzy reject for dissimilar words
- **WHEN** the dictionary contains canonical input `shadcn`
- **AND** transcription contains `Chef CN component`
- **THEN** the system does not replace `Chef CN`
- **AND** logs a score-based rejection reason

### Requirement: Split-Path Threshold Control
The system SHALL expose a dedicated split-token fuzzy threshold setting separate from the general word correction threshold.

#### Scenario: Split threshold blocks weak candidate
- **WHEN** split threshold is stricter than general threshold
- **AND** a split-token candidate score exceeds split threshold
- **THEN** the candidate is rejected even if it would pass the general threshold

### Requirement: No Legacy Dictionary Compatibility
The system SHALL not support legacy dictionary entry schema that omits alias fields.

#### Scenario: Legacy dictionary entry shape is loaded
- **WHEN** persisted settings contain dictionary entries without required alias field
- **THEN** settings parsing fails
- **AND** existing fallback behavior resets to default settings

### Requirement: User Guidance
The system SHALL provide user-facing documentation for effective dictionary configuration with canonical terms, aliases, and debugging guidance.

#### Scenario: Hard term setup example
- **WHEN** a user reads dictionary documentation
- **THEN** the guide includes a worked `shadcn` example with aliases such as `shad cn`
- **AND** explains expected outcomes for exact alias and split-token cases

#### Scenario: Mixed use-case examples
- **WHEN** a user reads dictionary documentation
- **THEN** the guide includes examples for `chatgpt` and abbreviation replacement (for example `btw -> by the way`)
- **AND** includes a session-filtered log debugging workflow

## MODIFIED Requirements
### Requirement: Threshold-Based Acceptance
Matches MUST only be accepted if their score is below the threshold for the selected matching path.

#### Scenario: Standard fuzzy score below threshold
- **GIVEN** standard threshold is `0.18`
- **WHEN** a standard fuzzy candidate score is `0.15`
- **THEN** correction is applied

#### Scenario: Split fuzzy score below split threshold
- **GIVEN** split threshold is `0.14`
- **WHEN** a split-token candidate score is `0.13`
- **THEN** correction is applied

#### Scenario: Split fuzzy score above split threshold
- **GIVEN** split threshold is `0.14`
- **WHEN** a split-token candidate score is `0.16`
- **THEN** correction is not applied

### Requirement: Exact Match Priority
Custom word matching MUST check exact canonical and exact alias matches before fuzzy matching.

#### Scenario: Canonical exact match is found
- **GIVEN** the user has canonical input `chatgpt`
- **WHEN** transcription contains `Chat GPT`
- **THEN** exact normalized matching is selected before fuzzy scoring
