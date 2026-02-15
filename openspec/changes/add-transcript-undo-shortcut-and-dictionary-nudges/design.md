## Context
Handy already supports global shortcuts for transcription and has alias suggestions in History UI. What is missing is a strict "undo last Handy paste" action plus proactive nudges based on repeated undo corrections.

This change introduces four state layers:
1. recent-paste tracking (strict undo gate)
2. frontend heuristic bridge state (Rust<->frontend evaluation handshake)
3. persisted undo-evidence counters (nudge trigger memory across restarts)
4. overlay nudge presentation state (interactive feedback/actions)

## Goals / Non-Goals
- Goals:
  - Provide a strict undo shortcut that acts only on tracked recent Handy pastes.
  - Define concrete paste-tracking semantics (data shape, storage, TTL, single-use behavior).
  - Reuse existing frontend alias heuristic without duplicating logic in Rust.
  - Trigger dictionary nudges from repeated undo evidence with persisted counters.
  - Deliver undo feedback and nudges through existing overlay UX (primary path).
  - Ensure overlay nudge controls are accessible for keyboard and assistive-technology users.
  - Avoid silent no-op undo behavior.
  - Add structured logs for debugging this multi-stage flow.
- Non-Goals:
  - Multi-level undo/redo stack across external apps.
  - Auto-adding dictionary entries without explicit user action.
  - Reworking shortcut metadata i18n architecture outside this feature.
  - Replacing the existing recording overlay visual language.

## Key Decisions
- Decision: keep `suggestAliasFromTranscript` in frontend and use an event-driven bridge from Rust.
  - Reason: avoids algorithm duplication and preserves parity with existing History suggestions.
  - Alternative considered: full Rust port; rejected for V1 due to duplicated maintenance surface.
- Decision: use overlay-first feedback/nudge flow instead of native OS notifications.
  - Reason: overlay is already interactive, familiar, and visible while main window is hidden.
  - Consequence: nudges are actionable in place without a notification->toast handoff.
  - Fallback: non-Linux uses focused-window toast when overlay is unavailable; Linux uses native notification -> app activation -> actionable toast when overlay is unavailable (default path).
- Decision: add a 200ms modifier-release delay before synthetic undo dispatch.
  - Reason: existing paste/refine actions already need this to avoid stale held modifiers.
- Decision: undo uses Escape-equivalent cancellation path for active operations before deciding undo dispatch.
  - Rule: if recording/transcription/refine/post-process is active, invoke the same cancellation flow as Escape.
  - Rule: maintain a 500ms stop-transition marker after recording stop so immediate undo can still cancel pending pipeline start.
  - Reason: avoids the confusing "Nothing to undo" race right after stopping recording.
- Decision: `paste_id` uses a monotonic `AtomicU64` generator in Rust (`fetch_add(1)`).
  - Reason: deterministic correlation without timestamp collisions.
- Decision: discoverability hint is one-time-ever with deferred timing.
  - Rule: show only after second successful tracked paste, with 2.5s delay, and only if user has not used undo yet.
  - Reason: reduces first-run interruption and keeps hint contextual.
- Decision: persist undo evidence in dedicated `undo_nudge_store.json`.
  - Reason: avoid bloating `settings_store.json` and isolate corruption risk.

## Data Model
### RecentPasteSlot (in-memory only)
- Storage: Rust managed app state.
- Lifecycle: ephemeral; reset on app restart.
- Cardinality: single slot (most recent Handy paste only).
- `paste_id` generation: global `AtomicU64` counter, incremented on each successful tracked paste.
- Fields:
  - `paste_id: u64`
  - `source_action: String` (`transcribe`, `transcribe_with_post_process`, `paste_last_transcript`, `refine_last_transcript`)
  - `auto_refined: bool` (true only for transcribe path when `auto_refine_enabled` produced final pasted text)
  - `pasted_text: String` (exact payload sent to paste routine, including trailing-space transport behavior)
  - `suggestion_text: String` (raw ASR transcript source used for heuristic evaluation)
  - `created_at_ms: u64`
  - `expires_at_ms: u64` (`created_at + 120_000`)
  - `consumed: bool`
- Explicitly not stored: target application bundle/process identity.
- Rules:
  - Slot is valid only if not expired and not consumed.
  - New paste overwrites previous slot.
  - After one undo dispatch attempt, slot is marked consumed.
  - A second undo press without a new paste is a no-op.
  - `paste_id` namespace resets on app restart together with this slot and pending bridge queue; cross-restart collisions are intentionally harmless.

