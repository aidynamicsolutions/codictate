# Change: Update Smart Insertion Boundary Punctuation Priority

## Why
Smart insertion currently collapses duplicate boundary punctuation only when inserted and right-boundary marks are identical. When they differ (for example `hello?` before `.`), output can become `hello?.`, which feels awkward and non-native for most users.

## What Changes
- Add boundary conflict handling for whitespace profiles (`CasedWhitespace`, `UncasedWhitespace`).
- When inserted terminal punctuation conflicts with existing right-boundary terminal punctuation, keep the right-boundary mark by stripping one inserted terminal mark.
- Keep existing same-mark duplicate collapse behavior unchanged.
- Keep conservative (`auto`, `tr`, unsupported) and no-boundary-spacing profile behavior unchanged.
- Add focused regression tests and checklist coverage.

## Impact
- **Affected specs**: `transcript-insertion`
- **Affected code**: `src-tauri/src/smart_insertion.rs`
- **Affected docs**:
  - `doc/test/smart-insertion-language-profiles-manual-test-checklist.md`
  - `doc/smart-insertion-notes.md`
