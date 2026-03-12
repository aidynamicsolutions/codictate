## MODIFIED Requirements

### Requirement: Shared Paste Flow Coverage
The system SHALL apply smart insertion formatting to transcript paste flows that use adaptive preparation, while supporting literal replay for `paste_last_transcript` by default, preserving exact multimodal payloads, and SHALL register the exact pasted payload for undo.

#### Scenario: Live voice-only transcription paste remains adaptive
- **WHEN** `transcribe` flow pastes a voice-only transcript output
- **THEN** shared smart insertion formatter is used

#### Scenario: Live multimodal transcription paste is rendered literally
- **WHEN** `transcribe` flow pastes a multimodal session output
- **THEN** the system renders the full multimodal payload
- **AND** pastes that rendered payload literally rather than adaptively rewriting the structured text

#### Scenario: Paste-last-transcript default is literal replay
- **WHEN** `paste_last_transcript` is triggered
- **AND** `paste_last_use_smart_insertion` is disabled or missing
- **THEN** shared paste utility uses literal preparation for paste-last
- **AND** pasted text equals the stored replay payload exactly

#### Scenario: Paste-last-transcript adaptive mode is opt-in for voice-only entries
- **WHEN** `paste_last_transcript` is triggered
- **AND** `paste_last_use_smart_insertion` is enabled
- **AND** the latest entry is voice-only
- **THEN** shared smart insertion formatter is used for paste-last

#### Scenario: Paste-last-transcript multimodal replay stays literal
- **WHEN** `paste_last_transcript` is triggered for a multimodal history entry
- **THEN** the system replays the exact rendered multimodal payload literally
- **AND** does not adaptively rewrite the structured payload even if adaptive paste-last mode is enabled

#### Scenario: Live post-processing strips multimodal sidecar before AI
- **WHEN** live post-processing runs for a multimodal session
- **THEN** the AI provider receives spoken transcript text only
- **AND** after generation the system re-renders the multimodal payload from the stored sidecar before paste

#### Scenario: Refine-last-transcript re-renders multimodal payload after AI
- **WHEN** `refine_last_transcript` runs for a multimodal history entry
- **THEN** the AI provider receives spoken transcript text only
- **AND** after generation the system re-renders the multimodal payload from the stored sidecar before paste

#### Scenario: Repeated refine uses latest refined spoken text
- **WHEN** `refine_last_transcript` is triggered for the latest history row
- **AND** that row has a non-empty `post_processed_text`
- **THEN** refine input uses that `post_processed_text`
- **AND** if no non-empty `post_processed_text` exists, refine input falls back to `raw_text`

#### Scenario: macOS refine replacement re-selection is best-effort
- **WHEN** `refine_last_transcript` runs on macOS
- **AND** latest row has non-empty `inserted_text`
- **THEN** system attempts AX re-selection of that `inserted_text` before paste
- **AND** if AX re-selection fails (or `inserted_text` is unavailable), refine paste continues at current cursor or selection with user feedback

#### Scenario: Refine history commit requires successful paste
- **WHEN** `refine_last_transcript` produces refined output
- **AND** paste is skipped or fails (`did_paste = false` or paste error)
- **THEN** latest row `post_processed_text` and `inserted_text` are not updated by that refine attempt

#### Scenario: Undo capture stores actual pasted text
- **WHEN** a transcript paste succeeds through the shared paste utility
- **THEN** the registered undo payload uses the `pasted_text` returned by that paste operation
- **AND** triggering configured `undo_last_transcript` reverses that exact paste as a single operation
