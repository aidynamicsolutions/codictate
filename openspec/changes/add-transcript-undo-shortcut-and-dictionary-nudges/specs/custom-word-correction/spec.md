## ADDED Requirements
### Requirement: Undo-Driven Dictionary Suggestions
The system SHALL detect repeated undo events for Handy-originated transcript pastes and proactively suggest dictionary alias additions when a repeatable correction pattern is identified.

#### Scenario: Repeated undo pattern reaches suggestion threshold
- **WHEN** a user repeatedly undoes transcript pastes that resolve to the same suggestion identity
- **AND** matching undo evidence count for that identity becomes greater than `3`
- **AND** a valid alias candidate can be mapped to an existing dictionary entry
- **THEN** the system emits a suggestion nudge containing the proposed alias and target dictionary term

#### Scenario: Pattern does not meet threshold
- **WHEN** undo events are infrequent or not pattern-consistent
- **OR** matching undo evidence count is `3` or less
- **THEN** no dictionary suggestion nudge is shown

### Requirement: Shared Alias Heuristic Consistency
Undo-driven suggestions SHALL use the same alias suggestion heuristics used by History-based alias suggestions.

#### Scenario: Undo and History produce same candidate
- **WHEN** the same transcript phrase is evaluated by undo-driven suggestion flow and History suggestion flow
- **THEN** both flows produce the same alias candidate and dictionary target identity

#### Scenario: Similarity identity definition
- **WHEN** multiple undone transcripts are evaluated for repeat detection
- **THEN** transcripts are treated as similar only when they produce the same `entry_identity + alias` suggestion identity

### Requirement: Undo Heuristic Evaluation Bridge
Undo-driven suggestion evaluation SHALL bridge backend undo events to frontend heuristic execution and return structured results to backend counters.

#### Scenario: Frontend evaluator handles undo request
- **WHEN** backend emits an undo-heuristic evaluation request containing `paste_id` and `suggestion_text`
- **THEN** frontend evaluates that payload using the existing alias suggestion heuristic
- **AND** frontend returns candidate/no-candidate result keyed to the originating `paste_id`

#### Scenario: Frontend evaluator unavailable
- **WHEN** backend needs heuristic evaluation while frontend evaluator is unavailable
- **THEN** evaluation requests are queued
- **AND** queued requests are processed when frontend evaluator becomes ready

#### Scenario: App restart resets queued bridge requests and correlation namespace
- **WHEN** the app restarts
- **THEN** pending undo evaluation requests are cleared
- **AND** paste-id correlation namespace is reset with no cross-restart request matching

### Requirement: Persistent Undo Evidence
Undo-driven nudge evidence counters SHALL persist across app restarts.

#### Scenario: Evidence survives restart
- **WHEN** a user accumulates undo evidence, restarts the app, and continues undoing the same pattern
- **THEN** the existing evidence count is preserved
- **AND** nudge threshold evaluation continues from the preserved count

### Requirement: Unresolved Pattern Fallback Nudge
The system SHALL provide a generic dictionary nudge when repeated undo patterns do not map to an existing dictionary entry.

#### Scenario: No candidate mapping after repeated undos
- **WHEN** undo-driven evaluation finds no valid alias candidate
- **AND** unresolved undo evidence count becomes greater than `3`
- **THEN** the system emits an unresolved-pattern nudge prompting the user to open Dictionary settings
- **AND** nudge content includes a short problematic phrase excerpt derived from the most recent no-candidate `suggestion_text`

### Requirement: Count-Based Nudge Gating
The system SHALL suppress duplicate nudges for the same suggestion using count-based evidence gates and SHALL NOT rely on time-window cooldowns.

#### Scenario: Duplicate suggestion is suppressed until additional evidence
- **WHEN** a nudge has already been shown for a specific alias suggestion identity
- **THEN** the system does not immediately repeat that nudge
- **AND** the nudge is shown again only after additional matching undo evidence count becomes greater than `3` since the prior nudge

#### Scenario: No time-based cooldown blocks valid repeat evidence
- **WHEN** additional matching undo evidence exceeds the repeat threshold soon after a previous nudge
- **THEN** the nudge may be shown again without waiting for a time delay

### Requirement: Overlay Nudge Delivery
Undo-driven suggestion nudges SHALL be delivered through the existing overlay UI as the primary interaction surface.

#### Scenario: Threshold reached with overlay available
- **WHEN** undo-driven suggestion threshold is reached and overlay UI is available
- **THEN** the system shows an interactive overlay nudge card
- **AND** the card presents the actions for that suggestion type

