## MODIFIED Requirements

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

## ADDED Requirements

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
