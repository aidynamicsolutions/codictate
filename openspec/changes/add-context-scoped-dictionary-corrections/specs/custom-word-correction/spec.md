## ADDED Requirements

### Requirement: Context-Scoped Exact Matching
The system SHALL support optional context-scoped exact matching for dictionary entries.

#### Scenario: Scoped exact match succeeds on next-token constraint
- **GIVEN** a dictionary entry maps `state` to `staged`
- **AND** the entry defines `context_scope.next_any = ["changes"]`
- **WHEN** transcription contains `review the state changes`
- **THEN** the system applies `staged` for `state`

#### Scenario: Scoped exact match is rejected outside scope
- **GIVEN** a dictionary entry maps `state` to `staged`
- **AND** the entry defines `context_scope.next_any = ["changes"]`
- **WHEN** transcription contains `I want the state change`
- **THEN** the system does not apply the replacement for `state`

### Requirement: Dictionary Entry Scope Metadata
The dictionary entry schema SHALL allow optional context scope fields and optional ambiguity metadata.

#### Scenario: Entry includes optional scope and ambiguity metadata
- **WHEN** a dictionary entry is saved with `context_scope` and `ambiguity_level`
- **THEN** the entry persists both fields
- **AND** runtime matching can evaluate scope constraints

#### Scenario: Legacy entry omits scope and metadata
- **WHEN** a dictionary entry has no `context_scope` or `ambiguity_level`
- **THEN** the system preserves existing matching behavior unchanged

### Requirement: Scoped Safety Guidance
The system SHALL provide user guidance for ambiguous single-word aliases.

#### Scenario: Ambiguous alias warning
- **WHEN** users create or edit an entry that appears ambiguous in global form
- **THEN** the UI guidance recommends phrase replacement or scoped context
- **AND** does not claim guaranteed pronunciation biasing

### Requirement: Guardrail Preservation
Context-scoped exact matching SHALL NOT weaken existing fuzzy safety guardrails.

#### Scenario: Existing short/common-term safety behavior remains unchanged
- **GIVEN** existing guard coverage for risky corrections such as `mode -> modal`
- **WHEN** context-scoped exact matching is introduced
- **THEN** existing fuzzy guardrails still prevent unintended overfire behavior
- **AND** scoped matching adds no bypass for those fuzzy protections

## MODIFIED Requirements

### Requirement: Exact Match Priority
Custom word matching MUST check exact canonical and exact alias matches before fuzzy matching, with scoped exact matches prioritized over unscoped exact matches when both are eligible.

#### Scenario: Canonical exact match is found
- **GIVEN** the user has canonical input `chatgpt`
- **WHEN** transcription contains `Chat GPT`
- **THEN** exact normalized matching is selected before fuzzy scoring

#### Scenario: Scoped exact takes precedence over unscoped exact
- **GIVEN** two eligible exact entries for the same token, one scoped and one unscoped
- **WHEN** the scoped entry's context constraint passes
- **THEN** the system applies the scoped exact entry
- **AND** the unscoped exact entry is not applied for that token

#### Scenario: Exact phrase replacement takes precedence over generic exact alias
- **GIVEN** a phrase replacement rule `state changes -> staged changes`
- **AND** a generic exact alias rule also maps `state -> staged`
- **WHEN** transcription contains `review the state changes`
- **THEN** the phrase replacement is applied first
- **AND** lower-precedence exact alias matching does not override phrase output

### Requirement: User Guidance
The system SHALL provide user-facing guidance that matches the intent-first flow and precision-first matching policy, and MUST clarify that Dictionary is a post-recognition correction layer rather than guaranteed decoder biasing.

#### Scenario: Intent-first help content
- **WHEN** users open Dictionary help or guide docs
- **THEN** guidance explains when to use recognize vs replace intent
- **AND** explains that aliases are the primary strategy before fuzzy

#### Scenario: Ambiguous alias guidance
- **WHEN** users read guidance for alias usage
- **THEN** guidance recommends avoiding global aliases for common ambiguous words
- **AND** guidance recommends phrase replacement or scoped matching for stable contexts
