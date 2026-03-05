# Change: Add Context-Scoped Dictionary Corrections

## Why
Users currently treat Dictionary as if it can always force ASR pronunciation outcomes. In practice, Dictionary is a post-recognition correction layer with precision-first guardrails. This mismatch leads to frustration in ambiguous cases such as `state` vs `staged`.

Two things are needed now:
- Align product/docs copy with actual behavior so expectations are correct.
- Propose a safe context-scoped correction model that improves ambiguous-word handling without causing global false-positive rewrites.

## What Changes

### Track A: Immediate expectation alignment (completed in this phase)
- Update user guides to clarify Dictionary behavior as post-ASR correction.
- Add ambiguous-word safety guidance.
- Reframe mispronunciation guidance to transcript-first correction.
- Update English microcopy to remove guaranteed-pronunciation implications.

### Track B: Context-scoped correction proposal (design-only in this phase)
- Add optional `context_scope` to dictionary entries:
  - `prev_any?: string[]`
  - `next_any?: string[]`
  - `window_any?: string[]`
- Add optional `ambiguity_level?: "low" | "high"` metadata for UX safety messaging.
- Matching precedence:
  1. Exact phrase replacement
  2. Scoped exact canonical/alias
  3. Unscoped exact canonical/alias
  4. Fuzzy (existing guardrails unchanged)
- Keep backward compatibility by making new fields optional.

## Industry Research Basis
- Google Speech adaptation emphasizes careful phrase weighting and avoiding over-biasing common terms: [Google Cloud Speech Adaptation](https://cloud.google.com/speech-to-text/docs/adaptation-model)
- Azure phrase lists show targeted domain phrases improve outcomes when applied intentionally: [Azure Phrase List](https://learn.microsoft.com/en-us/azure/ai-services/speech-service/improve-accuracy-phrase-list)
- Deepgram keyterm guidance warns against generic/common keyterms due to over-trigger risk: [Deepgram Keyterm Prompting](https://developers.deepgram.com/docs/keyterm)
- AWS custom vocabulary docs reinforce that vocabulary customization has scope/constraint limits: [AWS Transcribe Custom Vocabulary](https://docs.aws.amazon.com/transcribe/latest/dg/custom-vocabulary.html)
- Google Mondegreen frames post-ASR correction as a valid layer for recurrent mistakes: [Mondegreen Paper](https://research.google/pubs/mondegreen-a-post-processing-solution-for-addressing-frequent-asr-miscorrections/)

## Trade-offs
- Benefit: better handling for ambiguous misrecognitions without global alias side effects.
- Cost: additional schema and matching complexity.
- Risk: overly strict scope may reduce correction recall.
- Mitigation: phrase-first recommendation, optional scope fields, deterministic precedence, and explicit diagnostics.

## Impact
- Affected specs: `custom-word-correction`
- Affected docs: `doc/dictionary-user-guide.md`, `doc/custom-word-correction.md`
- Affected UI copy: `src/i18n/locales/en/translation.json`
- Future implementation touchpoints (not in this phase): matcher logic, dictionary schema validation, save-time ambiguity heuristics, UI scope editor/warnings.
