## MODIFIED Requirements
### Requirement: Shared Paste Flow Coverage
The system SHALL apply smart insertion formatting to all transcript paste flows that route through the shared paste utility, register transformed paste payloads for undo, and persist transformed inserted text for the exact associated history row when applicable.

#### Scenario: Live transcription paste
- **WHEN** `transcribe` flow pastes transcript output
- **THEN** shared smart insertion formatter is used

#### Scenario: Paste-last-transcript shortcut
- **WHEN** `paste_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Refine-last-transcript shortcut
- **WHEN** `refine_last_transcript` flow pastes text
- **THEN** shared smart insertion formatter is used

#### Scenario: Undo capture stores transformed pasted text
- **WHEN** a transcript paste succeeds through the shared paste utility
- **THEN** the registered undo payload uses the transformed `pasted_text` value returned by the paste operation
- **AND** triggering configured `undo_last_transcript` reverses that transformed paste as a single operation

#### Scenario: Transcribe persists inserted text by exact row id
- **WHEN** `transcribe` saves a history entry and paste succeeds (`did_paste = true`)
- **THEN** transformed `PasteResult.pasted_text` is stored in `inserted_text` for that exact saved entry id
- **AND** implementation does not rely on latest-row lookup heuristics

#### Scenario: Refine-last keeps raw input and updates same row
- **WHEN** `refine_last_transcript` runs on latest history entry
- **THEN** refine input is latest row raw ASR text
- **AND** refine output updates `post_processed_text` for that same row id

#### Scenario: Refine-last inserted text update is paste-success-only
- **WHEN** `refine_last_transcript` paste succeeds (`did_paste = true`)
- **THEN** transformed `PasteResult.pasted_text` updates `inserted_text` for the same row id
- **AND** if paste is skipped or fails, `inserted_text` is not overwritten
