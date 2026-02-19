## 1. OpenSpec Deltas
- [x] 1.1 Add `shortcut-settings` delta for transcript undo shortcut behavior
- [x] 1.2 Add `observability` delta for structured undo logs
- [x] 1.3 Remove `custom-word-correction` delta (undo-driven dictionary nudges removed)
- [x] 1.4 Validate this change with `openspec validate add-transcript-undo-shortcut-and-dictionary-nudges --strict`

## 2. Backend Shortcut and Input
- [x] 2.1 Add `undo_last_transcript` defaults (macOS `control+command+z`, Windows/Linux `ctrl+alt+z`)
- [x] 2.2 Ensure defaults pass reserved-shortcut validation
- [x] 2.3 Add undo action wiring in backend action map
- [x] 2.4 Keep platform-specific undo key dispatch behavior

## 3. Paste Tracking Infrastructure
- [x] 3.1 Implement single-slot `RecentPasteSlot`
- [x] 3.2 Add monotonic `paste_id` generation
- [x] 3.3 Capture tracked paste metadata fields
- [x] 3.4 Enforce 120s TTL
- [x] 3.5 Consume slot after one dispatch attempt
- [x] 3.6 Preserve cancellation-first short-circuit during active operations

## 4. Undo Feedback and Discoverability
- [x] 4.1 Emit feedback events for success/no-op/expired
- [x] 4.2 Keep discoverability hint after second successful tracked paste
- [x] 4.3 Persist only discoverability evidence fields in `settings_store.json` key `undo_discoverability`

## 5. Stats Rollback
- [x] 5.1 Keep stats contribution correlation tokens
- [x] 5.2 Keep rollback request/deferred/apply logic for transcribe-origin undo
- [x] 5.3 Keep non-transcribe rollback skip behavior

## 6. Remove Undo Nudge Subsystems
- [x] 6.1 Remove Rust<->frontend undo heuristic bridge
- [x] 6.2 Remove alias/unresolved nudge counters and gating
- [x] 6.3 Remove overlay nudge actions and dictionary-open command path
- [x] 6.4 Remove dictionary prefill intent store/util/modal plumbing
- [x] 6.5 Remove frontend undo evaluation helper and tests

## 7. Frontend Integration
- [x] 7.1 Remove undo evaluator and dictionary-open event listeners in `App.tsx`
- [x] 7.2 Keep `undo-main-toast` handling for feedback and discoverability only
- [x] 7.3 Remove nudge command wrappers/types from `bindings.ts`
- [x] 7.4 Remove nudge-specific i18n keys from English locale

## 8. Testing
- [x] 8.1 Unit tests for `RecentPasteSlot` semantics
- [x] 8.2 Unit tests for rollback source scope helper
- [x] 8.3 End-to-end test coverage for deferred rollback timing
- [x] 8.4 Manual verification across macOS/Windows/Linux

## 9. Documentation
- [x] 9.1 Update undo documentation for feedback-only behavior
- [x] 9.2 Update manual checklist to remove nudge scenarios
- [x] 9.3 Align overlay documentation with feedback/discoverability lane only
