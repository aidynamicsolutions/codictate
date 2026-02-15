## ADDED Requirements
### Requirement: Strict Transcript Undo Shortcut
The system SHALL provide a dedicated global shortcut action (`undo_last_transcript`) that reverts only the most recent tracked Handy-originated transcript paste.

#### Scenario: Undo recent transcript paste
- **WHEN** the user triggers `undo_last_transcript` after Handy pasted transcript text
- **THEN** the focused application receives exactly one platform-standard undo command
- **AND** the tracked recent paste slot is marked consumed
- **AND** the app keeps recording/transcription state unchanged
- **AND** overlay feedback confirms undo action (`Undo applied`)

#### Scenario: Undo shortcut while recording is active
- **WHEN** the user triggers `undo_last_transcript` while recording is active
- **THEN** the system triggers the same cancellation path used by Escape
- **AND** no synthetic undo key event is sent unless a valid tracked paste slot exists
- **AND** user-visible feedback indicates recording was canceled when no valid tracked paste exists

#### Scenario: Undo shortcut while transcript processing is in-flight
- **WHEN** the user triggers `undo_last_transcript` while transcription/post-process/refine is still in-flight and no tracked paste slot exists yet
- **THEN** the in-flight processing operation is canceled
- **AND** no synthetic undo key event is sent unless a valid tracked paste slot exists
- **AND** user-visible feedback indicates processing was canceled when no valid tracked paste exists

#### Scenario: Undo shortcut while processing with valid tracked paste
- **WHEN** the user triggers `undo_last_transcript` while processing is in-flight
- **AND** a valid tracked recent paste slot exists
- **THEN** the in-flight processing operation is canceled first
- **AND** exactly one undo command is dispatched for the tracked paste slot

#### Scenario: Undo shortcut during stop-to-pipeline transition window
- **WHEN** the user triggers `undo_last_transcript` immediately after recording stop while pipeline start is still pending
- **THEN** the system treats this as cancelable transition state
- **AND** it triggers the same cancellation path used by Escape
- **AND** if no valid tracked paste slot exists after cancellation, feedback indicates processing was canceled

#### Scenario: Undo shortcut without tracked Handy paste
- **WHEN** the user triggers `undo_last_transcript` and no valid tracked recent Handy paste exists
- **THEN** the undo action is ignored
- **AND** no synthetic undo key event is sent
- **AND** user-visible overlay feedback indicates `Nothing to undo`

#### Scenario: Undo shortcut after slot expiry
- **WHEN** the user triggers `undo_last_transcript` after tracked paste TTL has expired
- **THEN** the undo action is ignored
- **AND** no synthetic undo key event is sent
- **AND** user-visible overlay feedback indicates `Undo expired`

#### Scenario: Undo shortcut used twice without new paste
- **WHEN** the user triggers `undo_last_transcript` twice after a single tracked paste
- **THEN** only the first trigger may dispatch an undo command
- **AND** the second trigger is ignored unless a new tracked paste occurs
- **AND** the second trigger returns no-op feedback (`Nothing to undo`)

#### Scenario: Target app ignores undo command
- **WHEN** the system dispatches an undo command but the focused application does not apply it
- **THEN** the system still treats the operation as best-effort command dispatch and consumes the tracked slot

#### Scenario: Editors that require multiple undo steps
- **WHEN** the focused editor represents a paste as multiple undo entries
- **THEN** each `undo_last_transcript` trigger still sends exactly one undo command

### Requirement: Undo Feedback Delivery Fallback
Undo feedback SHALL remain visible even when overlay UI is unavailable.

#### Scenario: Overlay unavailable with focused main window
- **WHEN** overlay UI is unavailable and undo feedback must be shown while main window is focused
- **THEN** feedback is shown through in-app toast UI

#### Scenario: Linux overlay unavailable with hidden or minimized app window
- **WHEN** overlay UI is unavailable on Linux and app window is hidden/minimized
- **THEN** feedback is shown through lightweight native notification

### Requirement: Recent Paste Tracking Model
The system SHALL maintain a single in-memory tracked recent paste slot for strict undo correlation.

