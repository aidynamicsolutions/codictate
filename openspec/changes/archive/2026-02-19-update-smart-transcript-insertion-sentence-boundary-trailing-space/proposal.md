# Change: Update Sentence-Boundary Trailing Space for Whitespace Profiles

## Why
Smart insertion currently preserves some terminal punctuation (`?`, `.`, `؟`) but can glue the inserted token to the next word (for example `hello?Step`, `e.g.step`). This feels unnatural in whitespace-based writing systems and creates readability issues.

## What Changes
- Add a sentence-boundary trailing-space rule for whitespace profiles (`CasedWhitespace`, `UncasedWhitespace`).
- When inserted text ends with preserved sentence punctuation and right boundary is word-like, append one trailing space.
- Require inserted text to start with a word-like character for this rule so punctuation-only inserts do not force spacing.
- Keep conservative fallback (`auto`, `tr`, unsupported) unchanged.
- Keep no-boundary-spacing profiles (for example Chinese/Cantonese/Japanese) unchanged.
- Extend unit tests and manual checklist expectations for the new behavior.

## Impact
- **Affected specs**: `transcript-insertion`
- **Affected code**: `src-tauri/src/smart_insertion.rs`
- **Affected docs**: `doc/test/smart-insertion-language-profiles-manual-test-checklist.md`
- **Behavioral impact**:
  - Whitespace profiles avoid glued sentence punctuation at right word boundaries.
  - No changes to conservative or no-boundary-spacing profile behavior.
