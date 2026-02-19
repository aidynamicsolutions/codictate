## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using language-profile-aware punctuation normalization, conservative fallbacks, and CJK internal-space compaction for target languages when smart insertion is enabled.

#### Scenario: Chinese and Cantonese compact internal CJK spaces
- **WHEN** the normalized selected language is `zh`, `zh-tw`, or `yue`
- **AND** transcript text contains compactable internal whitespace between CJK Han/punctuation boundaries
- **THEN** internal whitespace is removed before paste

#### Scenario: Mixed-script ASCII phrase spacing is preserved
- **WHEN** the normalized selected language is `zh`, `zh-tw`, or `yue`
- **AND** transcript text contains intentional ASCII phrase spacing (for example `Open AI`)
- **THEN** ASCII phrase spacing is preserved

#### Scenario: CJK internal compaction keeps structural formatting
- **WHEN** compactable CJK internal spacing is evaluated
- **THEN** line breaks and tab-separated formatting are preserved

#### Scenario: Non-target language behavior is unchanged
- **WHEN** the normalized selected language is outside `zh`, `zh-tw`, and `yue`
- **THEN** CJK internal-space compaction does not run
- **AND** existing profile behavior remains unchanged

### Requirement: Smart Boundary Spacing
The system SHALL apply boundary spacing only for profiles that enable word-boundary spacing.

#### Scenario: Chinese sentence boundaries do not require inserted spaces
- **WHEN** insertion runs under `NoBoundarySpacing` profile for Chinese/Cantonese
- **THEN** no boundary spacing is injected around sentence punctuation boundaries
- **AND** final sentence separation relies on punctuation, not inserted spaces
