# local-ai-inference Specification

## Purpose
TBD - created by archiving change add-mlx-local-ai-provider. Update Purpose after archive.
## Requirements
### Requirement: Local MLX Model Management
The system SHALL provide local AI model management for Apple Silicon Macs, allowing users to download, load, and delete MLX-based language models for on-device transcription enhancement.

#### Scenario: Canonical model IDs
- **WHEN** MLX models are defined in the catalog
- **THEN** each model SHALL have a stable canonical ID for long-term compatibility

#### Scenario: Alias-based compatibility
- **WHEN** the persisted selected model ID matches a known alias
- **THEN** the system SHALL resolve it to the canonical model ID
- **AND** SHALL persist the canonical model ID back to settings

#### Scenario: Invalid selected model fallback
- **WHEN** the persisted selected model ID is missing or invalid
- **THEN** the system SHALL select a deterministic fallback model
- **AND** SHALL persist the fallback selection
- **AND** SHALL continue processing without user-facing hard failure

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

#### Scenario: Generation failure
- **WHEN** text generation fails
- **THEN** the system SHALL log the error and return the original transcription unchanged

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

---

### Requirement: Model Loading and Memory Management
The system SHALL manage MLX model memory efficiently, loading models on-demand and unloading when not needed.

#### Scenario: On-demand model loading
- **WHEN** a user triggers transcription with local MLX post-processing selected
- **AND** no model is currently loaded
- **THEN** the system SHALL load the selected model into GPU memory

#### Scenario: Model caching
- **WHEN** a model is loaded
- **THEN** the system SHALL keep the model in memory for subsequent requests until explicitly unloaded or timeout

#### Scenario: Unload model on timeout
- **WHEN** the MLX model has been idle for the configured model unload timeout period (same setting as transcription engine)
- **THEN** the system SHALL unload the model to free GPU memory

#### Scenario: Model switch while loaded
- **WHEN** the user selects a different model while one is currently loaded
- **THEN** the system SHALL unload the current model
- **AND** reset the model state for the new selection

---

### Requirement: Error Handling and Recovery
The system SHALL handle errors gracefully and provide meaningful feedback to users.

#### Scenario: Network error during download
- **WHEN** a network error occurs during model download
- **THEN** the system SHALL display a user-friendly error message
- **AND** allow the user to retry the download

#### Scenario: Disk space error
- **WHEN** there is insufficient disk space to download a model
- **THEN** the system SHALL display an appropriate error message
- **AND** not leave partial downloads on disk

#### Scenario: Model loading failure
- **WHEN** a downloaded model fails to load (corrupted files, incompatible format)
- **THEN** the system SHALL display an error message
- **AND** allow the user to delete and re-download the model

#### Scenario: Empty model directory cleanup
- **WHEN** the system detects an empty model directory during status check
- **THEN** the system SHALL remove the empty directory
- **AND** update the status to "not downloaded"

### Requirement: Source-Aware Model Definitions
The system SHALL support ordered model download sources in MLX catalog metadata.

#### Scenario: HF-only runtime
- **WHEN** no mirror source is configured/enabled
- **THEN** model download SHALL proceed via Hugging Face source

#### Scenario: Optional mirror readiness
- **WHEN** mirror source metadata exists but mirror support is disabled or unconfigured
- **THEN** the system SHALL skip mirror source attempts
- **AND** SHALL use the next valid source

#### Scenario: Deterministic source fallback contract
- **WHEN** a higher-priority source fails and fallback is allowed
- **THEN** the system SHALL attempt the next source in priority order
- **AND** SHALL preserve consistent progress/error reporting semantics

