## ADDED Requirements

### Requirement: Ghost Text Overlay
The system SHALL display suggested text corrections as an overlay near the text cursor.

#### Scenario: Displaying a suggestion
- **WHEN** the AI returns a correction for the selected text
- **THEN** an overlay appears near the cursor showing the corrected text with a distinct visual style (e.g., greyed out or highlighted).

#### Scenario: Accepting a suggestion
- **WHEN** the overlay is visible and the user presses the `Tab` key
- **THEN** the suggested text replaces the original text in the focused application and the overlay disappears.

#### Scenario: Dismissing a suggestion
- **WHEN** the overlay is visible and the user presses the `Esc` key
- **THEN** the overlay disappears without modifying the text.

#### Scenario: Middle of word correction
- **WHEN** the cursor is placed in the middle of a word
- **THEN** the correction overlay appears **to the right** of the current word
- **AND** does not obscure the original word.

#### Scenario: No context available
- **WHEN** the user triggers correction but no text is found (empty field)
- **THEN** the overlay appears showing "No text to correct"
- **AND** automatically dismisses after 2 seconds.
