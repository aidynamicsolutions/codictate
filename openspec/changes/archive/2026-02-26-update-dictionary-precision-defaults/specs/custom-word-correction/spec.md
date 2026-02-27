## ADDED Requirements
### Requirement: Exact + Alias Default Matching Mode
The system SHALL default dictionary entries to exact canonical and exact alias matching, with fuzzy matching disabled unless explicitly enabled for the entry.

#### Scenario: New vocabulary entry defaults to exact behavior
- **WHEN** a user creates a new non-replacement dictionary entry without enabling fuzzy matching
- **THEN** exact canonical and exact alias paths are evaluated
- **AND** fuzzy paths are not evaluated for that entry

### Requirement: Explicit Fuzzy Opt-In
The system SHALL evaluate fuzzy matching only when a dictionary entry explicitly enables fuzzy matching.

#### Scenario: Fuzzy disabled entry skips fuzzy path
- **GIVEN** an entry with `fuzzy_enabled = false`
- **WHEN** transcription contains a near-miss token for that entry
- **THEN** the system does not evaluate fuzzy scoring for that entry
- **AND** no fuzzy replacement is applied

#### Scenario: Fuzzy enabled entry allows fuzzy path
- **GIVEN** an entry with `fuzzy_enabled = true`
- **WHEN** transcription contains a fuzzy-eligible near-miss token
- **THEN** the system evaluates fuzzy scoring using existing thresholds and guards

### Requirement: Single-Word Short Target Fuzzy Block
The system SHALL block single-word fuzzy matching when the normalized dictionary target character length is less than or equal to 4 characters.

#### Scenario: Hard short-target block prevents false positive
- **GIVEN** an entry with normalized target character length `4`
- **AND** `fuzzy_enabled = true`
- **WHEN** transcription contains a near-miss single-word candidate
- **THEN** the candidate is rejected before fuzzy score acceptance
- **AND** no fuzzy replacement is applied

### Requirement: Legacy Dictionary Migration Compatibility
The system SHALL support migration of legacy dictionary entries that do not define `fuzzy_enabled`.

#### Scenario: Legacy non-short vocabulary entry retains fuzzy capability
- **GIVEN** a legacy non-replacement entry that is not a single-word canonical target with normalized character length `<= 4`
- **AND** `fuzzy_enabled` is unset
- **WHEN** settings are loaded
- **THEN** `fuzzy_enabled` is migrated to `true`

#### Scenario: Legacy short vocabulary entry is hardened to exact
- **GIVEN** a legacy non-replacement entry with single-word normalized canonical character length `<= 4`
- **AND** `fuzzy_enabled` is unset
- **WHEN** settings are loaded
- **THEN** `fuzzy_enabled` is migrated to `false`

#### Scenario: Replacement entries remain exact-only
- **GIVEN** a replacement entry (`is_replacement = true`)
- **WHEN** settings are loaded
- **THEN** `fuzzy_enabled` is enforced as `false`

## MODIFIED Requirements
### Requirement: Threshold-Based Acceptance
Matches MUST only be accepted if their score is below the threshold for the selected matching path, after opt-in and safety guards are satisfied.

#### Scenario: Standard fuzzy score below threshold
- **GIVEN** standard threshold is `0.18`
- **AND** the entry has `fuzzy_enabled = true`
- **AND** the entry passes short-target safety guards
- **WHEN** a standard fuzzy candidate score is `0.15`
- **THEN** correction is applied

#### Scenario: Standard fuzzy candidate blocked by short-target guard
- **GIVEN** an entry with normalized target character length `4`
- **AND** the entry has `fuzzy_enabled = true`
- **WHEN** a standard fuzzy candidate score would otherwise pass threshold
- **THEN** correction is not applied because the short-target guard takes precedence

## REMOVED Requirements
### Requirement: No Legacy Dictionary Compatibility
**Reason**: This change introduces deterministic migration support for legacy entries to preserve behavior for long terms while hardening short terms.
**Migration**: Legacy entries with missing `fuzzy_enabled` are migrated at settings load time using single-word short-target and replacement-mode rules.
