# shortcut-settings Specification

## Purpose
TBD - created by archiving change add-fn-key-conflict-detection. Update Purpose after archive.
## Requirements
### Requirement: Fn Key Conflict Detection
The system SHALL provide a mechanism to detect when another application is intercepting Fn key events before Codictate can receive them.

#### Scenario: User tests Fn key detection successfully
- **WHEN** user clicks "Test Fn Key" in Settings and presses Fn during the 3-second test window
- **THEN** the system SHALL display a success message indicating Fn key events are being received

#### Scenario: User tests Fn key detection with conflict
- **WHEN** user clicks "Test Fn Key" and another app is intercepting Fn key events
- **THEN** the system SHALL display a failure message and show troubleshooting steps

---

### Requirement: Fn Key Troubleshooting Guidance
The system SHALL provide clear troubleshooting steps when Fn key conflict is detected.

#### Scenario: Troubleshooting steps displayed
- **WHEN** Fn key test fails
- **THEN** the system SHALL display the following resolution steps:
  1. Open System Settings → Keyboard
  2. Set "Press fn key to" to "Do Nothing"
  3. Close other transcription apps that might use Fn
  4. Or choose a different shortcut

---

### Requirement: Test Fn Key UI in Settings
The Settings screen under Shortcuts section SHALL include a "Test Fn Key" button for macOS users.

#### Scenario: Test button visible on macOS
- **WHEN** user views Shortcuts settings on macOS
- **THEN** a "Test Fn Key" button SHALL be visible

#### Scenario: Test button hidden on non-macOS
- **WHEN** user views Shortcuts settings on Windows/Linux
- **THEN** the "Test Fn Key" button SHALL NOT be visible

---

### Requirement: Test Mode Backend API
The backend SHALL provide commands to start/stop test mode and retrieve event counts.

#### Scenario: Start test mode
- **WHEN** `start_fn_key_test` command is called
- **THEN** the event counter is reset to 0 and test mode is enabled

#### Scenario: Get test result
- **WHEN** `get_fn_key_test_result` command is called
- **THEN** the current event count is returned and test mode is disabled

