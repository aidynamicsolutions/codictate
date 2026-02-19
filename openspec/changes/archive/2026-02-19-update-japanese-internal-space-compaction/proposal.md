# Change: Update Japanese Internal Space Compaction for Smart Insertion

## Why
Japanese ASR output often contains internal spaces between words, particles, and mixed-script boundaries (for example `私 は コーヒー を 飲みました。`). Native Japanese prose is normally written without those spaces, so pasting raw ASR output feels unnatural and low quality.

Current internal-space compaction only targets Chinese/Cantonese (`zh`, `zh-tw`, `yue`) and does not run for Japanese. As a result, Japanese smart insertion misses a high-frequency cleanup that users expect from dictation UX.

## What Changes
- Extend internal-space compaction language routing to include Japanese (`ja`) with a dedicated mixed-script strategy.
- Keep existing Han/CJK compaction behavior unchanged for Chinese/Cantonese (`zh`, `zh-tw`, `yue`).
- For Japanese compaction:
  - remove compactable spaces at `Japanese↔Japanese` boundaries,
  - remove compactable spaces at `ASCII↔Japanese` boundaries,
  - preserve `ASCII↔ASCII` phrase spacing (`Open AI`, URL-like tokens).
- Preserve structural whitespace (`\n`, `\r`, `\t`) and keep emoji-adjacent spacing unchanged in v1.
- Add Japanese unit tests and non-regression coverage for existing profile behavior.
- Update manual test checklist and maintainer notes.

## Impact
- **Affected specs**: `transcript-insertion`
- **Affected code**: `src-tauri/src/smart_insertion.rs`
- **Affected docs**:
  - `doc/test/smart-insertion-language-profiles-manual-test-checklist.md`
  - `doc/smart-insertion-notes.md`
- **Behavioral impact**:
  - Japanese transcript paste output removes ASR artifact spaces at natural Japanese boundaries.
  - Existing Chinese/Cantonese compaction behavior remains unchanged.
  - Existing punctuation normalization, casing transforms, boundary-spacing logic, and conservative fallback behavior remain unchanged.