#### Scenario: Track most recent Handy paste only
- **WHEN** Handy performs multiple paste operations
- **THEN** only the latest paste is stored in the tracked slot
- **AND** earlier paste context is overwritten

#### Scenario: Paste-last overwrite semantics
- **WHEN** a user pastes transcript A and then triggers `paste_last_transcript` which creates a newer tracked paste B
- **THEN** `undo_last_transcript` targets paste B
- **AND** tracked context for paste A is no longer available

#### Scenario: Recent paste expiry
- **WHEN** more than 120 seconds pass after tracked paste creation
- **THEN** the tracked paste slot is treated as invalid for undo
- **AND** triggering `undo_last_transcript` performs no undo dispatch

#### Scenario: App restart clears tracked paste slot
- **WHEN** the app restarts
- **THEN** tracked recent paste state is empty
- **AND** `undo_last_transcript` requires a new Handy paste before it can act

#### Scenario: Paste slot write ordering after successful paste
- **WHEN** Handy successfully pastes transcript text from async transcription flow
- **THEN** tracked paste slot is written in the same main-thread execution path immediately after paste success
- **AND** slot write happens before the paste callback path completes

### Requirement: Suggestion Text Capture Semantics
The system SHALL capture `suggestion_text` and `pasted_text` with deterministic source semantics for undo-driven heuristic evaluation.

#### Scenario: Auto-refine transcribe tracks raw suggestion text
- **WHEN** `transcribe` pastes refined output because `auto_refine_enabled` is active
- **THEN** tracked `source_action` remains `transcribe`
- **AND** tracked `auto_refined` is true
- **AND** `suggestion_text` stores the raw ASR transcript source used before refinement

#### Scenario: Transcribe-with-post-process tracks raw suggestion text
- **WHEN** `transcribe_with_post_process` pastes post-processed output
- **THEN** tracked `source_action` is `transcribe_with_post_process`
- **AND** tracked `auto_refined` is true
- **AND** `suggestion_text` stores the raw ASR transcript source used before post-processing

#### Scenario: Paste-last tracks raw heuristic source
- **WHEN** `paste_last_transcript` pastes post-processed history text
- **THEN** tracked `pasted_text` stores that post-processed paste payload
- **AND** tracked `suggestion_text` stores the corresponding raw history transcript source

#### Scenario: Refine-last tracks raw refine input
- **WHEN** `refine_last_transcript` pastes refined text
- **THEN** tracked `pasted_text` stores refined output
- **AND** tracked `suggestion_text` stores the raw transcript used as refine input

#### Scenario: Pasted text preserves transport modifiers
- **WHEN** paste transport adds trailing-space behavior
- **THEN** tracked `pasted_text` includes the exact modified payload sent to paste
- **AND** `suggestion_text` excludes transport-only modifiers

### Requirement: Clipboard Handling Compatibility
Undo shortcut behavior SHALL remain compatible with existing clipboard handling modes.

#### Scenario: Clipboard preserve mode
- **WHEN** clipboard handling mode preserves clipboard around paste operations
- **THEN** undo removes pasted text in the target app
- **AND** original clipboard contents remain preserved by existing paste behavior

#### Scenario: Clipboard overwrite mode
- **WHEN** clipboard handling mode overwrites clipboard during paste operations
- **THEN** undo removes pasted text in the target app
- **AND** undo flow does not attempt separate clipboard restoration beyond existing paste-mode behavior

### Requirement: Undo Shortcut Default Bindings
The system SHALL provide concrete default bindings for `undo_last_transcript` on each desktop platform.

#### Scenario: macOS default binding
- **WHEN** default shortcuts are initialized on macOS
- **THEN** `undo_last_transcript` default binding is `control+command+z`

#### Scenario: Windows default binding
- **WHEN** default shortcuts are initialized on Windows
- **THEN** `undo_last_transcript` default binding is `ctrl+alt+z`

#### Scenario: Linux default binding
- **WHEN** default shortcuts are initialized on Linux
- **THEN** `undo_last_transcript` default binding is `ctrl+alt+z`

