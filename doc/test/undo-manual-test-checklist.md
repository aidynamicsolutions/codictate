# Undo Stats Rollback Manual Test Checklist

## Session Setup
- [ ] Step 0: Launch Codictate and a target editor (TextEdit/Notes).
- [ ] Step 1: Open Home and record baseline stats in notes: `total_words`, `wpm`, `time_saved_minutes`, `streak_days`.
- [ ] Step 2: Open Keyboard Shortcuts modal and confirm `undo_last_transcript` is present directly below `paste_last_transcript`.
- [ ] Step 3: Confirm undo default is `control+command+z` on macOS (or platform default on Windows/Linux).

## Log Verification Commands
- [ ] Step 4: Prepare latest log file command: `LOG_FILE=$(ls -1t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)`.
- [ ] Step 5: Use this command template after each test block: `rg -n "shortcut_init_|undo_slot_|undo_dispatch_|undo_operation_cancel_|undo_stats_rollback_|undo_discoverability_" "$LOG_FILE" | grep -v "Accessibility permission check" | tail -n 300`.

## Core Behavior: Transcribe Undo Rolls Back Stats
- [x] Step 6: Use `transcribe` to paste a sentence with at least 6 words into the editor.
- [x] Step 7: Confirm text appears and Home stats increase from baseline.
- [x] Step 8: Press `undo_last_transcript` once.
- [x] Step 9: Confirm pasted text is removed.
- [x] Step 10: Confirm Home stats return to baseline (allow minor display rounding only), and undo feedback appears in the shared overlay undo message lane.
- [x] Step 11: Ask assistant to verify logs for this block.
- [x] Step 12: Log pass criteria for this block: `undo_slot_created`, `undo_dispatch_attempted`, `undo_slot_consumed`, `undo_stats_rollback_requested`, and either `undo_stats_rollback_applied` directly or `undo_stats_rollback_deferred` then `undo_stats_rollback_applied`.

## Race Behavior: Deferred Rollback
- [x] Step 13: Trigger `transcribe` and press undo as quickly as possible right after paste appears.
- [x] Step 14: Confirm pasted text is removed.
- [x] Step 15: Confirm stats eventually return to baseline within a few seconds.
- [x] Step 16: Ask assistant to verify logs for deferred flow.
- [x] Step 17: Log pass criteria for this block: `undo_stats_rollback_requested` then `undo_stats_rollback_deferred` then `undo_stats_rollback_applied` with matching token lifecycle.

## Non-Transcribe Sources Must Not Roll Back Stats
- [x] Step 18: Trigger `paste_last_transcript`, then trigger undo.
- [x] Step 19: Confirm editor text undo behavior works.
- [x] Step 20: Confirm stats do not decrease for this undo.
- [x] Step 21: Ask assistant to verify logs for this block.
- [x] Step 22: Log pass criteria for this block: `undo_stats_rollback_skipped` with `reason=non_transcribe_source` and source action from paste-last flow.
- [x] Step 23: Trigger `refine_last_transcript`, then trigger undo.
- [x] Step 24: Confirm stats do not decrease for this undo.
- [x] Step 25: Ask assistant to verify logs for this block.
- [x] Step 26: Log pass criteria for this block: `undo_stats_rollback_skipped` with `reason=non_transcribe_source` and source action from refine-last flow.

## No-Op And Expiry Feedback
- [x] Step 27: Press undo again without a new tracked paste.
- [x] Step 28: Confirm UI shows `Nothing to undo` in the shared overlay undo message lane.
- [x] Step 29: Ask assistant to verify logs for this block.
- [x] Step 30: Log pass criteria for this block: `undo_dispatch_skipped` with `reason=missing_slot` or `reason=consumed_slot`.
- [x] Step 31: Create a new transcribe paste, wait more than 120 seconds, then press undo.
- [x] Step 32: Confirm UI shows `Undo expired` in the shared overlay undo message lane.
- [x] Step 33: Ask assistant to verify logs for this block.
- [x] Step 34: Log pass criteria for this block: first press shows `undo_slot_expired` and `undo_dispatch_skipped` with `reason=expired_slot`; immediate second press without a new paste shows `undo_dispatch_skipped` with `reason=missing_slot`.

## Processing Cancel Path
- [x] Step 35: Start recording and press undo while recording is active.
- [x] Step 36: Confirm UI stays on `Cancelling...` operation presentation and does not stack `Undo applied`/`Nothing to undo`/`Undo expired` for that keypress.
- [x] Step 37: Ask assistant to verify logs for this block.
- [x] Step 38: Log pass criteria for this block: `undo_operation_cancel_requested`, `undo_operation_cancel_completed`, and `undo_operation_cancel_short_circuit`; no `undo_dispatch_attempted` for that same keypress.

## Data Integrity
- [ ] Step 39: Open History view after rollback tests.
- [ ] Step 40: Confirm transcription history entries remain present even when stats were rolled back.
- [ ] Step 41: Ask assistant to verify there is rollback activity but no history-delete behavior tied to undo.

## Final Result
- [ ] Step 42: Mark test PASS only if all blocks passed UI and backend log checks.
- [ ] Step 43: If FAIL, record failing step number, observed UI behavior, expected behavior, and approximate timestamp.

## Startup Availability Regression (No UI Reopen Needed)
- [ ] Step 44: Set `start_hidden=true`, fully restart the app, and do not open the main window.
- [ ] Step 45: Trigger `transcribe`, then trigger `undo_last_transcript`.
- [ ] Step 46: Confirm undo works immediately without opening Keyboard Shortcuts modal.
- [ ] Step 47: Log pass criteria: startup path contains `shortcut_init_attempt` and `shortcut_init_success` with `source=backend_startup`.

## Permission Re-Grant Recovery
- [ ] Step 48: Launch with accessibility permission denied; confirm undo shortcut is unavailable.
- [ ] Step 49: Grant accessibility permission in System Settings and return to app.
- [ ] Step 50: Without opening Keyboard Shortcuts modal, trigger `transcribe`, then trigger `undo_last_transcript`.
- [ ] Step 51: Confirm undo now works and log pass criteria: `shortcut_init_deferred` followed by `shortcut_init_success` from a frontend recovery source.
