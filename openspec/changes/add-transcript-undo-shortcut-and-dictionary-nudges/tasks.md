## 1. OpenSpec Deltas
- [ ] 1.1 Add `shortcut-settings` spec delta for transcript undo shortcut behavior and settings integration
- [ ] 1.2 Add `custom-word-correction` spec delta for undo-driven dictionary suggestion nudges and overlay actions
- [ ] 1.3 Add `observability` spec delta for structured undo/nudge logging requirements
- [ ] 1.4 Validate this change with `openspec validate add-transcript-undo-shortcut-and-dictionary-nudges --strict`

## 2. Backend Shortcut and Input
- [ ] 2.1 Add `undo_last_transcript` default shortcut binding values:
  - [ ] macOS: `control+command+z`
  - [ ] Windows: `ctrl+alt+z`
  - [ ] Linux: `ctrl+alt+z`
- [ ] 2.2 Ensure defaults pass reserved shortcut validation; add/update reserved-shortcut tests if needed
- [ ] 2.3 Add undo action implementation in `src-tauri/src/actions.rs` and register in `ACTION_MAP`
- [ ] 2.4 Add platform-aware undo key helper in `src-tauri/src/input.rs`
- [ ] 2.5 Enforce undo dispatch constraints:
  - [ ] run on main thread
  - [ ] add 200ms modifier-release delay before synthetic undo key dispatch
  - [ ] macOS uses `Key::Other(6)` for `Z` chord path
  - [ ] Windows uses `Key::Other(0x5A)` (`VK_Z`)
  - [ ] Linux uses Linux-compatible `z` key dispatch path
- [ ] 2.6 Wire shortcut registration/reset modal flows to include `undo_last_transcript`
- [ ] 2.7 Update `/src/components/shared/KeyboardShortcutsModal.tsx` to render `undo_last_transcript` directly below `paste_last_transcript`
- [ ] 2.8 Follow `std::sync::LazyLock` patterns for new static wiring; avoid introducing fresh `once_cell::sync::Lazy` usage
- [ ] 2.9 Migrate `OVERLAY_STATE` in `src-tauri/src/overlay.rs` from `once_cell::sync::Lazy` to `std::sync::LazyLock` as part of this feature touch

## 3. Paste Tracking Infrastructure (Dependency for Undo/Nudges)
- [ ] 3.1 Implement `RecentPasteSlot` in managed Rust state (single-slot, in-memory)
- [ ] 3.2 Add monotonic `paste_id` generation via `AtomicU64`
- [ ] 3.3 Track paste metadata (`paste_id`, `source_action`, `auto_refined`, `pasted_text`, `suggestion_text`, `created_at`, `expires_at`, `consumed`)
- [ ] 3.4 Set and enforce TTL of 120 seconds for recent paste validity
- [ ] 3.5 Write slot synchronously in the same main-thread paste-success path immediately after `utils::paste(...)` returns `Ok(())`
- [ ] 3.6 Overwrite slot on new paste and consume slot after one undo dispatch attempt
- [ ] 3.7 Define strict undo/no-op behavior by reason (missing slot, consumed slot, expired slot) with cancellation-first handling for active operations
- [ ] 3.8 Implement processing-in-flight undo behavior:
  - [ ] trigger Escape-equivalent cancellation for active recording/transcription/refine/post-process
  - [ ] add 500ms stop-transition marker after recording stop to catch immediate undo presses before pipeline start
  - [ ] continue undo flow if valid slot exists
  - [ ] emit `Processing canceled` feedback when no valid slot exists after cancellation
- [ ] 3.9 Lock suggestion capture semantics:
  - [ ] `suggestion_text` uses raw ASR transcript source (`transcription_text`)
  - [ ] `pasted_text` uses exact paste payload including transport modifiers
  - [ ] auto-refine transcribe/transcribe_with_post_process sets `auto_refined=true` while keeping raw `suggestion_text`

