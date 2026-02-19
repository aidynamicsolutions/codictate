# Change: Update Custom Word Punctuation Merge Deduplication

## Why
Custom word replacement currently preserves suffix punctuation from the matched transcript segment. When the configured replacement already ends with the same sentence punctuation, output can duplicate punctuation (for example `e.g..`), which is poor UX and looks like a typo.

## What Changes
- Add a punctuation-merge guard in custom word correction to trim overlapping identical terminal sentence punctuation from the matched suffix.
- Apply this deduplication in both exact and fuzzy match paths.
- Keep punctuation preservation for non-duplicate suffix punctuation unchanged.
- Add regression tests for alias-based replacement and mixed punctuation behavior.

## Impact
- **Affected specs**: `custom-word-correction`
- **Affected code**: `src-tauri/src/audio_toolkit/text.rs`
- **Behavioral impact**:
  - Prevents duplicated sentence punctuation when replacement already includes the same terminal mark.
  - Keeps existing punctuation preservation behavior for non-duplicate suffix punctuation.
