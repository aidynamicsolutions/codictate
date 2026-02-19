# Smart Insertion Notes

This document is a maintainer-facing reference for smart insertion behavior changes and validation status.

## Session Update (2026-02-19)

### Scope Completed

1. Added strategy-based internal space compaction routing:
   - `HanBoundaryOnly` for `zh`, `zh-tw`, and `yue`
   - `JapaneseMixedScript` for `ja`
2. Extended internal-space compaction boundaries for Japanese mixed-script text:
   - hiragana, katakana, katakana phonetic extensions
   - halfwidth katakana
   - Japanese iteration marks
   - existing Han and CJK punctuation coverage
3. Enabled Japanese internal-space compaction with guardrails:
   - compact `Japanese↔Japanese` boundaries
   - compact `ASCII↔Japanese` boundaries
   - preserve `ASCII↔ASCII` phrase spacing
   - preserve line breaks and tabs
   - preserve emoji-adjacent spacing
4. Kept no-boundary-spacing profile behavior unchanged outside compaction scope.
5. Preserved punctuation normalization, boundary punctuation conflict cleanup, casing transforms, and conservative fallback behavior.
6. Replaced the Japanese no-compaction regression test with Japanese compaction test coverage across mixed script, numeric, line-break, emoji, and halfwidth katakana scenarios.
7. Updated manual QA checklist with Japanese NBS cases and log verification criteria.
8. Added OpenSpec change set: `openspec/changes/update-japanese-internal-space-compaction/`.

### Verification Completed

- `cargo test --manifest-path src-tauri/Cargo.toml --lib smart_insertion::tests::`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `openspec validate update-japanese-internal-space-compaction --strict`
- `openspec validate --all --strict`

## Session Update (2026-02-18)

### Scope Completed

1. Sentence-boundary trailing space was added for whitespace profiles (`CasedWhitespace`, `UncasedWhitespace`) when preserved sentence punctuation is followed by a word boundary.
2. Conservative behavior (`auto`, `tr`, unsupported languages) was intentionally left unchanged.
3. No-boundary-spacing behavior (`zh`, `zh-tw`, `yue`, `ja`, `th`, `km`, `lo`, `my`, `bo`) was intentionally left unchanged.
4. CJK internal-space compaction coverage was kept for `zh`, `zh-tw`, and `yue`.
5. Smart Insertion tooltip copy was simplified and translated in frontend + tauri locale files.
6. Manual checklist expectation updated: `CWS-07` -> `start hello? Step`.
7. Manual checklist expectation updated: `CWS-08` -> `start e.g. step`.
8. Manual checklist expectation updated: `SEL-02` -> `start hello? Step`.
9. Manual checklist expectation updated: `UWS-04` -> `مرحبا e.g؟ سلام`.
10. Manual checklist guard case added: `CWS-13` for punctuation-only insert.
11. OpenSpec change added: `openspec/changes/update-smart-transcript-insertion-sentence-boundary-trailing-space/`.
12. Custom dictionary merge logic now deduplicates overlapping terminal sentence punctuation when replacement already ends with the same mark.
13. Alias-based replacement regression fixed for `e g` -> `e.g.` so output is `e.g. step` (not `e.g.. step`).
14. Conflicting terminal punctuation at whitespace-profile boundaries now prefers right-boundary punctuation (for example `hello?.` -> `hello.`).
15. Whitespace profiles now resolve sentence-vs-clause boundary conflicts by keeping right-boundary clause punctuation (for example `there.,` -> `there,`) with an abbreviation-period guard (`e.g.,` stays intact).

### Verification Completed

- `cargo test --lib preserves_trailing_punctuation_on_uppercase_continuation`
- `cargo test --lib preserves_abbreviation_like_internal_dot`
- `cargo test --lib conservative_profile`
- `cargo test --lib no_boundary_spacing_profile`
- `cargo test` (passed: `228 passed, 0 failed`)
- `openspec validate update-smart-transcript-insertion-sentence-boundary-trailing-space --strict`
- `openspec validate --all --strict`
- `openspec validate update-custom-word-punctuation-dedup --strict`
- `openspec validate update-smart-transcript-insertion-clause-boundary-punctuation-cleanup --strict`

## Maintenance Guidance

1. Keep this file for implementation notes and validation history.
2. Keep `doc/transcription-cleanup.md` focused on cleanup filters only.
3. When behavior changes, update this file for maintainer context.
4. When behavior changes, update `doc/test/smart-insertion-language-profiles-manual-test-checklist.md` for manual QA expectations.
5. Do not store transient git-status snapshots in this document.
