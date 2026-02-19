## 1. Specification
- [x] 1.1 Add `transcript-insertion` delta for whitespace-profile sentence-vs-clause boundary cleanup.
- [x] 1.2 Validate with `openspec validate update-smart-transcript-insertion-clause-boundary-punctuation-cleanup --strict`.

## 2. Implementation
- [x] 2.1 Add clause-boundary punctuation helper(s) in `src-tauri/src/clipboard.rs`.
- [x] 2.2 Add sentence-vs-clause boundary conflict cleanup branch for whitespace profiles.
- [x] 2.3 Add abbreviation-period guard for clause-boundary cleanup.
- [x] 2.4 Keep conservative and no-boundary-spacing behavior unchanged.
- [x] 2.5 Add debug reason strings for clause-boundary cleanup and abbreviation guard.

## 3. Validation
- [x] 3.1 Add/adjust unit tests for cased clause-boundary cleanup (`.,`, `?,`, `!;`).
- [x] 3.2 Add abbreviation-guard unit tests (`e.g.,`, `e.g.:`).
- [x] 3.3 Add uncased Arabic clause-boundary cleanup test.
- [x] 3.4 Add conservative + no-boundary regression tests for unchanged behavior.
- [x] 3.5 Run `cargo test --lib` targeted clipboard checks and full `cargo test`.

## 4. Documentation
- [x] 4.1 Update `doc/test/smart-insertion-language-profiles-manual-test-checklist.md` with clause-boundary QA rows.
- [x] 4.2 Update `doc/smart-insertion-notes.md` with clause-boundary conflict policy summary.

## 5. Final Validation
- [x] 5.1 Validate `update-smart-transcript-insertion-clause-boundary-punctuation-cleanup` with strict OpenSpec validation.
- [x] 5.2 Validate all OpenSpec changes with `openspec validate --all --strict`.