#### Scenario: Default bindings pass reserved shortcut validation
- **WHEN** default bindings are validated during initialization/reset
- **THEN** `undo_last_transcript` defaults are accepted by reserved shortcut checks
- **AND** default registration does not fail due to reserved-key rejection

### Requirement: Undo Input Safety Constraints
Undo key dispatch SHALL follow existing input-safety constraints for Enigo-based key simulation.

#### Scenario: Undo dispatch uses main thread
- **WHEN** undo key simulation is dispatched
- **THEN** key dispatch runs on main thread path

#### Scenario: Undo dispatch applies modifier-release delay
- **WHEN** undo action is triggered by global shortcut
- **THEN** the system waits 200ms before synthetic undo key dispatch
- **AND** delay exists to avoid stale held modifier keys from the trigger shortcut

#### Scenario: macOS undo uses virtual keycode
- **WHEN** undo key simulation runs on macOS
- **THEN** the `Z` key path uses virtual keycode semantics rather than Unicode key lookup
- **AND** the implementation avoids layout-dependent non-main-thread crashes

#### Scenario: Windows undo key path
- **WHEN** undo key simulation runs on Windows
- **THEN** the `Z` key path uses Windows virtual keycode semantics

#### Scenario: Linux undo key path
- **WHEN** undo key simulation runs on Linux
- **THEN** the `Z` key path uses Linux-compatible key dispatch semantics

### Requirement: Undo Shortcut Settings Integration
The shortcuts settings UI SHALL expose `undo_last_transcript` as a configurable binding with platform defaults and reset support.

#### Scenario: Undo shortcut appears in settings
- **WHEN** the user opens keyboard shortcut settings
- **THEN** an entry for `undo_last_transcript` is visible with localized name and description
- **AND** conflicts are validated by existing shortcut conflict rules

#### Scenario: Undo shortcut placement in modal list
- **WHEN** the keyboard shortcuts modal renders its shortcut cards
- **THEN** `undo_last_transcript` appears immediately after `paste_last_transcript` in rendered order
- **AND** when `refine_last_transcript` is present, `undo_last_transcript` appears before it

#### Scenario: Full shortcut card ordering
- **WHEN** shortcut cards include `transcribe`, `transcribe_handsfree`, `paste_last_transcript`, `undo_last_transcript`, `refine_last_transcript`, and `correct_text`
- **THEN** they are rendered in exactly that order

#### Scenario: Reset shortcuts includes undo shortcut
- **WHEN** the user resets shortcuts to defaults
- **THEN** `undo_last_transcript` is restored to its platform default binding
- **AND** existing shortcuts keep their documented default behavior

### Requirement: Undo Shortcut Discoverability
The system SHALL provide a lightweight one-time-ever discoverability hint for `undo_last_transcript`.

#### Scenario: Delayed second-paste hint with unseen state
- **WHEN** a second successful tracked paste completes
- **AND** the user has not previously seen the undo hint
- **AND** the user has not used undo yet
- **AND** a short delay elapses after paste completion
- **THEN** the user sees a discoverability hint that includes the undo shortcut
- **AND** hint copy clarifies that undo is available within 2 minutes of paste
- **AND** the hint is delivered through overlay UI when available
- **AND** a persisted flag is set so the hint is not shown again

#### Scenario: Overlay unavailable during discoverability hint
- **WHEN** the one-time discoverability hint is due but overlay UI is unavailable
- **THEN** the hint is shown once via fallback channel based on platform/window visibility (focused toast, or Linux hidden-window native notification)
- **AND** the persisted flag is set only after display

### Requirement: Undo Shortcut Documentation
The project SHALL include a dedicated documentation page for undo-last-transcript behavior in the `doc/` folder.

#### Scenario: Documentation page covers core behavior
- **WHEN** the change is implemented
- **THEN** a document exists at `doc/undo-paste-last-transcript.md`
- **AND** it describes shortcut defaults, tracked-paste constraints (TTL/single-use), processing-cancel behavior, nudge suppression, Linux notification flow, clipboard interaction limits, and single-slot overwrite behavior when `paste_last_transcript` is used