### UndoHeuristicBridgeState (in-memory only)
- Storage: Rust managed app state.
- Lifecycle: ephemeral.
- Fields:
  - `evaluator_ready: bool` (frontend listener availability flag)
  - `pending_requests: VecDeque<UndoEvalRequest>` (FIFO queue, cap 64)
- Rules:
  - If evaluator is unavailable, requests are queued.
  - Queue is flushed when frontend signals evaluator-ready.
  - On overflow, oldest request is dropped with warning log.
  - Queue is ephemeral and cleared on restart.

### UndoOperationState (in-memory only)
- Storage: Rust managed app state.
- Lifecycle: ephemeral.
- Fields:
  - `operation_active: bool` (any cancelable recording/transcription/refine/post-process state)
  - `recent_stop_transition_started_ms: Option<u64>` (set when recording stop is triggered, cleared on pipeline settle)
- Rules:
  - `recent_stop_transition_started_ms` expires after 500ms.
  - Undo treats active operation or non-expired stop transition as cancelable.

### UndoNudgeEvidence (persisted)
- Storage: dedicated JSON store file `undo_nudge_store.json`.
- Lifecycle: survives restart.
- Fields:
  - `alias_counts: HashMap<String, u32>` where key is `entry_identity + "|" + alias`
  - `alias_last_shown: HashMap<String, u32>` (count snapshot when last shown)
  - `alias_last_seen_ms: HashMap<String, u64>` (for deterministic eviction)
  - `suppressed_alias_identities: HashSet<String>` (user-selected "Don't suggest this")
  - `unresolved_count: u32` (undos with no candidate mapping)
  - `unresolved_last_shown: u32`
  - `has_seen_undo_hint: bool`
  - `successful_paste_count: u32`
  - `has_used_undo: bool`
- Rules:
  - No time cooldown.
  - Alias nudge trigger when `alias_counts[key] > 3` and `alias_counts[key] - alias_last_shown[key] > 3`.
  - Unresolved nudge trigger when `unresolved_count > 3` and `unresolved_count - unresolved_last_shown > 3`.
  - Alias evidence map cap: 100 identities; evict oldest by `alias_last_seen_ms` when cap exceeded.
  - If `identity_key` exists in `suppressed_alias_identities`, alias nudge is never surfaced.

### OverlayUndoFeedbackState (in-memory only)
- Storage: frontend overlay component state plus Rust event payloads.
- Modes:
  - `undo_success`
  - `undo_recording_canceled`
  - `undo_processing_canceled`
  - `undo_noop_empty`
  - `undo_noop_expired`
  - `undo_nudge_alias`
  - `undo_nudge_unresolved`
- Rules:
  - undo/no-op feedback auto-dismisses quickly.
  - nudge modes remain until dismiss/action timeout policy.
  - single active overlay card at a time with deterministic arbitration rules.

## Paste-Capture Semantics
### Capture Source By Action
- `transcribe` with `auto_refine_enabled=false`:
  - `pasted_text`: final transcription payload passed to paste routine
  - `suggestion_text`: raw ASR transcript (`transcription_text`)
  - `auto_refined=false`
- `transcribe_with_post_process`:
  - `pasted_text`: post-processed text payload passed to paste routine
  - `suggestion_text`: raw ASR transcript (`transcription_text`) before post-processing
  - `auto_refined=true`
- `transcribe` with `auto_refine_enabled=true` and refinement success:
  - `pasted_text`: refined text payload passed to paste routine
  - `suggestion_text`: raw ASR transcript (`transcription_text`), not refined output
  - `auto_refined=true`
- `paste_last_transcript`:
  - `pasted_text`: text selected for paste (post-processed if present, else raw)
  - `suggestion_text`: underlying raw transcript from latest history entry
  - `auto_refined=false`
- `refine_last_transcript`:
  - `pasted_text`: refined output payload
  - `suggestion_text`: raw transcript used as refine input
  - `auto_refined=true`

## Behavioral Flow
### Paste Tracking
1. Paste is executed on main thread (`run_on_main_thread` closure).
2. Inside that same closure, immediately after `utils::paste(...)` returns `Ok(())`, write `RecentPasteSlot` with new `paste_id`.
3. This ordering prevents stale-slot races with other shortcut triggers.

