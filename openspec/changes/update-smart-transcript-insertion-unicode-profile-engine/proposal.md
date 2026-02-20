# Change: Expand Smart Insertion Coverage with Unicode-Aware Profile Engine

## Why
Current smart insertion behavior is safe but still over-conservative for many Whisper languages. The active profile mapping is code-based and intentionally narrow, which protects UX quality but leaves meaningful quality gaps for multilingual users. A staged upgrade is needed to improve coverage without regressing existing behavior.

## What Changes
- Deliver this as a staged change with explicit risk controls:
  - Medium effort (3-7 days): broader safe coverage
  - High effort (2-4 weeks): robust long-term engine
- Medium effort scope:
  - Expand language/profile routing to cover most Whisper languages using script-based grouping.
  - Keep conservative fallback for unresolved/ambiguous languages and uncertain contexts.
  - Add representative regression tests for each smart insertion profile.
- High effort scope:
  - Move language/profile and punctuation rules into a data-driven profile table.
  - Upgrade boundary heuristics to Unicode-aware token/word segmentation aligned with Unicode guidance.
  - Add golden fixtures plus privacy-safe telemetry for tuning and long-term quality control.

## Impact
- **Affected Specs**:
  - `transcript-insertion`
- **Affected Code (planned)**:
  - `src-tauri/src/smart_insertion.rs`
  - `src-tauri/src/clipboard.rs`
  - `src-tauri/src/settings.rs`
  - `src/i18n/locales/*/translation.json` (if user-facing copy changes are needed)
  - `src-tauri/resources/locales/*/translation.json` (if user-facing copy changes are needed)
- **Behavioral Impact**:
  - Better smart insertion quality for non-English languages with lower risk of spacing/casing regressions.
  - Stronger long-term maintainability through data-driven rules and objective regression harnesses.
  - Better observability for tuning conservative fallbacks and profile routing decisions over time.
