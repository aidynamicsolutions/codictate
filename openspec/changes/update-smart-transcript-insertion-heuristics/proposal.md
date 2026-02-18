# Change: Refine Smart Transcript Insertion Heuristics

## Why
Initial smart insertion improved capitalization and word-boundary spacing, but two UX issues remain:
- ASR can return trailing sentence punctuation (`.?!`) when replacing a mid-sentence selection, creating incorrect punctuation.
- Inserting a word immediately after sentence-ending punctuation without an existing space can still produce joined text (for example `working.What`).

## What Changes
- Extend insertion context with right-side non-whitespace boundary metadata.
- Add conservative punctuation sanitation for selection-replace flow:
  - candidate punctuation: `.`, `?`, `!`
  - strip only when right continuation is lowercase letter or digit
  - preserve abbreviation-like tokens with internal dots.
- Add leading-space insertion when word-like text is inserted directly after sentence punctuation (`.?!`) with no whitespace.
- Add observability fields for punctuation-strip and leading-space reason decisions.

## Impact
- **Affected specs**:
  - `transcript-insertion`
- **Affected code**:
  - `src-tauri/src/accessibility/mod.rs`
  - `src-tauri/src/accessibility/macos.rs`
  - `src-tauri/src/clipboard.rs`
  - `src/i18n/locales/en/translation.json`
- **Behavioral impact**:
  - Better mid-sentence replacement quality with fewer accidental trailing punctuation artifacts.
  - Better sentence-boundary spacing after `.?!` when cursor has no whitespace separation.