## 4. Heuristic Bridge and Evidence Engine
- [ ] 4.1 Implement backend->frontend undo heuristic evaluation request event payload (includes `paste_id` and `suggestion_text`)
- [ ] 4.2 Implement frontend evaluator listener that reuses `suggestAliasFromTranscript` and returns candidate/no-candidate results to backend
- [ ] 4.3 Implement evaluator availability handshake and pending request queue when frontend is unavailable
- [ ] 4.4 Add bounded pending request queue policy (cap 64, drop oldest with warning on overflow)
- [ ] 4.5 Persist `UndoNudgeEvidence` counters across restarts in dedicated `undo_nudge_store.json`
- [ ] 4.6 Define and enforce similarity identity key as `entry_identity + alias`
- [ ] 4.7 Implement alias nudge threshold rule (`> 3`) and repeat gating (`> 3` additional events)
- [ ] 4.8 Implement unresolved fallback counter/trigger for no-candidate undos (`> 3`)
- [ ] 4.9 Include phrase excerpts in unresolved nudge payloads
- [ ] 4.10 Cap alias evidence identities at 100 and evict least-recently-seen entries
- [ ] 4.11 Implement identity suppression set for alias nudges (`Don't suggest this`) with persistent storage and lookup

## 5. Overlay UX and Action Routing
- [ ] 5.1 Extend overlay state model for undo feedback and nudge cards (`undo_success`, `undo_recording_canceled`, `undo_processing_canceled`, `undo_noop_empty`, `undo_noop_expired`, alias nudge, unresolved nudge)
- [ ] 5.2 Emit overlay feedback for no-op reasons:
  - [ ] `Recording canceled`
  - [ ] `Processing canceled`
  - [ ] `Nothing to undo`
  - [ ] `Undo expired`
- [ ] 5.3 Emit overlay success feedback on undo dispatch (`Undo applied`)
- [ ] 5.4 Implement overlay nudge actions:
  - [ ] alias nudge: `Add "<alias>" -> "<term>"`
  - [ ] alias nudge: `Don't suggest this`
  - [ ] unresolved nudge: `Open Dictionary`
  - [ ] dismiss
- [ ] 5.5 Add backend+frontend navigation bridge so overlay action opens main window and routes to Dictionary section
- [ ] 5.6 Add fallback for overlay unavailable/disabled:
  - [ ] non-Linux focused main window -> in-app toast
  - [ ] Linux overlay unavailable -> native notification attention signal
  - [ ] Linux notification activation -> app focus -> actionable in-app toast
  - [ ] Linux notification-permission denied -> pending toast on next app focus
- [ ] 5.7 Add overlay event arbitration rules (single active card, nudge priority, deterministic replacement/queue behavior)
- [ ] 5.8 Add one-time-ever discoverability hint after second successful tracked paste:
  - [ ] delay hint by 2.5 seconds
  - [ ] gate on `has_used_undo=false`
  - [ ] include hint copy text `within 2 minutes`
  - [ ] suppress while nudge card is active
  - [ ] overlay primary with toast/notification fallback when overlay unavailable
- [ ] 5.9 Keep alias apply flow idempotent via existing normalization/duplicate checks
- [ ] 5.10 Implement frontend presentation-state reset on visibility events:
  - [ ] define behavior when overlay hides while nudge is visible
  - [ ] persist/requeue pending nudge state across overlay/main-window visibility transitions
  - [ ] restore pending presentation deterministically on next eligible visibility event

