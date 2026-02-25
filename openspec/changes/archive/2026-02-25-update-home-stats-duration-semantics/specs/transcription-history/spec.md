## ADDED Requirements
### Requirement: Dual Duration Stats Semantics
The system SHALL track recording elapsed duration and speech-retained duration as separate metrics for home stats.

#### Scenario: Save transcription records both duration dimensions
- **WHEN** a transcription is saved
- **THEN** `transcription_history.duration_ms` stores recording elapsed duration
- **AND** `transcription_history.speech_duration_ms` stores VAD-retained unpadded speech duration
- **AND** `user_stats.total_duration_ms` increments by recording duration
- **AND** `user_stats.total_speech_duration_ms` increments by speech duration

#### Scenario: Home stats uses hybrid formulas
- **WHEN** home stats are requested after duration semantics version is active
- **THEN** WPM is computed from `total_speech_duration_ms`
- **AND** Time Saved is computed from `total_duration_ms`

### Requirement: Crash-Safe Duration Semantics Migration
The system SHALL migrate duration semantics atomically and idempotently with pre-mutation backups.

#### Scenario: Migration writes backup artifacts before stats mutation
- **WHEN** duration semantics migration runs for a database with marker version `< 1`
- **THEN** a filesystem snapshot is written under `stats-backups/`
- **AND** a backup row is inserted into `user_stats_migration_backup`
- **AND** the migration logs backup artifact references

#### Scenario: Migration commits atomically
- **WHEN** migration updates duration aggregates
- **THEN** aggregate rewrites and marker update occur in one transaction
- **AND** `duration_stats_semantics_version` is set to `1` only after successful commit

#### Scenario: Marker prevents partial semantic reads
- **WHEN** `duration_stats_semantics_version` is `0`
- **THEN** home stats continues legacy duration interpretation
- **AND** hybrid interpretation is used only when marker is `1`

### Requirement: Undo Reverses Both Duration Counters
The system SHALL reverse both recording and speech duration contributions when undoing transcribe-origin entries.

#### Scenario: Undo rollback subtracts both duration totals
- **WHEN** undo rollback is applied for a tracked transcribe contribution
- **THEN** `user_stats.total_duration_ms` is decremented by contribution recording duration
- **AND** `user_stats.total_speech_duration_ms` is decremented by contribution speech duration
- **AND** both values clamp at zero