### Undo Action
1. If recording/transcription/refine/post-process is active, trigger Escape-equivalent cancel flow.
2. If no operation is active but stop-transition marker is non-expired (<=500ms after recording stop), trigger the same cancel flow.
3. Resolve slot validity after cancel step:
  - if no valid `RecentPasteSlot` exists because slot missing/consumed and cancel occurred, emit `Processing canceled`
  - if no valid `RecentPasteSlot` exists because slot missing/consumed and cancel did not occur, emit `Nothing to undo`
4. If slot exists but TTL expired, ignore undo and emit overlay feedback: `Undo expired`.
5. Sleep 200ms to allow user shortcut modifiers to be released.
6. Dispatch exactly one platform undo chord on main thread.
7. Mark slot consumed immediately after dispatch attempt and mark `has_used_undo=true`.
8. Emit overlay success feedback (`Undo applied`) and emit/queue heuristic evaluation request using `suggestion_text` and `paste_id`.

### Heuristic Bridge Flow
1. Rust emits `undo-heuristic-evaluate` request payload.
2. Frontend listener runs `suggestAliasFromTranscript` using current dictionary settings.
3. Frontend returns result via Tauri command/event (`candidate` or `no_candidate`) keyed by `paste_id`.
4. Rust updates persisted evidence counters from returned result.
5. If frontend is unavailable (tray-only/not loaded), request stays queued until evaluator-ready signal.

### Similarity Definition
- "Similar problematic phrase pattern" is defined as producing the same suggestion identity (`entry_identity + alias`) from the frontend heuristic.

### Nudge Evaluation
1. Candidate result:
  - increment `alias_counts[identity]`
  - skip surfacing if `identity` is user-suppressed
  - apply alias threshold and repeat-gating rules otherwise
2. No-candidate result:
  - increment `unresolved_count`
  - derive `unresolved_excerpt` from most recent no-candidate `suggestion_text` using deterministic truncation/normalization
  - apply unresolved threshold and repeat-gating rules

## Overlay UX Flow
### Undo Feedback
- Success dispatch: overlay message `Undo applied`.
- Recording canceled with no valid slot: overlay message `Recording canceled`.
- Processing canceled with no valid slot: overlay message `Processing canceled`.
- No-op missing/consumed slot: overlay message `Nothing to undo`.
- No-op expired slot: overlay message `Undo expired`.

### Nudge Presentation
- Primary delivery: overlay nudge card (interactive).
- Alias nudge card:
  - primary action: `Add "<alias>" -> "<term>"`
  - secondary action: `Don't suggest this`
  - dismiss action
- Unresolved nudge card:
  - primary action: `Open Dictionary`
  - dismiss action
  - body includes localized phrase excerpt derived from most recent no-candidate input (for example: `"speck" repeatedly misrecognized`)

### Dictionary Navigation From Overlay
- Overlay `Open Dictionary` action triggers backend window-focus path and frontend navigation event:
  - backend ensures main window is shown/focused
  - backend emits navigation event payload (`dictionary` section)
  - frontend App listener sets active sidebar section to dictionary

### Fallbacks
- Non-Linux: if overlay webview is unavailable or disabled and main window is focused, surface feedback/nudge as in-app toast immediately.
- Linux: if overlay is unavailable/disabled (default), use native notification as attention signal.
- Linux notification activation flow:
  - notification click brings app window to foreground
  - on activation/focus, app surfaces actionable toast for pending nudge
  - toast holds dictionary actions (`Add alias` or `Open Dictionary`)
- Linux permission fallback:
  - if notification permission is denied, queue pending nudge and surface actionable toast on next app focus.

### Overlay Event Arbitration
- Single-card model:
  - only one overlay card is visible at a time.
- Priority:
  - nudge cards (`undo_nudge_alias`, `undo_nudge_unresolved`) have higher priority than transient undo feedback states.
- Replacement:
  - a newer nudge replaces an existing nudge card.
  - transient feedback received while a nudge is active is queued as latest-only and shown after nudge dismissal/timeout.

## Shortcut Defaults
- macOS: `control+command+z`
- Windows: `ctrl+alt+z`
- Linux: `ctrl+alt+z`
- Defaults must pass reserved-shortcut validation.

