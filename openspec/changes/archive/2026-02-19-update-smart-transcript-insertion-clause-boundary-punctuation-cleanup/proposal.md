# Change: Update Smart Transcript Insertion Clause-Boundary Punctuation Cleanup

## Why
Whitespace-profile smart insertion can currently preserve an inserted terminal sentence mark directly before existing clause punctuation at the cursor boundary, producing awkward text like `there.,` or `hello?,`.

## What Changes
- Add clause-boundary conflict cleanup for whitespace profiles (`CasedWhitespace`, `UncasedWhitespace`).
- When inserted text ends with sentence punctuation and the immediate right boundary is clause punctuation, keep the right-boundary clause mark and strip one inserted terminal sentence mark.
- Preserve abbreviation-like period endings (for example `e.g.` and `U.S.`) before clause punctuation to avoid over-stripping.
- Keep `Conservative` and `NoBoundarySpacing` behavior unchanged.
- Add regression tests and update manual QA notes/checklist.

## Impact
- **Affected specs**: `transcript-insertion`
- **Affected code**: `src-tauri/src/smart_insertion.rs`
- **Affected docs**: `doc/test/smart-insertion-language-profiles-manual-test-checklist.md`, `doc/smart-insertion-notes.md`
- **Behavioral impact**:
  - Fixes user-facing boundary outputs such as `there.,` -> `there,` for whitespace profiles.
  - Preserves expected abbreviation behavior such as `e.g.,`.
