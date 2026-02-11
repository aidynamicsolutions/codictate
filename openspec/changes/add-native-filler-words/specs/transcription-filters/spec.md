## ADDED Requirements
### Requirement: Native Filler Word Removal
The system MUST remove common filler words specific to the language of the transcription.

#### Scenario: French transcription cleanup
- **WHEN** the user transcribes French speech containing "euh" and "ben"
- **THEN** the system removes "euh" and "ben" from the final text

#### Scenario: Japanese transcription cleanup
- **WHEN** the user transcribes Japanese speech containing "eto" (えっと) and "ano" (あの)
- **THEN** the system removes these filler words from the final text

#### Scenario: Fallback to English
- **WHEN** the language is not one of the supported 15 languages
- **THEN** the system defaults to English filler word removal or a safe empty list (to avoiding removing valid words in unknown languages)

### Requirement: Language Awareness
The cleaning filter MUST use the detected or selected language to determine which words to remove.

#### Scenario: Language-dependent filtering
- **WHEN** the same sound exists in multiple languages but is a filler in only one
- **THEN** it is only removed when processing that specific language
