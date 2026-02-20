# History Inserted-Text Parity Manual Test Checklist

## Goal
Verify that History primary text matches what was actually inserted into the target app (when available), while preserving raw ASR text for transparency.

## Key Behavior to Remember
1. Primary history text now uses `effective_text`:
   - `inserted_text` -> `post_processed_text` -> `transcription_text`.
2. `Original transcript` appears only when raw ASR differs from primary.
3. Older rows are not rewritten; they use fallback behavior.

## Setup
- [ ] Run app with latest code:
  - `cd /Users/tiger/Dev/opensource/speechGen/Handy`
  - `bun run tauri:dev`
- [ ] Open a plain target editor (TextEdit/Notes) so inserted text is easy to verify.
- [ ] Open Codictate homepage history list.
- [ ] Ensure Smart Insertion is enabled.

## A. Baseline (Old Entries)
- [ ] Open several older history rows from before this change.
- [ ] Confirm app does not crash and rows still render.
- [ ] Confirm many old rows may look unchanged (expected).

Expected:
- Old rows normally have `inserted_text = null`.
- Their primary text falls back to post-processed or raw transcript.
- If fallback result equals raw text, `Original transcript` button does not appear.

## B. New Transcribe Row (Inserted Text Persistence)
- [ ] Place cursor in target editor at punctuation boundary:
  - Seed text: `Hello |, world` (cursor between `Hello` and comma).
- [ ] Trigger `transcribe` and speak something ending with punctuation, for example: `there.`
- [ ] Confirm inserted text in editor reflects smart insertion transform (for this case, comma boundary behavior should apply).
- [ ] Open latest history row immediately.

Expected:
- Primary history text matches the inserted editor text, not necessarily raw ASR verbatim.
- If raw ASR differs, `Original transcript` button is visible.

## C. Original Transcript Disclosure UX
- [ ] On a row where primary and raw differ, click `Original transcript`.
- [ ] Confirm raw panel expands inline under the primary text.
- [ ] Confirm button text toggles to hide state.
- [ ] Collapse it again.

Expected:
- No modal/popover.
- Inline panel with muted style.
- Keyboard operable control with expanded/collapsed behavior.

## D. Copy Behavior
- [ ] On a row where primary differs from raw, click row copy action.
- [ ] Paste into editor.

Expected:
- Copied text equals primary/effective text.
- It should match what was inserted result, not raw ASR.

## E. Search Behavior (Effective + Raw)
- [ ] Search by a token visible in primary text only.
- [ ] Search by a token only present in raw transcript (not in primary).

Expected:
- Both searches return the row.
- For raw-only match while collapsed, hint appears: `Matched in original transcript`.
- Clicking hint expands raw panel.

## F. Paste Last Transcript
- [ ] Create a fresh row where primary differs from raw.
- [ ] Trigger `paste_last_transcript`.

Expected:
- Pasted text equals row primary/effective text.
- Undo behavior remains unchanged.

## G. Refine Last Transcript
- [ ] Trigger `refine_last_transcript` on the latest row.
- [ ] Confirm refine output is pasted.
- [ ] Re-open same latest row in history.

Expected:
- Same row is updated (latest-only semantics).
- Refine input is still based on raw ASR.
- Primary now reflects latest inserted refined output when paste succeeded.
- Raw panel still shows original ASR transcript.

## H. Failure/Skip Safety
- [ ] Temporarily set paste method to `None` and run a transcription.
- [ ] Restore paste method and run another transcription.

Expected:
- With paste skipped: no crash, row still appears, primary falls back correctly.
- After restore: inserted text persistence resumes on successful paste.

## Pass Criteria
- [ ] No migration/data loss symptoms (old rows intact).
- [ ] New rows reflect inserted output in primary line.
- [ ] Raw transcript is available on demand only when different.
- [ ] Search, copy, paste-last, refine-last all behave as specified.
- [ ] No regressions in smart insertion, undo, or normal paste flow.
