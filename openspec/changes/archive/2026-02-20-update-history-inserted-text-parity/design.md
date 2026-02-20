## Context
History rows are audio-backed and already store raw ASR text plus optional post-processed text. Smart insertion can transform pasted text at insertion time, so users may see text in target apps that does not exactly match what history displays today.

## Goals / Non-Goals
- Goals:
  - Preserve all existing history data with additive migration only.
  - Make primary history text match inserted target-app text when known.
  - Preserve original ASR visibility for transparency.
  - Keep refine-last tied to raw input while reflecting refined results in the same row.
- Non-Goals:
  - Persist non-audio `correct_text` operations in audio history.
  - Change smart insertion heuristics.
  - Add multi-version refine timelines.

## Decisions
- Decision: Add nullable `inserted_text` to `transcription_history`.
  - Reason: supports exact inserted parity without destructive backfill.
- Decision: Define `effective_text` fallback as `inserted_text -> post_processed_text -> transcription_text`.
  - Reason: deterministic display and reuse behavior across backend/frontend/tray.
- Decision: Persist inserted text by exact `entry_id` returned from save operation.
  - Reason: avoids race-prone “latest row” heuristics.
- Decision: Keep `refine_last_transcript` input as raw ASR text and update same latest row on successful refine output.
  - Reason: preserves existing refine semantics and user expectation.
- Decision: Use inline progressive disclosure (`Original transcript`) instead of hover-only details.
  - Reason: accessibility and compact list UX.

## Migration Plan
1. Apply expand-only migration:
   - `ALTER TABLE transcription_history ADD COLUMN inserted_text TEXT;`
2. Keep all existing columns/data unchanged.
3. Read paths compute effective fallback for legacy rows where `inserted_text` is null.
4. No data rewrite/backfill required.

## Risks / Trade-offs
- Save-then-paste ordering adds slight latency before insertion in transcribe flow.
  - Mitigation: keep operations minimal and preserve undo/paste behavior.
- Localized labels added across many locales may need iterative copy polish.
  - Mitigation: keep wording short, non-technical, and brand-consistent.
