## ADDED Requirements
### Requirement: Restore-Aware Daily Streak Continuity
The system SHALL continue an eligible restored streak from the restore date without rewriting historical transcription dates.

#### Scenario: Restore day shows preserved streak before new transcription
- **WHEN** an eligible carried streak has just been restored
- **AND** the user requests home stats on the same local day before creating a new transcription
- **THEN** home stats report the preserved streak length from the backup

#### Scenario: First post-restore transcription increments preserved streak
- **WHEN** an eligible carried streak has been restored
- **AND** the user creates the first transcription on the restore local day
- **THEN** the reported streak increments from the preserved streak length
- **AND** the streak does not restart at `1`

#### Scenario: First missed post-restore day expires preserved streak
- **WHEN** an eligible carried streak has been restored
- **AND** the user reaches the next local calendar day after restore without any new transcription since restore
- **THEN** home stats report a current streak of `0`

#### Scenario: Consecutive post-restore days continue from preserved baseline
- **WHEN** an eligible carried streak has been restored
- **AND** the user records on the restore local day and on each following consecutive local day
- **THEN** the reported streak continues increasing from the preserved streak baseline
