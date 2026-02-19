## MODIFIED Requirements
### Requirement: Punctuation Preservation
Corrections MUST preserve punctuation before and after the word while avoiding duplicated terminal sentence punctuation when replacement text already ends with the same mark.

#### Scenario: Trailing punctuation remains preserved
- **Given** the custom word is "hello"
- **When** the original word is "helo?"
- **Then** the correction is "hello?"

#### Scenario: Duplicate terminal sentence punctuation is deduplicated on merge
- **Given** the replacement text is "e.g."
- **And** a matched alias contributes trailing suffix punctuation "."
- **When** custom word correction merges replacement with preserved suffix punctuation
- **Then** output keeps only one terminal "."
- **And** the merged token is "e.g."

#### Scenario: Non-duplicate suffix punctuation remains preserved
- **Given** the replacement text is "e.g."
- **And** a matched alias contributes trailing suffix punctuation "?"
- **When** custom word correction merges replacement with preserved suffix punctuation
- **Then** output remains "e.g.?"
