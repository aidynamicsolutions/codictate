## MODIFIED Requirements

### Requirement: Shared Paste Flow Coverage
The system SHALL apply smart insertion formatting to transcript paste flows that use adaptive preparation, while supporting literal replay for `paste_last_transcript` by default, and SHALL register the exact pasted payload for undo.

#### Scenario: Live transcription paste remains adaptive
- **WHEN** `transcribe` flow pastes transcript output
- **THEN** shared smart insertion formatter is used

#### Scenario: Paste-last-transcript default is literal replay
- **WHEN** `paste_last_transcript` is triggered
- **AND** `paste_last_use_smart_insertion` is disabled or missing
- **THEN** shared paste utility uses literal preparation for paste-last
- **AND** pasted text equals History primary text (`effective_text`) exactly

#### Scenario: Paste-last-transcript adaptive mode is opt-in
- **WHEN** `paste_last_transcript` is triggered
- **AND** `paste_last_use_smart_insertion` is enabled
- **THEN** shared smart insertion formatter is used for paste-last

#### Scenario: Refine-last-transcript remains adaptive
- **WHEN** `refine_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Undo capture stores actual pasted text
- **WHEN** a transcript paste succeeds through the shared paste utility
- **THEN** the registered undo payload uses the `pasted_text` returned by that paste operation
- **AND** triggering configured `undo_last_transcript` reverses that exact paste as a single operation

## ADDED Requirements

### Requirement: Paste-Last Formatting Mode Setting
The system SHALL provide a user setting to control whether `paste_last_transcript` uses literal replay or adaptive smart insertion formatting.

#### Scenario: New and existing users default to literal mode
- **WHEN** settings are initialized or loaded from older versions where this setting does not exist
- **THEN** `paste_last_use_smart_insertion` defaults to `false`

#### Scenario: User enables adaptive mode
- **WHEN** user enables the paste-last smart insertion setting
- **THEN** subsequent `paste_last_transcript` actions use adaptive smart insertion preparation
