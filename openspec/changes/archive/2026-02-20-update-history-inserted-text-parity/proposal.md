# Change: Update History Inserted-Text Parity

## Why
History currently emphasizes raw/post-processed transcript fields but can diverge from what users actually received in target apps after smart insertion transforms. This reduces trust when users compare pasted output to history rows.

Users also need transparent access to original ASR output for debugging and AI refine behavior checks.

## What Changes
- Add additive history schema support for `inserted_text` without mutating existing raw data.
- Expose `effective_text` and `raw_text` in history entry API shape.
- Persist transformed `PasteResult.pasted_text` by exact history row id when a paste action succeeds.
- Keep `refine_last_transcript` input sourced from raw ASR and update the same latest row with refine output.
- Keep `correct_text` out of transcription history because it is not audio-backed.
- Update history UI to:
  - show effective text as primary,
  - provide inline `Original transcript` disclosure when raw differs,
  - copy primary/effective text,
  - surface raw-only search hints.
- Update i18n, docs, and tests for parity behavior.

## Impact
- **Affected specs**:
  - `transcript-insertion`
  - `transcription-history` (new capability)
- **Affected code**:
  - `src-tauri/src/managers/history.rs`
  - `src-tauri/src/actions.rs`
  - `src-tauri/src/tray.rs`
  - `src/components/shared/HistoryList.tsx`
  - `src/components/shared/historyDisplayUtils.ts`
  - `src/bindings.ts`
  - `src/i18n/locales/*/translation.json`
  - `doc/smart-insertion-notes.md`
  - `doc/test/smart-insertion-language-profiles-manual-test-checklist.md`
