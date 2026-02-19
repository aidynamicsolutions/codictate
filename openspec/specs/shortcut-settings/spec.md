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

### Requirement: Strict Transcript Undo Shortcut
The system SHALL provide a dedicated global shortcut action (`undo_last_transcript`) that reverts only the most recent tracked Codictate-originated transcript paste.

#### Scenario: Undo recent transcript paste
- **WHEN** user triggers `undo_last_transcript` after Codictate pasted transcript text
- **THEN** focused application receives exactly one platform-standard undo command
- **AND** tracked slot is consumed
- **AND** overlay feedback shows `Undo applied`

#### Scenario: Undo while operation is active
- **WHEN** user triggers `undo_last_transcript` while recording/transcription/refine/post-process is active
- **THEN** system triggers cancellation path used by Escape
- **AND** no synthetic undo key event is sent for that keypress
- **AND** overlay remains on cancelling presentation

#### Scenario: Undo during stop transition window
- **WHEN** user triggers `undo_last_transcript` immediately after recording stop while pipeline start is pending
- **THEN** system treats this as cancelable transition state
- **AND** triggers cancellation path instead of undo dispatch

#### Scenario: No tracked paste
- **WHEN** user triggers `undo_last_transcript` and no valid tracked paste exists
- **THEN** no synthetic undo key event is sent
- **AND** feedback indicates `Nothing to undo`

#### Scenario: Expired tracked paste
- **WHEN** user triggers `undo_last_transcript` after tracked paste TTL expiry
- **THEN** no synthetic undo key event is sent
- **AND** feedback indicates `Undo expired`
- **AND** expired slot is cleared

#### Scenario: Repeated press after expiry
- **WHEN** user presses undo again without a new tracked paste after expiry feedback
- **THEN** feedback indicates `Nothing to undo`

### Requirement: Undo Feedback and Discoverability Lane
Undo feedback and discoverability hint SHALL be rendered through the shared overlay message lane.

#### Scenario: Feedback message lane
- **WHEN** undo feedback (`Undo applied`, `Nothing to undo`, `Undo expired`) is shown
- **THEN** it is rendered via shared overlay message lane with non-loading static style

#### Scenario: Discoverability message lane
- **WHEN** discoverability hint is shown
- **THEN** it is rendered via shared overlay message lane and auto-dismisses

#### Scenario: Overflow marquee behavior
- **WHEN** overlay message text exceeds lane width
- **THEN** marquee scrolling is enabled
- **AND** short messages remain centered and static

### Requirement: Recent Paste Tracking Model
The system SHALL maintain a single in-memory tracked recent paste slot for strict undo correlation.

#### Scenario: Track most recent paste only
- **WHEN** Codictate performs multiple tracked paste operations
- **THEN** only latest paste is stored
- **AND** older tracked context is overwritten

#### Scenario: Recent paste expiry
- **WHEN** more than 120 seconds pass after tracked paste creation
- **THEN** tracked slot is invalid for undo dispatch

#### Scenario: App restart clears tracked state
- **WHEN** app restarts
- **THEN** tracked recent-paste state is empty

### Requirement: Undo Stats Rollback Semantics
The system SHALL reverse cumulative stats contribution for undone transcribe-origin transcript pastes while keeping history entries intact.

#### Scenario: Rollback transcribe-origin contribution
- **WHEN** valid undo dispatch targets source `transcribe` or `transcribe_with_post_process`
- **THEN** corresponding stats contribution is reversed from `user_stats`

#### Scenario: Non-transcribe source does not rollback stats
- **WHEN** valid undo dispatch targets source `paste_last_transcript` or `refine_last_transcript`
- **THEN** no stats rollback is applied

### Requirement: Undo Shortcut Default Bindings
The system SHALL provide platform defaults for `undo_last_transcript`.

#### Scenario: macOS default
- **WHEN** defaults initialize on macOS
- **THEN** binding is `control+command+z`

#### Scenario: Windows default
- **WHEN** defaults initialize on Windows
- **THEN** binding is `ctrl+alt+z`

#### Scenario: Linux default
- **WHEN** defaults initialize on Linux
- **THEN** binding is `ctrl+alt+z`

