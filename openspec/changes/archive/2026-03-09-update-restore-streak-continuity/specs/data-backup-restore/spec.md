## ADDED Requirements
### Requirement: Restore Preserves Active Streak Eligibility
The system SHALL preserve an active daily streak from backup as restore carry-over state when the streak was still active at backup creation time.

#### Scenario: Active streak remains eligible after delayed restore
- **WHEN** a backup contains canonical streak dates whose most recent day is the backup creation local day or the immediately preceding local day
- **AND** the user restores that backup on a later local day
- **THEN** restore preserves the backed-up streak length as eligible carry-over state
- **AND** elapsed wall-clock days between backup creation and restore do not by themselves count as missed streak days

#### Scenario: Broken streak is not resurrected by restore
- **WHEN** a backup contains streak dates whose most recent day is more than one local day older than the backup creation local day
- **THEN** restore does not create restore carry-over state for that streak
- **AND** the restored streak remains broken

#### Scenario: Restore does not invent historical transcription days
- **WHEN** restore preserves an eligible carried streak
- **THEN** restore does not synthesize missing historical days into canonical transcription history
- **AND** restore preserves historical transcription dates exactly as represented by the backup payload
