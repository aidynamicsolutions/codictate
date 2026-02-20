# Smart Insertion Notes

This document is a maintainer-facing reference for smart insertion behavior changes and validation status.

## Session Update (2026-02-20) - Refine Replace Fallback + UTF-16 Selection Safety

### Scope Completed

1. Refine-last replacement on macOS is now best-effort instead of hard-fail:
   - if `inserted_text` exists, Codictate attempts AX re-selection before paste.
   - if re-selection fails (or `inserted_text` is unavailable), refine paste still proceeds at current cursor/selection.
2. Refine history persistence is now aligned to actual paste success:
   - refine output commit to history row now requires `did_paste = true`.
   - skipped/failed refine paste no longer mutates `post_processed_text`/`inserted_text`.
3. AX text selection range conversion now uses UTF-16-safe offsets:
   - byte offsets from Rust string search are converted to UTF-16 code units before `AXSelectedTextRange` write.
   - prevents mis-selection for multibyte text (emoji/CJK/accents) in re-selection path.
4. Added regression tests covering UTF-16 selection range behavior and refine history commit gating.

### Verification Completed

- `cargo test --manifest-path src-tauri/Cargo.toml actions::tests`
- `cargo test --manifest-path src-tauri/Cargo.toml accessibility::macos::tests`
- `cargo test --manifest-path src-tauri/Cargo.toml`

## Session Update (2026-02-20) - Post-Process Punctuation Artifact Sanitizer

### Scope Completed

1. Added shared punctuation-artifact cleanup helper used by post-processing output normalization:
   - `collapse_spaced_punctuation_artifacts(...)` in smart insertion module.
2. Sanitizer now runs only when source text contains spoken punctuation cues:
   - `comma`, `period`, `question mark`, `full stop`,
   - `exclamation mark`, `exclamation point`,
   - `semicolon`, `colon`,
   - `hyphen`, `dash`, `en dash`, `em dash`.
3. Sanitizer is applied from shared post-processing paths (transcribe auto-refine and refine-last), while non-post-processed transcribe flows are unchanged.
4. Added cleanup coverage for spaced duplicate punctuation tokens, including dash variants:
   - sentence-vs-clause cleanup (for example `. ,` -> `,`)
   - spaced duplicate sentence punctuation (for example `. .` -> `.`)
   - spaced duplicate dash tokens (for example `- -` -> `-`, `– –` -> `–`, `— —` -> `—`).
5. Added guard coverage to avoid unsafe regressions:
   - preserve negative-number expressions (`x - -1`)
   - preserve CLI flags (`--help`)
   - avoid cue false positives in non-cue words (`dashboard`).

### Residual Low-Risk Note

1. Spaced ellipsis forms (for example `. . .`) are currently normalized down to a single period by the duplicate-period collapse rule.
2. This behavior is accepted as low-risk for now because the sanitizer is only cue-gated for spoken punctuation conversion cases.
3. If product direction requires preserving spaced ellipsis in refine output, add an explicit ellipsis guard before period dedupe.

### Verification Completed

- `cargo test --manifest-path src-tauri/Cargo.toml smart_insertion::tests`
- `cargo test --manifest-path src-tauri/Cargo.toml clipboard::tests`
- `cargo test --manifest-path src-tauri/Cargo.toml`

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

## Session Update (2026-02-19) - History Inserted-Text Parity

### Scope Completed

1. History entry model now stores `inserted_text` (nullable) alongside existing raw and post-processed fields.
2. History primary line now resolves from `effective_text` with deterministic fallback:
   - `inserted_text` -> `post_processed_text` -> `transcription_text`.
3. Raw ASR text is preserved as `raw_text` and shown on demand via inline `Original transcript` disclosure.
4. Transcribe flow now persists `inserted_text` from the exact `PasteResult.pasted_text` payload for the exact saved row id when paste succeeds.
5. Refine-last flow uses latest non-empty refined text (`post_processed_text`) as input when available, falls back to raw ASR for first-pass refine, and updates row `post_processed_text`/`inserted_text` only when refine paste succeeds.
6. Paste-last flow now reuses `effective_text` so re-paste behavior matches what users most recently got inserted.
7. Search now matches both primary effective text and raw ASR text.
8. Migration remains additive (`ALTER TABLE ... ADD COLUMN inserted_text TEXT`) with no destructive history rewrites.

## Session Update (2026-02-20) - Deterministic Paste-Last Mode

### Scope Completed

1. `paste_last_transcript` now defaults to literal replay: it pastes exactly History primary text (`effective_text`) without a second smart-insertion transform pass.
2. Added optional setting `paste_last_use_smart_insertion` to re-enable adaptive smart insertion for paste-last only.
3. Added Settings UI toggle in Advanced (near Paste Method) with user-facing copy explaining literal vs adaptive behavior.
4. Kept transcribe/refine paste behavior unchanged (still using adaptive smart insertion through shared paste flow).
5. Added backend tests for mode resolution, smart-vs-literal text preparation, and backward-compatible settings defaults.

### Behavior Notes

1. Default UX is deterministic replay for trust and predictability.
2. Adaptive paste-last remains available for users who prefer context-aware capitalization/spacing at insertion time.
3. If global Smart Insertion is disabled, adaptive paste-last falls back to non-transform behavior naturally.

## Maintenance Guidance

1. Keep this file for implementation notes and validation history.
2. Keep `doc/transcription-cleanup.md` focused on cleanup filters only.
3. When behavior changes, update this file for maintainer context.
4. When behavior changes, update `doc/test/smart-insertion-language-profiles-manual-test-checklist.md` for manual QA expectations.
5. Do not store transient git-status snapshots in this document.
