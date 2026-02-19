## Context
The dictionary matcher currently supports one `input` phrase per entry and uses exact/fuzzy matching over normalized n-grams. This works well for direct matches (`chat gpt -> chatgpt`) but fails for many OOV brand/tech terms where ASR emits partially similar split tokens (`shat cn`, `chef cn`).

Additionally, troubleshooting is difficult because logs do not provide reason-coded rejections with aggregate session summaries.

Current ASR interfaces in this project return only a single transcription hypothesis (`text`) plus segments. They do not expose alternatives/N-best in a cross-engine way.

## Goals / Non-Goals
- **Goals**
  - Add canonical term + aliases modeling to represent real spoken variants.
  - Improve split-token matching for single-token targets using strict constraints.
  - Keep false positives low with conservative guards.
  - Add deterministic logging for match decisions.
  - Provide clear user guide examples and troubleshooting.
  - Add a forward-compatible shape for future rescoring hooks.
- **Non-Goals**
  - Full N-best/lattice rescoring implementation in this change.
  - Cross-engine ASR decoder changes.
  - Backward compatibility for legacy dictionary entry schema.

## Decisions
- **Decision 1: Canonical + aliases data model**
  - Add `aliases: Vec<String>` / `aliases: string[]` to each dictionary entry.
  - Match candidates are built from `input + aliases`.
  - Canonical form remains `input` to minimize UX churn.

- **Decision 2: Constrained split-token fuzzy path**
  - Allow fuzzy matching from 2-3 word n-grams to single-token dictionary targets.
  - Require additional guards:
    - normalized candidate length >= 5
    - not all tokens are stop words
    - existing length ratio guard
    - strict split threshold (`word_correction_split_threshold`)
  - Keep exact matching precedence over all fuzzy paths.

- **Decision 3: Structured observability**
  - Emit reason-coded debug logs for acceptance/rejection decisions.
  - Emit per-session summary counters at info level.
  - Include fields for path, score, threshold, n-gram, and source form (canonical/alias).

- **Decision 4: Rescoring readiness without behavior change**
  - Introduce an internal hypothesis abstraction in correction pipeline.
  - Process the primary hypothesis only for now.
  - Keep API and behavior stable until alternatives are available across all engines.

- **Decision 5: No legacy dictionary compatibility**
  - Existing legacy dictionary entries (without aliases) are unsupported by design.
  - On parse failure, settings are reset via existing fallback behavior.

## Risks / Trade-offs
- **Risk**: Split-token fuzzy path increases false positives.
  - **Mitigation**: Strict split threshold, guard-word constraints, and reason-coded logs.
- **Risk**: No legacy migration can reset settings for developer/test installs with old dictionary shape.
  - **Mitigation**: Explicitly documented in requirements and change notes.
- **Risk**: Rescoring deferred may leave some hard terms unresolved.
  - **Mitigation**: Alias support covers most practical failures now; add rescoring hook for future.

## Migration Plan
- No schema migration is provided.
- Legacy settings that cannot deserialize under the new dictionary schema will fall back to defaults using existing behavior.

## Testing Strategy
- Unit tests for alias exact matching and punctuation retention.
- Unit tests for split-token fuzzy success and false-positive rejection.
- Regression tests for existing `chat gpt` behavior and stop-word guards.
- Log assertions for reason codes on representative paths.

## External Guidance
- Cloud ASR systems consistently separate canonical display forms from spoken variants (aliases/sounds-like), which informed the `input + aliases + replacement` model.
- Cloud ASR systems that expose alternatives/N-best enable second-pass rescoring; this change only adds internal readiness hooks because active engines in this app do not expose a unified alternatives interface yet.