#### Scenario: Non-Linux overlay unavailable fallback
- **WHEN** undo-driven suggestion threshold is reached and overlay UI is unavailable
- **AND** the main window is focused
- **THEN** the nudge is shown as in-app toast immediately

#### Scenario: Linux overlay unavailable uses notification-to-toast flow
- **WHEN** undo-driven suggestion threshold is reached on Linux and overlay UI is unavailable
- **THEN** the system shows a native notification as attention signal
- **AND** activating the notification brings the app to foreground
- **AND** an actionable in-app toast is shown after activation

#### Scenario: Linux notification permission denied fallback
- **WHEN** Linux notification-to-toast flow is needed
- **AND** native notification permission is denied
- **THEN** the nudge is queued as pending
- **AND** the pending actionable toast is shown on next app focus

### Requirement: Focused Nudge Actions
Undo-driven nudge actions SHALL minimize cognitive load and present only the minimum actionable choices.

#### Scenario: Alias nudge actions
- **WHEN** alias nudge is shown in overlay
- **THEN** it provides a single primary action `Add "<alias>" -> "<term>"`
- **AND** it provides suppression action `Don't suggest this`
- **AND** it provides dismiss

#### Scenario: Unresolved nudge actions
- **WHEN** unresolved nudge is shown in overlay
- **THEN** it provides a single primary action `Open Dictionary`
- **AND** it provides dismiss
- **AND** it includes phrase context so users know what to add
- **AND** phrase context copy uses localized template interpolation with the dynamic phrase value

### Requirement: Dictionary Navigation From Nudge
Nudge actions that require dictionary editing SHALL open the app window and navigate directly to Dictionary.

#### Scenario: User opens dictionary from unresolved nudge
- **WHEN** the user clicks `Open Dictionary` in an unresolved overlay nudge
- **THEN** the main app window is shown and focused
- **AND** the app navigates directly to the Dictionary section

### Requirement: Alias Nudge Suppression
Users SHALL be able to suppress repeated nudges for a specific alias suggestion identity.

#### Scenario: User suppresses identity from alias nudge
- **WHEN** the user clicks `Don't suggest this` for an alias suggestion identity
- **THEN** that identity is added to persistent suppression state
- **AND** future undo evidence for that identity does not surface alias nudges

#### Scenario: Suppressed identity does not affect other identities
- **WHEN** one alias suggestion identity is suppressed
- **THEN** other alias suggestion identities continue to evaluate and nudge normally

### Requirement: Overlay Event Arbitration
Undo overlay feedback and nudge events SHALL resolve overlapping events deterministically.

#### Scenario: Transient undo feedback arrives while nudge card is visible
- **WHEN** a nudge card is visible and transient undo feedback arrives
- **THEN** the nudge card remains visible
- **AND** transient feedback is queued using latest-only replacement behavior

#### Scenario: New nudge arrives while nudge card is visible
- **WHEN** a nudge card is visible and a newer nudge event is triggered
- **THEN** the existing nudge card is replaced by the newer nudge card

### Requirement: Overlay Nudge Accessibility
Interactive overlay nudge controls SHALL be accessible to screen-reader and keyboard users.

#### Scenario: Accessible control labeling
- **WHEN** overlay nudge actions are rendered
- **THEN** action controls expose accessible names/labels for assistive technologies
- **AND** semantic roles are applied so controls are announced as actionable buttons

#### Scenario: Screen reader announcement for transient feedback
- **WHEN** undo feedback or nudge text appears in overlay
- **THEN** feedback is announced through an accessibility live-region mechanism

#### Scenario: Keyboard activation paths
- **WHEN** overlay nudge card is focused
- **THEN** users can navigate actions via keyboard
- **AND** Enter/Space activates the focused action

### Requirement: Evidence Map Capacity
Undo-driven alias evidence storage SHALL cap tracked alias identities to bounded size.

#### Scenario: Alias identity cap reached
- **WHEN** alias evidence storage reaches 100 tracked identities
- **THEN** adding a new identity evicts the least-recently-seen identity
- **AND** eviction preserves deterministic nudge behavior for remaining identities

### Requirement: Nudge Apply Safety
Applying an undo-driven alias suggestion SHALL update dictionary settings safely and idempotently.

#### Scenario: User accepts suggested alias
- **WHEN** the user chooses to add a suggested alias
- **THEN** the alias is appended to the targeted dictionary entry
- **AND** dictionary persistence uses the same normalization/duplicate checks as manual alias editing

#### Scenario: Suggested alias already exists
- **WHEN** the user chooses to add an alias that is already present
- **THEN** dictionary entries remain unchanged
- **AND** the user receives a non-error informational response
