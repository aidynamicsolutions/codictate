## MODIFIED Requirements
### Requirement: Smart Transcript Insertion Formatting
The system SHALL format transcript paste text using language-profile-aware punctuation normalization, conservative fallbacks, and internal-space compaction for target languages when smart insertion is enabled.

#### Scenario: Chinese and Cantonese compact internal CJK spaces
- **WHEN** the normalized selected language is `zh`, `zh-tw`, or `yue`
- **AND** transcript text contains compactable internal whitespace between CJK Han/punctuation boundaries
- **THEN** internal whitespace is removed before paste

#### Scenario: Japanese compacts mixed-script internal spaces
- **WHEN** the normalized selected language is `ja`
- **AND** transcript text contains compactable internal whitespace at `Japanese↔Japanese` or `ASCII↔Japanese` boundaries
- **THEN** internal whitespace is removed before paste

#### Scenario: Mixed-script ASCII phrase spacing is preserved
- **WHEN** internal-space compaction is evaluated for supported compaction languages
- **AND** transcript text contains intentional `ASCII↔ASCII` phrase spacing (for example `Open AI`)
- **THEN** `ASCII↔ASCII` spacing is preserved

#### Scenario: Internal compaction keeps structural formatting
- **WHEN** compactable internal spacing is evaluated
- **THEN** line breaks and tab-separated formatting are preserved

#### Scenario: Non-target language behavior is unchanged
- **WHEN** the normalized selected language is outside the internal-space compaction targets
- **THEN** internal-space compaction does not run
- **AND** existing profile behavior remains unchanged

### Requirement: Smart Boundary Spacing
The system SHALL apply boundary spacing only for profiles that enable word-boundary spacing.

#### Scenario: No-boundary-spacing profile behavior remains unchanged
- **WHEN** insertion runs under `NoBoundarySpacing`
- **THEN** no boundary spacing is injected around sentence punctuation or word boundaries
