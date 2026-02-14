## ADDED Requirements
### Requirement: Single-Archive Backup Artifact (V1)
The system SHALL export backups as a single `.codictatebackup` archive file.

#### Scenario: User exports full backup
- **WHEN** the user selects backup export with recordings included
- **THEN** the system writes one `.codictatebackup` file
- **AND** the archive contains `manifest.json`, `checksums.sha256`, history payload files, dictionary payload file, and `recordings/`

#### Scenario: User exports lightweight backup
- **WHEN** the user selects backup export without recordings
- **THEN** the system writes one `.codictatebackup` file
- **AND** the archive does not include `recordings/`

#### Scenario: User cancels file-save dialog
- **WHEN** the user cancels destination selection
- **THEN** backup export is aborted
- **AND** no user data is modified

### Requirement: Manifest and Checksum Integrity
The system SHALL include a versioned manifest and checksums in every backup and SHALL verify them before restore.

#### Scenario: Manifest includes required metadata
- **WHEN** backup export completes
- **THEN** `manifest.json` includes `backup_format_version`, `created_with_app_version`, `created_at`, `includes_recordings`, component payload versions, and per-component counts

#### Scenario: Checksum mismatch blocks restore
- **WHEN** any payload file hash differs from `checksums.sha256`
- **THEN** restore is rejected before writing active data

#### Scenario: Missing required payload file blocks restore
- **WHEN** a required file such as `history/history.jsonl` is absent
- **THEN** restore is rejected before writing active data

### Requirement: Archive Extraction Safety
The system SHALL reject unsafe archive entries during restore preflight.

#### Scenario: Path traversal entry is present
- **WHEN** an archive contains a path with parent traversal (for example `../settings_store.json`)
- **THEN** restore is rejected before extraction into restore-managed paths

#### Scenario: Absolute-path entry is present
- **WHEN** an archive contains an absolute-path entry
- **THEN** restore is rejected before extraction into restore-managed paths

#### Scenario: Symlink or hardlink entry is present
- **WHEN** an archive contains a symlink or hardlink entry
- **THEN** restore is rejected before extraction into restore-managed paths

### Requirement: Logical History Export
The system SHALL export history as logical payload records rather than raw live SQLite file copying.

#### Scenario: Export while app has active history database
- **WHEN** backup export runs
- **THEN** history rows and stats are serialized into payload files
- **AND** raw `history.db` is not copied into backup archive

### Requirement: Backup Scope Options (V1)
The system SHALL support exactly two backup scopes in v1.

#### Scenario: Full scope selected
- **WHEN** export scope is `full`
- **THEN** backup includes history, dictionary, and available referenced recordings

#### Scenario: Lightweight scope selected
- **WHEN** export scope is `lightweight`
- **THEN** backup includes history and dictionary only
- **AND** manifest sets `includes_recordings` to false

### Requirement: Missing Recording Handling During Export
The system SHALL tolerate missing referenced recording files during full export and SHALL record warnings.

#### Scenario: Full export has missing recording files
- **WHEN** a history row references a recording file that no longer exists
- **THEN** export still succeeds
- **AND** manifest warning metadata includes the missing filename
- **AND** the missing file is not added to archive

### Requirement: Restore Preflight Validation
The system SHALL run non-destructive preflight validation before any restore write.

#### Scenario: Preflight passes
- **WHEN** manifest, checksums, and compatibility gates are valid
- **THEN** restore may proceed to staging import

#### Scenario: Preflight fails
- **WHEN** any preflight validation check fails
- **THEN** restore is aborted
- **AND** active app data remains unchanged

### Requirement: Operation Concurrency Guard
The system SHALL allow at most one backup or restore operation at a time.

#### Scenario: Second operation is requested while one is running
- **WHEN** a backup or restore operation is already in progress
- **THEN** a new backup or restore request is rejected with a busy/operation-in-progress response

### Requirement: Storage Capacity Precheck
The system SHALL check required free disk space before export and restore.

#### Scenario: Export blocked by low disk space
- **WHEN** available disk space is lower than export workspace plus output requirements
- **THEN** export is aborted with actionable low-space error
- **AND** no partial backup file is left at destination

#### Scenario: Restore blocked by low disk space
- **WHEN** available disk space is lower than unpack + staging + rollback requirements
- **THEN** restore is aborted before active data write
- **AND** active app data remains unchanged

### Requirement: Backup Format Compatibility Window
The system SHALL enforce backup format support for current major and previous major only.