## 6. Localization and Copy
- [ ] 6.1 Add translation keys for undo shortcut label/description in settings UI
- [ ] 6.2 Add translation keys for overlay undo feedback copy (including processing-canceled state), alias nudge copy (including don't-suggest action), unresolved nudge copy, phrase-excerpt copy, and fallback toast/notification copy
- [ ] 6.3 Ensure unresolved phrase excerpt text uses localized interpolation template (for example `{ phrase }`) rather than string concatenation
- [ ] 6.4 Add translation keys for discoverability hint copy that includes shortcut token
- [ ] 6.5 Document that backend shortcut binding metadata remains hardcoded English by current architecture

## 7. Structured Logging (log skill aligned)
- [ ] 7.1 Add structured Rust logs for paste slot lifecycle (`undo_slot_created`, `undo_slot_overwritten`, `undo_slot_consumed`, `undo_slot_expired`)
- [ ] 7.2 Add structured Rust logs for undo dispatch/no-op reasons (`undo_dispatch_attempted`, `undo_dispatch_skipped` with reason)
- [ ] 7.3 Add operation-cancellation logs (`undo_operation_cancel_requested`, `undo_operation_cancel_completed`, `undo_operation_cancel_no_slot`, `undo_stop_transition_cancel_requested`)
- [ ] 7.4 Add structured bridge logs (`undo_eval_enqueued`, `undo_eval_sent`, `undo_eval_result_received`, overflow drop warning)
- [ ] 7.5 Add structured nudge decision logs (`undo_nudge_triggered`, `undo_nudge_suppressed`, `undo_nudge_applied`, `undo_nudge_identity_suppressed`)
- [ ] 7.6 Add overlay arbitration logs (`undo_overlay_event_queued`, `undo_overlay_event_replaced`)
- [ ] 7.7 Ensure logs include correlation fields (`paste_id`, `source_action`, `auto_refined`, `identity_key`, counts, queue length)

## 8. Testing
- [ ] 8.1 Unit tests for `RecentPasteSlot` semantics (TTL, overwrite, single-use consume)
- [ ] 8.2 Unit/integration tests for undo dispatch constraints (main-thread, 200ms delay, platform keycode paths)
- [ ] 8.3 Unit tests for paste tracking write ordering (slot write immediately after paste success path)
- [ ] 8.4 Bridge tests for frontend-unavailable queue and evaluator-ready flush behavior
- [ ] 8.5 Unit tests for similarity keying, suppression blocklist behavior, and alias/unresolved threshold behavior (`> 3`)
- [ ] 8.6 Persistence tests: evidence counters and `has_seen_undo_hint` survive restart in dedicated store
- [ ] 8.7 Unit tests for alias evidence cap/eviction at 100 identities
- [ ] 8.8 Frontend tests for overlay nudge rendering, action routing, and overlay arbitration behavior
- [ ] 8.9 Tests for alias apply idempotency (no duplicate additions)
- [ ] 8.10 Tests for no-op feedback reasons (`recording`, `missing`, `consumed`, `expired`) plus processing-cancel behavior
- [ ] 8.11 Tests for discoverability gating (second paste + delay + has_used_undo gate)
- [ ] 8.12 Tests for tracking source action `transcribe_with_post_process`
- [ ] 8.13 Accessibility tests for overlay nudge controls (labels/roles, keyboard activation, live-region announcements)
- [ ] 8.14 Manual verification across macOS/Windows/Linux:
  - [ ] shortcut defaults and reserved validation
  - [ ] overlay feedback and nudge interactions while main window is hidden
  - [ ] Linux default overlay-disabled behavior uses notification activation -> toast -> action flow
  - [ ] user-facing behavior in editors that require multiple undos for multi-part paste

## 9. Documentation
- [ ] 9.1 Add `doc/undo-paste-last-transcript.md` with:
  - [ ] shortcut defaults and how to customize
  - [ ] strict tracked-paste semantics (TTL, single-use)
  - [ ] explicit note that undo applies within 2 minutes
  - [ ] single-slot overwrite behavior (`paste_last_transcript` can replace the previous undo target)
  - [ ] processing-in-flight undo cancellation behavior
  - [ ] dictionary nudge flow and suppression action
  - [ ] Linux-specific notification -> app focus -> toast action flow
  - [ ] clipboard-mode limitation note
