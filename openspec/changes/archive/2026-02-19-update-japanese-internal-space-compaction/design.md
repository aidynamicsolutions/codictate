## Context
Smart insertion already compacts internal ASR spaces for `zh`, `zh-tw`, and `yue` using a Han/CJK-boundary heuristic. Japanese currently uses the `NoBoundarySpacing` profile but skips internal compaction.

This creates poor UX for Japanese dictation because Whisper output frequently inserts whitespace around particles and mixed-script boundaries.

## Goals / Non-Goals
- Goals:
  - Make Japanese smart insertion output look natural to native readers by removing ASR artifact spaces.
  - Keep behavior deterministic, fast (single pass), and dependency-free.
  - Avoid regressions for Chinese/Cantonese compaction and other smart insertion logic.
- Non-Goals:
  - Add morphology/tokenizer dependencies (MeCab/Sudachi/lindera).
  - Rework sentence punctuation/casing/boundary-spacing systems.
  - Add user-facing toggles for compaction strategy.

## Research Summary
- Japanese text is normally written without inter-word spaces; ASR pipelines commonly require post-processing cleanup.
- Inter-script spacing in Japanese is typically treated as typography/layout behavior rather than mandatory plain-text spacing.
- For dictation UX, preserving `ASCII↔ASCII` spacing while compacting Japanese boundaries best balances readability and naturalness.

References used:
- W3C Japanese Layout Requirements (JLREQ)
- JTF Japanese translation style guidance
- Whisper community discussions on Japanese post-processing

## Decisions
- Decision: Introduce a strategy-based internal compaction router.
  - `HanBoundaryOnly` for `zh`, `zh-tw`, `yue`
  - `JapaneseMixedScript` for `ja`
  - preserve existing call site and telemetry fields
- Decision: Keep current Han/CJK behavior unchanged under `HanBoundaryOnly`.
- Decision: Implement Japanese boundaries with script ranges, not morphology.
  - Include:
    - hiragana `U+3040..=U+309F`
    - katakana `U+30A0..=U+30FF`
    - katakana extensions `U+31F0..=U+31FF`
    - halfwidth katakana `U+FF66..=U+FF9F`
    - iteration marks `U+3005`, `U+303B`
    - existing Han + CJK punctuation
- Decision: Japanese compaction rule
  - compact when whitespace run is compactable and boundary is `Japanese↔Japanese` or `ASCII↔Japanese`
  - preserve `ASCII↔ASCII`
  - preserve emoji-adjacent spacing in v1
- Decision: Keep structural whitespace guard unchanged (`\n`, `\r`, `\t` are never compacted).

## Alternatives Considered
- Approach A: Extend current boundary set directly for all CJK languages.
  - Rejected because it couples Japanese mixed-script heuristics to Han-only behavior used by Chinese/Cantonese.
- Approach B: Strip all spaces except ASCII↔ASCII.
  - Rejected because it is too aggressive around symbol/emoji and unclear boundaries.
- Approach C: Morphological re-analysis.
  - Rejected for this change due dependency, dictionary, binary size, and latency cost.

## Risks / Trade-offs
- Risk: Over-compaction around unusual symbols.
  - Mitigation: keep emoji-adjacent spacing unchanged; add explicit tests.
- Risk: Missing rare Japanese code points.
  - Mitigation: include common ranges + iteration marks; extend in follow-up if needed.
- Risk: Regression in existing CJK behavior.
  - Mitigation: isolate by strategy enum and retain existing tests as merge gates.

## Performance
- Keep O(n) single-pass scan over char buffer.
- No new runtime dependency or dictionary load.
- Preserve existing allocation pattern (`String::with_capacity(text.len())`).

## Test Strategy
- Add Japanese compaction tests for mixed-script, numeric-unit, particle boundaries, line breaks, emoji spacing, and halfwidth katakana.
- Preserve and rerun all existing compaction/profile tests.
- Require full `clipboard::tests` and `--lib` pass before merge.

## Migration Plan
- No config/data migration needed.
- Rollout is code-path-only and language-gated by normalized language key.
