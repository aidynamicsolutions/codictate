## MODIFIED Requirements

### Requirement: Local AI Text Generation
The system SHALL support text generation using locally loaded MLX models via a Python sidecar for transcription post-processing.

#### Scenario: Process transcription with local model
- **WHEN** the user has post-processing enabled with the local MLX provider selected
- **AND** a transcription is completed
- **THEN** the system SHALL process the transcription through the MLX sidecar using the selected prompt with proper chat template formatting

#### Scenario: Sidecar auto-start
- **WHEN** the user triggers transcription with local MLX post-processing
- **AND** the Python sidecar is not running
- **THEN** the system SHALL spawn the sidecar via uv run
- **AND** wait for the sidecar to be ready before proceeding

#### Scenario: Model not loaded (on-demand loading)
- **WHEN** the user attempts to process text but no MLX model is loaded
- **THEN** the system SHALL load the selected model on-demand before processing
- **OR** fallback to the original transcription if loading fails

#### Scenario: Multimodal session excludes sidecar metadata from model input
- **WHEN** local post-processing runs for a multimodal session
- **THEN** the MLX provider receives spoken transcript text only
- **AND** the multimodal context block, inline figure markers, footer paths, and sidecar metadata are excluded from the model input

#### Scenario: Multimodal session reattaches sidecar metadata after generation
- **WHEN** the MLX provider returns text for a multimodal session
- **THEN** the system re-renders the final multimodal payload from the generated spoken text plus stored sidecar metadata

#### Scenario: Generation failure
- **WHEN** text generation fails
- **THEN** the system SHALL log an error and return the original transcription unchanged

#### Scenario: Model not downloaded
- **WHEN** the user attempts to process text but the selected model is not downloaded
- **THEN** the system SHALL return the original transcription unchanged
- **AND** log a warning about the missing model

#### Scenario: Sidecar timeout
- **WHEN** the Python sidecar fails to start within 60 seconds
- **THEN** the system SHALL log an error and return the original transcription unchanged

#### Scenario: Chat template formatting
- **WHEN** generating text with Qwen3 models
- **THEN** the system SHALL apply the model's chat template with enable_thinking=False
- **AND** format the prompt as a ChatML message structure

#### Scenario: Repetition penalty
- **WHEN** generating text
- **THEN** the system SHALL apply repetition penalty to prevent model output loops
- **AND** use logits processors to penalize repeated tokens

#### Scenario: Response cleaning
- **WHEN** the model returns a response
- **THEN** the system SHALL strip any thinking tags and special tokens from the output
- **AND** remove duplicate "Output:" or "Text:" patterns if model is looping
- **AND** handle empty responses gracefully