## Technical Constraints
- Undo key simulation must follow existing Enigo safety constraints:
  - dispatch on main thread
  - 200ms pre-dispatch delay for modifier release
  - macOS: `Key::Other(6)` (`Z`)
  - Windows: `Key::Other(0x5A)` (`VK_Z`)
  - Linux: `Key::Unicode('z')`
- New static/global additions for this feature should use `std::sync::LazyLock` patterns consistent with ongoing migration; avoid introducing fresh `once_cell::sync::Lazy` usage.
- Existing static touched by this feature in `overlay.rs` should be migrated from `once_cell::sync::Lazy` to `std::sync::LazyLock` as prep/refactor work in this change.
- This feature follows existing shortcut-binding metadata pattern in `settings.rs` (hardcoded English defaults); UI-facing labels remain i18n-driven in frontend translation files.

## Discoverability
- Purpose: the shortcut is hidden behind settings and not naturally discoverable in first sessions.
- Show undo discoverability hint in overlay context after second successful tracked paste when:
  - `has_seen_undo_hint == false`
  - `has_used_undo == false`
  - no active nudge card is visible
  - a 2.5 second delay has elapsed since paste success
  - hint copy explicitly says undo applies within 2 minutes
- If overlay is not available at that moment, show once via focused-window toast or hidden-window native notification fallback.
- Persist `has_seen_undo_hint = true` after first display (one-time ever behavior).

## Logging Plan (Using log skill guidance)
- Emit structured logs for the following events and reason codes:
  - paste slot lifecycle: `undo_slot_created`, `undo_slot_overwritten`, `undo_slot_consumed`, `undo_slot_expired`
  - undo command path: `undo_dispatch_attempted`, `undo_dispatch_skipped` with reason (`missing_slot`, `consumed_slot`, `expired_slot`)
  - operation cancellation path: `undo_operation_cancel_requested`, `undo_operation_cancel_completed`, `undo_operation_cancel_no_slot`, `undo_stop_transition_cancel_requested`
  - heuristic bridge: `undo_eval_enqueued`, `undo_eval_sent`, `undo_eval_result_received`, `undo_eval_dropped_overflow`
  - nudge decisions: `undo_nudge_triggered`, `undo_nudge_suppressed`, `undo_nudge_applied`, `undo_nudge_identity_suppressed`
  - overlay arbitration: `undo_overlay_event_queued`, `undo_overlay_event_replaced`
- Required fields where relevant:
  - `paste_id`, `source_action`, `auto_refined`, `reason`, `identity_key`, `alias_count`, `unresolved_count`, `queue_len`

## Risks / Trade-offs
- Risk: target app may ignore synthetic undo despite command dispatch.
  - Mitigation: deterministic slot consumption and explicit best-effort docs.
- Risk: some editors may require multiple undos for multi-part paste operations.
  - Mitigation: V1 intentionally sends one undo command per trigger.
- Risk: 120-second TTL may feel short/long in edge workflows.
  - Rationale: balances fast correction loops with reducing accidental late undos across context switches.
  - Mitigation: explicit `Undo expired` feedback and revisit configurability if needed.
- Risk: unresolved nudge may still feel generic.
  - Mitigation: include problematic phrase excerpt in nudge body.
- Risk: evidence growth over long usage.
  - Mitigation: explicit cap of 100 alias identities with deterministic eviction.
- Risk: clipboard overwrite mode can still lose previous clipboard content even if undo removes pasted text.
  - Mitigation: document as known limitation in V1; undo does not alter existing clipboard mode semantics.
- Risk: Linux default overlay disabled (`OverlayPosition::None`) reduces immediate visual feedback.
  - Mitigation: explicit Linux-native notification activation path that leads to actionable toast in app.
- Risk: extremely narrow gaps may remain around state transitions.
  - Mitigation: explicit stop-transition marker plus cancellation-first undo semantics.

## Migration Plan
1. Add undo shortcut defaults and registration/reset coverage.
2. Implement `RecentPasteSlot` tracking with atomic `paste_id`, explicit capture semantics, and strict undo behavior.
3. Implement Rust<->frontend heuristic bridge with queue/fallback behavior.
4. Implement persisted `UndoNudgeEvidence` in `undo_nudge_store.json` with cap/eviction.
5. Extend overlay states/events for undo feedback + nudge actions, suppression action, and arbitration rules.
6. Add structured logs and tests for slot semantics, bridge reliability, threshold behavior, and UX flows.
7. Add user-facing documentation in `doc/undo-paste-last-transcript.md`.
