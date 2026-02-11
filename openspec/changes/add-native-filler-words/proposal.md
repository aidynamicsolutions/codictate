# Change: Add Native Filler Word Support

## Why
Currently, the Filler Word Removal filter only removes English filler words ("um", "uh", "hmm", etc.) regardless of the spoken language. This leads to a poor user experience for non-English speakers, as their native hesitation sounds are preserved in the transcription.

## What Changes
- Updates the `AudioToolkit` text processing logic to support language-specific filler word lists.
- Modifies `filter_transcription_output` to accept a language code parameter.
- Adds comprehensive filler word lists for all 15 supported languages (Arabic, Czech, German, Spanish, French, Italian, Japanese, Korean, Polish, Portuguese, Russian, Turkish, Ukrainian, Vietnamese, Chinese).
- Updates the call sites in `transcription.rs` (and potentially `lib.rs`) to pass the detected language code to the filter.

## Impact
- **Affected Specs**: `transcription-filters`
- **Design Document**: `openspec/changes/add-native-filler-words/design.md` (contains research findings)
- **Affected Code**: 
    - `src-tauri/src/audio_toolkit/text.rs` (core logic)
    - `src-tauri/src/managers/transcription.rs` (integration)
- **User Experience**: Non-English users will see significantly cleaner transcriptions with native filler words removed.
