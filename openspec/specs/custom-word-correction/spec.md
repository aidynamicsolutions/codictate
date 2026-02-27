# custom-word-correction Specification

## Purpose
TBD - created by archiving change improve-custom-word-algorithm. Update Purpose after archive.
## Requirements
### Requirement: Exact Match Priority
Custom word matching MUST check exact canonical and exact alias matches before fuzzy matching.

#### Scenario: Canonical exact match is found
- **GIVEN** the user has canonical input `chatgpt`
- **WHEN** transcription contains `Chat GPT`
- **THEN** exact normalized matching is selected before fuzzy scoring

### Requirement: Phonetic Matching with Double Metaphone

Custom word matching MUST use Double Metaphone algorithm for phonetic similarity detection.

#### Scenario: Primary phonetic code match

- **Given** the user has added "Smith" to custom words
- **When** the transcription contains "Smyth"
- **Then** the words match phonetically via primary code comparison
- **And** the match is weighted appropriately

#### Scenario: Secondary phonetic code match

- **Given** the user has added "Schmidt" to custom words
- **When** the transcription contains "Smith"
- **Then** the words may match via secondary code comparison
- **And** the match is weighted appropriately

#### Scenario: Non-English pronunciation handling

- **Given** the user has added "Nguyen" to custom words
- **When** the transcription contains a phonetically similar variant
- **Then** Double Metaphone handles the non-English pronunciation
- **And** matching is more accurate than Soundex would provide

### Requirement: Transposition-Aware Edit Distance

Custom word matching MUST treat character transpositions as a single edit operation.

#### Scenario: Adjacent character swap

- **Given** the user has added "the" to custom words
- **When** the transcription contains "teh"
- **Then** the edit distance is calculated as 1 (not 2)
- **And** matching sensitivity is improved for common typos

#### Scenario: Multiple transpositions

- **Given** the user has added "receive" to custom words
- **When** the transcription contains "recieve" (common misspelling)
- **Then** the transposition "ie" → "ei" counts as 1 edit
- **And** the word matches more reliably

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

### Requirement: Case Pattern Preservation

Corrections MUST preserve the case pattern of the original word.

#### Scenario: All uppercase original

- **Given** the custom word is "hello"
- **When** the original word is "HELO"
- **Then** the correction is "HELLO"

#### Scenario: Title case original

- **Given** the custom word is "world"
- **When** the original word is "Wrold"
- **Then** the correction is "World"

### Requirement: Punctuation Preservation

Corrections MUST preserve punctuation before and after the word.

#### Scenario: Trailing punctuation

- **Given** the custom word is "hello"
- **When** the original word is "helo?"
- **Then** the correction is "hello?"

#### Scenario: Surrounding punctuation

- **Given** the custom word is "hello"
- **When** the original word is "...helo!"
- **Then** the correction is "...hello!"

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

### Requirement: User Guidance
The system SHALL provide user-facing guidance that matches the intent-first flow and precision-first matching policy.

#### Scenario: Intent-first help content
- **WHEN** users open Dictionary help or guide docs
- **THEN** guidance explains when to use recognize vs replace intent
- **AND** explains that aliases are the primary strategy before fuzzy

#### Scenario: Conditional fuzzy guidance remains constrained
- **WHEN** users view fuzzy guidance from the optional fuzzy toggle row
- **THEN** guidance states fuzzy is optional and intended for uncommon harder terms
- **AND** guidance states short single-word targets remain exact-only

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

### Requirement: Intent-First Dictionary Entry Flow
The Dictionary modal SHALL require users to choose between recognition and replacement intent before configuring optional fuzzy behavior.

#### Scenario: User creates recognize entry
- **WHEN** a user opens the add-entry modal
- **THEN** the default intent is `Recognize this term`
- **AND** the primary field is `Word or phrase`

#### Scenario: User creates replacement entry
- **WHEN** a user switches intent to `Replace spoken phrase`
- **THEN** the modal shows `What you say` and `Output text`
- **AND** fuzzy matching controls are hidden

#### Scenario: Recognize entry hides fuzzy until eligible input
- **WHEN** a user is in recognize intent and input is empty or short single-word ineligible
- **THEN** fuzzy matching controls are hidden
- **AND** exact input + aliases remain the default path

### Requirement: Aliases Visible by Default
The Dictionary modal SHALL display alias controls by default for both recognize and replace intents.

#### Scenario: Recognize intent alias visibility
- **WHEN** recognize intent is active
- **THEN** aliases are visible without opening an extra panel

#### Scenario: Replace intent alias visibility
- **WHEN** replace intent is active
- **THEN** aliases are visible with replacement-focused labeling

### Requirement: Replace Intent Output Validation
Replacement entries SHALL require output text that is non-empty and different from spoken input.

#### Scenario: Invalid replacement output
- **WHEN** replace intent output is empty or equals spoken input
- **THEN** save is blocked
- **AND** the UI shows an inline validation message

### Requirement: Dictionary and Snippets Boundary
Dictionary SHALL keep correction/recognition terminology and defer reusable long-template expansion to Snippets.

#### Scenario: Dictionary replacement wording
- **WHEN** users configure replacement inside Dictionary
- **THEN** UI uses `Replace spoken phrase` terminology
- **AND** does not rename the mode to `Snippet`