#### Scenario: Current major restore
- **WHEN** backup format major equals current supported major
- **THEN** restore is allowed subject to other validations

#### Scenario: Previous major restore
- **WHEN** backup format major equals previous supported major
- **THEN** restore is allowed through migration pipeline

#### Scenario: Too-old restore is rejected
- **WHEN** backup format major is older than previous supported major
- **THEN** restore is rejected with unsupported-version guidance

#### Scenario: Forward-incompatible restore is rejected
- **WHEN** backup format major is newer than current supported major
- **THEN** restore is rejected with update-app guidance

### Requirement: Schema Migration for Supported Backups
The system SHALL migrate supported older payload schemas to current import schemas before staging import.

#### Scenario: Migration path exists
- **WHEN** payload schema versions are older but supported
- **THEN** restore applies deterministic migrations and imports migrated payload

#### Scenario: Migration path missing
- **WHEN** a supported backup requires a missing migration step
- **THEN** restore is aborted before active data replacement

### Requirement: Replace-Only Staged Restore (V1)
The system SHALL perform restore in staging and SHALL replace active restore-managed data only after staging succeeds.

#### Scenario: Restore success
- **WHEN** restore completes staging import and validation
- **THEN** active restore-managed data is replaced with staged data
- **AND** the user receives success summary with restored counts

#### Scenario: Restore failure before completion
- **WHEN** restore fails after preflight but before final success
- **THEN** rollback preserves pre-restore active data
- **AND** user receives actionable error summary

### Requirement: Cancellation Safety
The system SHALL handle backup or restore cancellation without corrupting active data.

#### Scenario: User cancels export
- **WHEN** a user cancels an in-progress export
- **THEN** temporary workspace files are cleaned up
- **AND** no partial archive remains at destination

#### Scenario: User cancels restore
- **WHEN** a user cancels an in-progress restore before final swap completes
- **THEN** active restore-managed data remains unchanged
- **AND** staging and rollback temp files are cleaned up

### Requirement: Crash and Interruption Recovery
The system SHALL recover safely from interruption during restore.

#### Scenario: App interruption during restore
- **WHEN** the app process exits unexpectedly during restore
- **THEN** next startup runs restore reconciliation
- **AND** active restore-managed data is either intact pre-restore data or fully restored data
- **AND** partial intermediate state is not left active

### Requirement: Missing Audio Behavior on Restore
The system SHALL preserve history text restore when recordings are absent and SHALL fail audio playback gracefully.

#### Scenario: Restore from lightweight backup
- **WHEN** backup has no recordings
- **THEN** history text and dictionary are restored
- **AND** attempts to play unavailable audio return a user-facing unavailable message

#### Scenario: Restore from full backup with partial audio
- **WHEN** some history rows reference recordings not present in archive
- **THEN** those history rows are restored
- **AND** unavailable audio is handled gracefully without app crash

### Requirement: Deterministic Import Conflict Handling
The system SHALL resolve duplicate imported identifiers and filename collisions deterministically.

#### Scenario: Duplicate history IDs in payload
- **WHEN** imported history payload contains duplicate row identifiers
- **THEN** restore rekeys conflicting rows deterministically
- **AND** preserves all valid rows

#### Scenario: Duplicate recording filenames in payload
- **WHEN** imported recordings include filename collisions
- **THEN** restore applies deterministic rename strategy
- **AND** restored history rows reference the resolved filenames correctly

### Requirement: Operation Reports
The system SHALL return structured export and restore reports for UI display.

#### Scenario: Export report
- **WHEN** backup export completes
- **THEN** report includes scope, item counts, output path, and warning counts

#### Scenario: Restore report
- **WHEN** restore completes or fails
- **THEN** report includes restored item counts, missing-audio count, and error/warning details

### Requirement: Unencrypted Backups in V1
The system SHALL export backups without built-in encryption in v1 and SHALL disclose this before export.

#### Scenario: User starts export
- **WHEN** the user opens backup export action
- **THEN** UI indicates the backup archive is not app-encrypted
- **AND** exported archive is unencrypted by app logic

### Requirement: Path Scope Restriction
The system SHALL restrict backup and restore file operations to user-selected archive paths and app-managed data directories.

#### Scenario: Restore attempts to write outside allowed scope
- **WHEN** restore processing resolves a path outside user-selected archive scope or app data scope
- **THEN** the operation is rejected as unsafe

#### Scenario: Export attempts to write outside allowed scope
- **WHEN** export destination is not the user-selected save path
- **THEN** export write is rejected
