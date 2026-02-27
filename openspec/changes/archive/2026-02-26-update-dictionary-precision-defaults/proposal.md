# Change: Dictionary Precision Defaults and Fuzzy Opt-In Hardening

## Why
The current dictionary behavior allows fuzzy matching by default for vocabulary entries. In practice this can create high-impact false positives (for example, `went -> qwen`) that reduce trust in transcription output.

Industry guidance for speech adaptation and custom lexicons consistently recommends constrained biasing/replacement behavior and caution with aggressive matching, especially on short/common words.

This change makes dictionary behavior precision-first: exact canonical/alias matching by default, explicit fuzzy opt-in, and a hard safety guard for short single-token targets.

## What Changes
- Introduce per-entry `fuzzy_enabled` as explicit opt-in state.
- Enforce a hard guard that blocks single-word fuzzy matching for normalized character length `<= 4` (canonical or active alias target).
- Keep exact canonical and exact alias matching as first-class/default behavior.
- Keep global fuzzy thresholds unchanged (`word_correction_threshold = 0.18`, `word_correction_split_threshold = 0.14`).
- Add deterministic legacy migration logic so existing entries are upgraded safely:
  - Legacy entries store `fuzzy_enabled = None` at load time.
  - Legacy non-replacement entries with single-word normalized canonical character length `<= 4` migrate to `Some(false)`.
  - All other legacy non-replacement entries migrate to `Some(true)`.
  - Replacement entries always enforce `Some(false)`.
  - Any short single-word canonical entry with `Some(true)` is coerced to `Some(false)`.
- Update dictionary UI to present fuzzy as an advanced toggle defaulting to OFF.
- Update dictionary documentation to describe exact+aliases default and short-word fuzzy blocking.

## Impact
- **Affected specs**:
  - `custom-word-correction`
- **Affected code**:
  - `src-tauri/src/settings.rs`
  - `src-tauri/src/audio_toolkit/text.rs`
  - `src-tauri/src/dictionary_normalization.rs` (new)
  - `src/components/dictionary/DictionaryEntryModal.tsx`
  - `src/utils/dictionaryUtils.ts`
  - `src/i18n/locales/en/translation.json`
  - `doc/dictionary-user-guide.md`
  - `doc/custom-word-correction.md`
  - `src/bindings.ts` (generated)
- **Behavioral impact**:
  - Eliminates common short-word fuzzy false positives.
  - Preserves exact canonical/alias reliability.
  - Preserves long-word fuzzy behavior only when explicitly enabled.

## Best-Practice References
- Google Speech adaptation caution about over-biasing: https://cloud.google.com/speech-to-text/ondevice/docs/model_adaptation
- Azure phrase list weighting guidance: https://learn.microsoft.com/en-us/azure/ai-services/speech-service/improve-accuracy-phrase-list
- AWS custom vocabulary mapping (`Phrase`/`DisplayAs`) guidance: https://docs.aws.amazon.com/transcribe/latest/dg/custom-vocabulary.html
- Deepgram find-and-replace explicit mapping: https://developers.deepgram.com/docs/find-and-replace
- AssemblyAI custom spelling explicit mapping: https://www.assemblyai.com/docs/guides/custom-spelling
- Elasticsearch fuzziness AUTO (short-token strictness reference): https://www.elastic.co/docs/reference/elasticsearch/rest-apis/common-options#fuzziness
