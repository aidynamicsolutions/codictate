# Change: Update CJK Internal-Space Compaction for Smart Insertion

## Why
ASR output for Chinese and Cantonese can include internal spaces inside sentence text (for example `是 請`). With current no-boundary-spacing behavior, these internal spaces are pasted verbatim, which feels non-native and breaks expected Chinese/Cantonese writing flow.

## What Changes
- Add a language-gated internal-space compaction step in smart insertion for normalized languages `zh`, `zh-tw`, and `yue`.
- Remove compactable internal whitespace only for CJK boundary pairs (Han/CJK punctuation on both sides).
- Preserve intentional ASCII phrase spacing (for example `Open AI`) to avoid mixed-script regressions.
- Preserve line breaks and tab-separated formatting.
- Add debug observability fields for compaction behavior in the existing smart insertion log event.
- Extend unit and manual test coverage for Chinese/Cantonese compaction and mixed-script preservation.

## Impact
- **Affected specs**: `transcript-insertion`
- **Affected code**: `src-tauri/src/smart_insertion.rs`
- **Affected docs**: `doc/test/smart-insertion-language-profiles-manual-test-checklist.md`
- **Behavioral impact**:
  - Chinese/Cantonese text no longer keeps ASR-internal spaces between CJK words/sentence units.
  - Auto/Turkish/unknown fallback behavior remains unchanged.
  - Existing boundary spacing profile behavior remains unchanged outside target languages.
