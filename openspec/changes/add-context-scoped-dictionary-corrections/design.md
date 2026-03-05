## Context
Dictionary currently applies global entry logic after ASR text is produced. This is precision-first, but global aliases for ambiguous words can still create unwanted rewrites. Users need safer control over where a correction should apply.

## Goals
- Add optional context-scoped exact correction controls for ambiguous cases.
- Preserve existing behavior for all entries without scope.
- Keep matching deterministic and debuggable.
- Keep fuzzy protections unchanged.

## Non-Goals
- No decoder-time hotword/phrase-bias integration in this change.
- No ASR lattice/N-best rescoring.
- No breaking schema migration.

## Research-Informed Rationale
- [Google Speech Adaptation](https://cloud.google.com/speech-to-text/docs/adaptation-model): over-biasing common terms increases false positives; precision controls matter.
- [Azure Phrase List Guidance](https://learn.microsoft.com/en-us/azure/ai-services/speech-service/improve-accuracy-phrase-list): targeted phrase/domain context improves practical recognition outcomes.
- [Deepgram Keyterm Prompting](https://developers.deepgram.com/docs/keyterm): avoid generic keyterms; prefer specific terms for control.
- [AWS Custom Vocabulary](https://docs.aws.amazon.com/transcribe/latest/dg/custom-vocabulary.html): vocabulary customization has explicit constraints and is not unconstrained semantic correction.
- [Mondegreen](https://research.google/pubs/mondegreen-a-post-processing-solution-for-addressing-frequent-asr-miscorrections/): post-ASR correction is an appropriate layer for recurring, systematic misrecognitions.

## Data Model
Proposed additive `CustomWordEntry` shape:

```ts
interface ContextScope {
  prev_any?: string[];
  next_any?: string[];
  window_any?: string[];
}

interface CustomWordEntry {
  input: string;
  aliases?: string[];
  replacement: string;
  is_replacement: boolean;
  fuzzy_enabled?: boolean;
  context_scope?: ContextScope;
  ambiguity_level?: "low" | "high";
}
```

Notes:
- All new fields are optional.
- Scope checks evaluate normalized tokens.
- `ambiguity_level` is metadata for UX warning behavior; matching logic should not require it.

## Matching Order
Deterministic precedence:
1. Exact phrase replacement
2. Scoped exact canonical/alias
3. Unscoped exact canonical/alias
4. Fuzzy paths (current guardrails unchanged)

## Scope Evaluation
For a candidate token span:
- `prev_any`: pass when any listed token matches immediately previous normalized token.
- `next_any`: pass when any listed token matches immediately next normalized token.
- `window_any`: pass when any listed token appears within a bounded local context window around span.
- If a field is omitted, it does not constrain matching.
- Scoped candidate is accepted only when all present scope fields pass.

Fallback behavior:
- If scoped candidate fails, continue normal flow (including unscoped candidates).

## Ambiguity Metadata
`ambiguity_level` can be computed at save-time, runtime, or both. Initial phase recommendation:
- Save-time heuristic for common/high-risk single-word aliases marks `high`.
- UI shows warning and recommends phrase replacement or scoped rules.
- Do not hard-block save in Phase 1.

## Migration and Compatibility
- Existing `user_dictionary.json` entries remain valid.
- No required data migration because fields are additive and optional.
- Old entries (without `context_scope`/`ambiguity_level`) follow exactly current behavior.

## Logging and Diagnostics
Add reason-coded debug outcomes:
- `skip_scope_prev`
- `skip_scope_next`
- `skip_scope_window`
- `accept_scoped_exact`

These should be emitted in existing CustomWords diagnostics channels.

## Test Scenarios
1. Scoped positive:
- Input: `review the state changes`
- Rule: `state -> staged`, `context_scope.next_any = ["changes"]`
- Expected: `review the staged changes`

2. Scoped negative:
- Input: `I want the state change`
- Same scoped rule as above
- Expected: unchanged

3. Phrase precedence:
- Rules include phrase replacement `state changes -> staged changes`
- Also include generic unscoped alias `state -> staged`
- Expected: phrase replacement precedence is applied

4. Backward compatibility:
- Legacy entries without scope/ambiguity metadata
- Expected: exact current behavior

5. Safety regression:
- Existing guard scenarios such as `mode -> modal`
- Expected: no new overfire behavior

## Trade-offs and Risks
- Too-broad scope windows can still over-apply.
- Too-strict scopes can reduce recall.
- Overlapping scoped and unscoped entries add complexity.

Mitigations:
- deterministic precedence
- UI warnings for high ambiguity
- phrase-first guidance
- diagnostic logs for acceptance/rejection reasons
