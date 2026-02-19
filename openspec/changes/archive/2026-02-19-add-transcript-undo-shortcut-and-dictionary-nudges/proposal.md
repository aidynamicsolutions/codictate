# Change: Add Transcript Undo Shortcut (Feedback-Only)

## Why
Users need a fast way to undo the last Handy/Codictate paste without manually selecting text in the target app.

The original proposal included undo-driven dictionary nudges, but that path added significant complexity and lower reliability than existing History-based alias suggestions. This change now focuses undo on strict paste undo behavior only.

## What Changes
- Add a dedicated global shortcut action `undo_last_transcript` that undoes only the most recent tracked Handy-originated paste.
- Expose `undo_last_transcript` in shortcut settings with platform defaults and reset coverage.
- Keep strict tracking semantics:
  - one in-memory recent-paste slot
  - 120 second TTL
  - single-use consume behavior
  - overwrite on newer tracked paste
- Keep cancel-first behavior:
  - when recording/transcription/refine/post-process is active, undo triggers cancellation flow instead of synthetic undo dispatch
  - stop-transition grace window remains in place
- Keep user feedback states:
  - `Undo applied`
  - `Nothing to undo`
  - `Undo expired`
- Keep discoverability hint behavior:
  - one-time hint after second successful tracked paste
  - persisted fields in `settings_store.json` key `undo_discoverability`: `has_seen_undo_hint`, `successful_paste_count`, `has_used_undo`
- Keep stats rollback semantics for transcribe-origin undo targets.
- Remove all undo-driven dictionary nudge systems:
  - Rust<->frontend heuristic bridge
  - alias/unresolved nudge evidence and gating
  - overlay nudge actions
  - dictionary prefill intent plumbing
  - undo-driven dictionary commands

## Default Shortcut Bindings
- macOS: `control+command+z`
- Windows: `ctrl+alt+z`
- Linux: `ctrl+alt+z`

## Impact
- **Affected specs**:
  - `shortcut-settings`
  - `observability`
- **Removed delta**:
  - `custom-word-correction` (undo-driven dictionary suggestions removed)
- **Affected code**:
  - `src-tauri/src/undo.rs`
  - `src-tauri/src/lib.rs`
  - `src/App.tsx`
  - `src/overlay/RecordingOverlay.tsx`
  - `src/overlay/RecordingOverlay.css`
  - `src/bindings.ts`
  - `src/i18n/locales/en/translation.json`
  - `doc/undo-paste-last-transcript.md`
  - `doc/undo-nudges-manual-test-checklist.md`
