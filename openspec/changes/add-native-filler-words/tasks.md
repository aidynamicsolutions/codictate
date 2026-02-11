## 1. Implementation
- [ ] 1.1 Research filler words for all supported languages (See `design.md`)
- [ ] 1.2 Define language-specific filler word lists in `src-tauri/src/audio_toolkit/text.rs`
- [ ] 1.2 Update `filter_transcription_output` signature to accept `language_code: &str`
- [ ] 1.3 Implement logic to select the correct filler word list based on the language code
- [ ] 1.4 Update `managers/transcription.rs` to pass the detected language code to `filter_transcription_output`
- [ ] 1.5 Add unit tests for each language to verify filler word removal
- [ ] 1.6 Update documentation in `doc/transcription-cleanup.md` to reflect native support
