# Change: Fix Smart Transcript Insertion Edge Cases

## Why
Smart insertion already improves sentence-aware casing and spacing, but three edge cases still create undesirable output:
- no-selection inserts can keep trailing `.?!` before lowercase/digit continuation,
- boundary interactions can produce duplicate terminal punctuation (`..`, `??`, `!!`),
- undo tracking must remain explicitly aligned to the transformed pasted text.

## What Changes
- Extend trailing `.?!` sanitation to run for both selection and no-selection insertions when continuation is lowercase/digit.
- Add conservative duplicate punctuation collapse for matching boundary `.`, `?`, `!`.
- Preserve existing casing and spacing logic order, while inserting new punctuation normalization before spacing.
- Keep undo shortcut behavior unchanged (`undo_last_transcript`) and ensure undo capture uses transformed paste payload.
- Update Smart Insertion English description to reflect continuation cleanup and duplicate boundary collapse.

## Impact
- **Affected specs**:
  - `transcript-insertion`
- **Affected code**:
  - `src-tauri/src/clipboard.rs`
  - `src-tauri/src/actions.rs`
  - `src/i18n/locales/en/translation.json`
- **Behavioral impact**:
  - Fewer awkward sentence breaks from no-selection continuation punctuation.
  - No duplicate `.?!` at insertion boundaries.
  - Undo remains one-step reversal of the transformed transcript paste.
