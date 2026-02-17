# Undo Last Transcript

## Overview

`undo_last_transcript` is a strict global shortcut that undoes only the most recent Handy/Codictate-originated paste.

Fast loop:
1. Dictate
2. Check result
3. Undo quickly
4. Retry

## Default Shortcut

- macOS: `control+command+z`
- Windows: `ctrl+alt+z`
- Linux: `ctrl+alt+z`

You can customize this in Keyboard Shortcuts settings.

## Strict Tracking Rules

Undo operates on a single in-memory tracked paste slot.

- Only the most recent eligible paste is tracked.
- TTL is 120 seconds.
- First undo press after expiry shows `Undo expired` and clears that slot.
- Next undo press without a new tracked paste shows `Nothing to undo`.
- Slot is single-use: one dispatch attempt consumes it.
- App restarts clear tracked slot state.

## Eligible Paste Sources

- `transcribe`
- `transcribe_with_post_process`
- `paste_last_transcript`
- `refine_last_transcript`

## Processing-In-Flight Behavior

When undo is pressed during recording or processing:

- The app triggers the same cancellation path as Escape.
- A 500ms stop-transition marker catches immediate undo after recording stop.
- That keypress short-circuits after cancellation (no slot-based undo/no-op evaluation).
- UI remains on legacy cancelling overlay presentation.

## Feedback States

Undo feedback is explicit and uses the shared overlay message lane:

- `Undo applied`
- `Nothing to undo`
- `Undo expired`

These are transient, non-loading feedback messages.

## Discoverability Hint

One-time hint behavior:

- shown after second successful tracked paste
- delay: 2.5 seconds
- only if undo has not yet been used
- includes explicit `2 min` guidance

Persisted fields in `settings_store.json` under key `undo_discoverability`:

- `has_seen_undo_hint`
- `successful_paste_count`
- `has_used_undo`

## Stats Rollback

For transcribe-origin undo targets (`transcribe`, `transcribe_with_post_process`):

- undo dispatch triggers best-effort rollback of cumulative `user_stats` contribution
- rollback may be deferred until async contribution metadata is available
- history entries are not deleted by rollback

For non-transcribe sources (`paste_last_transcript`, `refine_last_transcript`):

- no stats rollback is applied

## Clipboard Note

Undo removes pasted text in target app, but does not perform extra clipboard restoration beyond existing paste-mode behavior.
