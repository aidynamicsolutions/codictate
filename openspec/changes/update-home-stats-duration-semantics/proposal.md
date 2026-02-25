# Change: Update Home Stats Duration Semantics

## Why
Home stats currently derive duration from padded ASR samples and subtract a fixed 900ms heuristic. This causes WPM and Time Saved to drift away from real recording behavior, especially for short clips, and makes lifetime metrics difficult to reason about.

## What Changes
- Replace heuristic duration accounting with explicit dual-duration tracking.
- Persist both recording elapsed duration and VAD-retained speech duration per transcription.
- Compute `WPM` from speech duration and `Time Saved` from recording duration.
- Add crash-safe, idempotent duration backfill with a semantics version marker.
- Add pre-mutation stats backups to filesystem and database before backfill writes.
- Keep `HomeStats` API shape stable for frontend compatibility.
- Update docs and tooltip copy to reflect the new formulas.

## Impact
- Affected specs: `transcription-history`
- Affected code:
  - `src-tauri/src/managers/audio.rs`
  - `src-tauri/src/actions.rs`
  - `src-tauri/src/managers/history.rs`
  - `src-tauri/src/undo.rs`
  - `src/i18n/locales/en/translation.json`
  - `doc/stats.md`
