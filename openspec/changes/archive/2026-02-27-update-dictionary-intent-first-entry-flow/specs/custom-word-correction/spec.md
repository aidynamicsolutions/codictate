## ADDED Requirements
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

## MODIFIED Requirements
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
