## ADDED Requirements

### Requirement: Active Window Context Capture
The system SHALL capture active application context when multimodal capture is enabled and a dictation session begins.

#### Scenario: Native accessibility text selection capture
- **WHEN** the user triggers dictation in a standard macOS application
- **AND** multimodal capture is enabled
- **AND** the active element exposes the `AXSelectedText` accessibility attribute
- **THEN** the system captures selected text without modifying the clipboard

#### Scenario: Browser context fallback
- **WHEN** multimodal capture is enabled
- **AND** native text selection capture fails in a supported browser
- **THEN** the system attempts best-effort capture of app name, window title, URL, and selected text via AppleScript/browser JavaScript
- **AND** if that attempt fails, the system falls back to the clipboard-copy method

#### Scenario: Multimodal capture disabled
- **WHEN** the user disables multimodal capture
- **THEN** the system bypasses active-context capture entirely
- **AND** dictation continues as voice-only

### Requirement: App-Owned Screenshot Capture
The system SHALL capture multimodal screenshots through an app-owned action rather than by observing arbitrary macOS screenshot output.

#### Scenario: Dedicated screenshot binding during active hands-free dictation
- **WHEN** the user triggers `capture_dictation_screenshot`
- **AND** the binding is configured or defaults to `control+command+s` on macOS
- **AND** an active hands-free dictation session is running
- **THEN** the system launches `/usr/sbin/screencapture -i -o -x` for that session

#### Scenario: Screenshot action outside active hands-free dictation
- **WHEN** the user triggers `capture_dictation_screenshot`
- **AND** there is no active hands-free dictation session
- **THEN** the system does not create a figure attachment

#### Scenario: Successful figure capture
- **WHEN** the screenshot command exits successfully
- **AND** the target output file exists
- **THEN** the system records a new figure for the active session with capture order and capture time

#### Scenario: Capture timestamp tracks user intent rather than file creation
- **WHEN** the screenshot binding is accepted for an active hands-free dictation session
- **THEN** the system records the figure timestamp from the accepted capture action time
- **AND** does not use the delayed file creation time as the alignment anchor

#### Scenario: Region capture on a non-primary display
- **WHEN** the interactive screenshot UI is active
- **AND** the user moves the pointer to another attached display in a multi-monitor setup
- **THEN** the system continues to allow region capture on that display
- **AND** a successful capture from that display is stored like any other figure

#### Scenario: Window capture on a non-primary display
- **WHEN** the interactive screenshot UI is active
- **AND** the user switches to window-selection mode and selects a window on another attached display
- **THEN** the system stores that successful capture like any other figure

#### Scenario: Additional screenshot triggers are serialized
- **WHEN** an interactive screenshot capture is already in progress for the active session
- **AND** the user triggers `capture_dictation_screenshot` again
- **THEN** the system does not launch a second screenshot process
- **AND** the original interactive capture remains the only in-flight screenshot action

#### Scenario: Overlay is hidden during interactive capture
- **WHEN** an interactive screenshot capture is in progress
- **THEN** transient Codictate capture UI such as the recording overlay is hidden until the screenshot resolves
- **AND** the UI is restored after the screenshot succeeds, is cancelled, or fails

#### Scenario: Screen Recording permission is denied
- **WHEN** the screenshot capture cannot complete because macOS Screen Recording permission is denied
- **THEN** the system records no new figure
- **AND** dictation continues without error
- **AND** the UI surfaces localized guidance to grant the required permission

#### Scenario: Interactive capture is redirected to clipboard
- **WHEN** the interactive screenshot flow is redirected to the clipboard instead of creating a file
- **THEN** the system records no new figure
- **AND** dictation continues without error
- **AND** user guidance documents that V1 supports file-backed screenshots only

#### Scenario: Dictation stop requested while a screenshot is in progress
- **WHEN** the user requests dictation stop while an interactive screenshot capture is still in progress
- **THEN** final transcription dispatch waits for that in-flight screenshot process to resolve
- **AND** a successful resulting file is attached to the session
- **AND** a cancelled or missing-file outcome produces no figure

#### Scenario: Cancelled or missing screenshot output
- **WHEN** the screenshot command is cancelled
- **OR** the command exits without a created file
- **THEN** the system records no new figure
- **AND** dictation continues without error

### Requirement: Multimodal Sidecar Persistence
The system SHALL persist multimodal metadata separately from spoken transcript text.

#### Scenario: History stores spoken text and sidecar separately
- **WHEN** a multimodal dictation session is saved to history
- **THEN** the spoken transcript remains stored in `transcription_text`
- **AND** any refined spoken transcript remains stored in `post_processed_text`
- **AND** the exact rendered multimodal payload remains stored in `inserted_text`
- **AND** the app stores a nullable serialized `MultimodalSidecar` alongside the history row

#### Scenario: Screenshot promotion into stable history storage
- **WHEN** a multimodal session is committed to history
- **THEN** successful session screenshots are promoted into app-managed history storage
- **AND** the persisted sidecar references the promoted absolute file paths

#### Scenario: History cleanup removes figure files
- **WHEN** a history entry with persisted figures is deleted or pruned by retention
- **THEN** the system deletes the associated stored screenshot files

### Requirement: Multimodal Payload Rendering
The system SHALL render a stable text payload from spoken text and multimodal sidecar metadata.

#### Scenario: Context block formatting
- **WHEN** at least one context field is non-empty
- **THEN** the payload begins with a `Context:` heading
- **AND** emits non-empty fields only in this order: `App`, `Window`, `URL`, `Selection`

#### Scenario: Context block omission
- **WHEN** all context fields are empty
- **THEN** the payload omits the entire `Context:` block

#### Scenario: Inline figure placeholder placement
- **WHEN** a session contains captured figures
- **THEN** the renderer inserts inline placeholders exactly as `[Figure N]`
- **AND** aligns them to sentence boundaries on a best-effort basis while preserving figure order

#### Scenario: Figure footer formatting
- **WHEN** a session contains captured figures
- **THEN** the payload appends footer lines formatted exactly as `Figure N: <absolute path>`
- **AND** orders footer lines by figure number

#### Scenario: Reinsert figures after refined text changes
- **WHEN** refined or post-processed spoken text no longer preserves the original sentence boundary exactly
- **THEN** the renderer places remaining figures at the end of the text body before the footer

### Requirement: Multimodal Controls and Guidance
The system SHALL provide user controls and guidance for multimodal capture.

#### Scenario: Settings guidance for permissions
- **WHEN** the user views multimodal capture settings
- **THEN** the UI explains that the feature may request Accessibility, Automation, and Screen Recording permissions

#### Scenario: Localized help entry points
- **WHEN** the user views multimodal capture settings
- **THEN** the UI provides localized help text or a link to the multimodal usage guide
