## ADDED Requirements

### Requirement: Local MLX Model Management
The system SHALL provide local AI model management for Apple Silicon Macs, allowing users to download, load, and delete MLX-based language models for on-device transcription enhancement.

#### Scenario: List available models
- **WHEN** a user opens the post-processing settings on an Apple Silicon Mac
- **THEN** the system SHALL display a list of available MLX models with their download status (not downloaded, downloading, downloaded, loading, ready)

#### Scenario: Download a model from Hugging Face Hub
- **WHEN** a user initiates a model download
- **THEN** the system SHALL download the model from Hugging Face Hub with progress reporting
- **AND** the system SHALL allow the user to cancel the download

#### Scenario: Cancel download
- **WHEN** a user cancels an in-progress download
- **THEN** the system SHALL stop the download immediately
- **AND** reset the download progress to 0
- **AND** update the status to reflect cancellation

#### Scenario: Retry failed download
- **WHEN** a model download fails due to network error
- **THEN** the system SHALL allow the user to retry the download
- **AND** limit retry attempts to a maximum of 3

#### Scenario: Delete a model
- **WHEN** a user requests to delete a downloaded model
- **THEN** the system SHALL remove the model files from disk and update the model status

#### Scenario: Prevent delete while busy
- **WHEN** a user attempts to delete a model that is currently downloading, loading, or running
- **THEN** the system SHALL prevent the deletion and show an appropriate error message

#### Scenario: Platform restriction
- **WHEN** the app runs on a non-Apple Silicon platform (x86_64, Windows, Linux)
- **THEN** the system SHALL NOT display the local MLX provider option in settings

---

### Requirement: Local AI Text Generation
The system SHALL support text generation using locally loaded MLX models for transcription post-processing.

#### Scenario: Process transcription with local model
- **WHEN** the user has post-processing enabled with the local MLX provider selected
- **AND** a transcription is completed
- **THEN** the system SHALL process the transcription through the loaded MLX model using the selected prompt

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
