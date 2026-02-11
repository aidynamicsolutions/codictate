## ADDED Requirements

### Requirement: Context Capture via Accessibility
The system SHALL capture text context from the currently focused application using macOS Accessibility APIs.

#### Scenario: User triggers correction in TextEdit
- **WHEN** the user triggers the correction shortcut while focused on a text field in TextEdit
- **THEN** the system retrieves the selected text range and up to 500 characters of surrounding text.

#### Scenario: Fallback to Clipboard
- **WHEN** the Accessibility API fails to retrieve text (e.g., in an incompatible app)
- **THEN** the system MAY attempt to simulate a copy command (Cmd+C) to retrieve the selected text from the clipboard.

### Requirement: Permission Handling
The system MUST handle "Accessibility" permission states gracefully.

#### Scenario: Permission not granted
- **WHEN** the user attempts to use the feature without granting Accessibility permissions
- **THEN** the system shows a dialog explaining the need for permissions and offering to open System Settings.
