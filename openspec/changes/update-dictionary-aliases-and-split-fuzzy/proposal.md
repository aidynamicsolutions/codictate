# Change: Update Dictionary Aliases and Split-Token Matching

## Why
Custom dictionary matching is currently unreliable for hard out-of-vocabulary terms that ASR often splits or distorts (for example, `shadcn` becoming `shat cn` or `chef cn`). Users can only define one spoken form per entry, and the current fuzzy logic rejects many split-token variants for single-token dictionary entries.

This change aligns the dictionary feature with industry practice: canonical term + spoken aliases, constrained matching for split-token variants, and stronger observability so failures can be debugged deterministically.

## What Changes
- Add alias support to dictionary entries using a canonical + aliases data model.
- Add constrained split-token fuzzy matching for 2-3 word n-grams mapped to single-token dictionary terms.
- Add a dedicated split-fuzzy threshold setting.
- Add structured reason-coded logs and per-session matching summaries.
- Add documentation with concrete setup examples and troubleshooting guidance.
- Add explicit rescoring readiness hooks while deferring full N-best/lattice rescoring until all active ASR engines expose alternatives.
- Intentionally do not support legacy dictionary entry schema migration.

## Impact
- **Affected specs**:
  - `custom-word-correction`
  - `observability`
- **Affected code**:
  - `src-tauri/src/settings.rs`
  - `src-tauri/src/audio_toolkit/text.rs`
  - `src-tauri/src/managers/transcription.rs`
  - `src/components/dictionary/DictionaryEntryModal.tsx`
  - `src/components/dictionary/DictionaryPage.tsx`
  - `src/utils/dictionaryUtils.ts`
  - `src/bindings.ts`
  - `doc/custom-word-correction.md`
- **Behavioral impact**:
  - Improved reliability for difficult words such as `shadcn`.
  - Existing settings with legacy dictionary entry shape are treated as unsupported and replaced by defaults.

## Best-Practice References
- Google Speech adaptation guidance: https://cloud.google.com/speech-to-text/docs/adaptation-model
- Google alternatives (`maxAlternatives`): https://cloud.google.com/speech-to-text/docs/reference/rest/v1/RecognitionConfig
- AWS custom vocabulary tables (`Phrase`, `SoundsLike`, `DisplayAs`): https://docs.aws.amazon.com/transcribe/latest/dg/custom-vocabulary-create-table.html
- AWS alternatives: https://docs.aws.amazon.com/transcribe/latest/dg/alternatives.html
- Azure phrase list and weight guidance: https://learn.microsoft.com/en-us/azure/ai-services/speech-service/improve-accuracy-phrase-list
- Deepgram keyterm guidance: https://developers.deepgram.com/docs/keyterm
- Kaldi lattice foundations: https://kaldi-asr.org/doc/lattices.html
