# transcription-history Specification

## Purpose
TBD - created by archiving change update-history-inserted-text-parity. Update Purpose after archive.
## Requirements
### Requirement: History Entry Text Provenance
The system SHALL preserve raw ASR transcript content while exposing inserted-result parity through explicit history fields.

#### Scenario: Additive migration preserves existing rows
- **WHEN** an existing user upgrades to a build that includes `inserted_text`
- **THEN** migration only adds nullable column `inserted_text`
- **AND** existing `transcription_text`, `post_processed_text`, and `post_process_prompt` values remain unchanged

#### Scenario: Effective text fallback on legacy rows
- **WHEN** a history row has `inserted_text = NULL`
- **AND** row has `post_processed_text`
- **THEN** row `effective_text` resolves to `post_processed_text`

#### Scenario: Effective text fallback to raw ASR
- **WHEN** a history row has `inserted_text = NULL`
- **AND** row has `post_processed_text = NULL`
- **THEN** row `effective_text` resolves to `transcription_text`

#### Scenario: Raw alias is always present
- **WHEN** history row is returned through API
- **THEN** `raw_text` equals stored `transcription_text`

### Requirement: History Search Coverage
History queries SHALL match both effective text and raw ASR text.

#### Scenario: Match by effective inserted text
- **WHEN** user searches for a token that appears only in `effective_text`
- **THEN** matching row is returned

#### Scenario: Match by raw ASR text
- **WHEN** user searches for a token that appears only in `raw_text`
- **THEN** matching row is returned

### Requirement: History UI Disclosure Behavior
History UI SHALL keep rows compact while exposing raw transcript details on demand.

#### Scenario: Primary row text uses effective text
- **WHEN** history row is rendered
- **THEN** primary visible row text uses `effective_text`

#### Scenario: Original transcript action visibility
- **WHEN** `raw_text` differs from `effective_text`
- **THEN** row shows an `Original transcript` toggle action
- **AND** row hides this action when values are identical

#### Scenario: Inline original transcript disclosure
- **WHEN** user activates `Original transcript`
- **THEN** row expands inline to show raw ASR text
- **AND** disclosure is keyboard operable with explicit expanded state semantics

#### Scenario: Copy action behavior
- **WHEN** user activates row copy action
- **THEN** copied value is row `effective_text`

#### Scenario: Raw-only search hint
- **WHEN** current search query matches row `raw_text` but not row `effective_text`
- **AND** original transcript panel is collapsed
- **THEN** row shows a hint that match exists in original transcript

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

### Requirement: Restore Stats Fidelity
Backup/restore SHALL preserve canonical home stats aggregates when canonical stats payload is available.

#### Scenario: Canonical stats payload roundtrip
- **WHEN** backup archive includes `history/user_stats.json`
- **AND** restore applies that archive
- **THEN** `user_stats` aggregate fields are restored from canonical payload
- **AND** WPM / Total Words / Time Saved match source backup aggregates

#### Scenario: Legacy archive fallback recompute
- **WHEN** restore archive is missing `history/user_stats.json` (legacy format)
- **THEN** restore remains non-blocking
- **AND** `user_stats` are recomputed from history rows with runtime-consistent word counting semantics
- **AND** speech-duration fallback treats missing row speech duration as recording duration

#### Scenario: Legacy fallback warning surface
- **WHEN** restore uses fallback recompute instead of canonical stats payload
- **THEN** preflight reports a recoverable warning
- **AND** apply-restore warning output includes the stats-recompute warning context
