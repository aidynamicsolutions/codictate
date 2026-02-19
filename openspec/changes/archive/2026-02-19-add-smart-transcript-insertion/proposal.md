# Change: Add Smart Transcript Insertion

## Why
Transcript paste currently applies a blunt trailing-space behavior and does not account for cursor context. This causes two recurring issues:
- mid-sentence replacements can be inserted with unwanted leading capitalization
- dictated words can be glued to adjacent text when no space exists at the insertion boundary

Users need insertion that respects surrounding sentence structure and word boundaries.

## What Changes
- Add smart insertion formatting for transcript paste on macOS using accessibility-derived cursor boundary context.
- Apply sentence-aware first-character casing:
  - capitalize at sentence/document start
  - de-capitalize mistaken title-case starts for mid-sentence insertion
  - preserve acronym-like uppercase tokens
- Apply boundary-aware spacing so inserted word-like text does not join adjacent word-like neighbors.
- Keep `append_trailing_space` as the existing setting key and command path, but change semantics to smart insertion enablement.
- Preserve legacy fallback (`text + " "`) when context cannot be captured while smart insertion is enabled.
- Keep all downstream paste behavior unchanged (paste methods, auto-submit, clipboard handling, undo tracking).

## Impact
- **Affected specs**:
  - `transcript-insertion` (new capability)
- **Affected code**:
  - `src-tauri/src/accessibility/mod.rs`
  - `src-tauri/src/accessibility/macos.rs`
  - `src-tauri/src/clipboard.rs`
  - `src/i18n/locales/en/translation.json`
- **Behavioral impact**:
  - macOS gains context-aware insertion.
  - Windows/Linux keep current trailing-space behavior under the same setting.
