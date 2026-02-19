# Change: Add Language Profiles for Smart Transcript Insertion

## Why
Smart insertion currently applies English-leaning heuristics too broadly. This can produce incorrect spacing/casing in languages with different writing systems while also missing some Unicode continuation patterns.

## What Changes
- Add deterministic language normalization from `selected_language` and route smart insertion through explicit language profiles.
- Introduce four profiles for smart insertion behavior:
  - `CasedWhitespace`
  - `UncasedWhitespace`
  - `NoBoundarySpacing`
  - `Conservative`
- Keep compatibility guarantees:
  - no API/command changes
  - same `append_trailing_space` setting key
  - same shared paste and undo paths
- Implement Unicode-safe continuation checks using `is_numeric()`.
- Preserve conservative fallback behavior:
  - context unavailable uses legacy trailing-space behavior (`text + " "`)
  - conservative profiles with context available only add trailing space for word-boundary continuation (no extra space before punctuation)
- Update Smart Insertion UI copy to explicitly disclose conservative fallback scope for `auto`, Turkish, unsupported languages, and missing context.
- Document that runtime behavior is source-of-truth in code (`smart_insertion.rs` for insertion logic and `settings.rs` for default prompt text), with prompt markdown kept as reference.

## Impact
- **Affected specs**:
  - `transcript-insertion`
- **Affected code**:
  - `src-tauri/src/smart_insertion.rs`
  - `src/i18n/locales/*/translation.json`
  - `src-tauri/resources/locales/*/translation.json`
- **Behavioral impact**:
  - safer defaults for unsupported/unknown language modes
  - profile-aware punctuation handling for Arabic/CJK punctuation
  - no boundary spacing for no-space script profile
