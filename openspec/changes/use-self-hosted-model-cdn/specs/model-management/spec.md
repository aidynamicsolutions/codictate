## ADDED Requirements

### Requirement: Configurable Model Source
The system SHALL support a configurable base URL for downloading model artifacts.

#### Scenario: Default Configuration
- **WHEN** the application is built with default settings
- **THEN** it uses the official project CDN (currently `blob.handy.computer`)

#### Scenario: Custom Configuration
- **WHEN** the application is built with a custom `MODEL_CDN_URL`
- **THEN** model download requests are directed to the custom URL
- **AND** the directory structure is preserved relative to the base URL
