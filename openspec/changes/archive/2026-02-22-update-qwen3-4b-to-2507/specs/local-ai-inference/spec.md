## MODIFIED Requirements

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

#### Scenario: 4B in-place upgrade to 2507
- **WHEN** the model registry is initialized
- **THEN** `qwen3_base_4b` SHALL map to `mlx-community/Qwen3-4B-Instruct-2507-4bit`
- **AND** existing selections using `qwen3_base_4b` SHALL remain valid without migration

#### Scenario: 4B metadata refresh only
- **WHEN** the MLX model catalog is rendered
- **THEN** only the 4B Qwen model entry SHALL receive updated storage/RAM metadata in this change
- **AND** the Qwen 0.6B, 1.7B, and 8B entries SHALL remain unchanged

#### Scenario: Public ungated direct download
- **WHEN** downloading the Qwen 4B `2507` model from Hugging Face
- **THEN** the system SHALL support unauthenticated download for public ungated access
- **AND** the system SHALL continue to support token-authenticated access for gated/private repositories when configured
