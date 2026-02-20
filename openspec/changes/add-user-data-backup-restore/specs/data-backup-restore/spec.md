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

### Requirement: Integrity Scope Disclosure (V1)
The system SHALL disclose that v1 checksum verification detects corruption/tamper evidence but does not provide trusted-origin authenticity.

#### Scenario: Preflight integrity message
- **WHEN** restore preflight reports checksum verification status
- **THEN** the report/UI states integrity checks are corruption-detection only
- **AND** the report/UI does not claim signature/authenticity guarantees in v1

### Requirement: Archive Extraction Safety
The system SHALL validate archive entry metadata during restore preflight and SHALL reject unsafe entries before file-content extraction.

#### Scenario: Path traversal entry is present
- **WHEN** an archive contains a path with parent traversal (for example `../settings_store.json`)
- **THEN** restore is rejected before file-content extraction into restore-managed paths

#### Scenario: Absolute-path entry is present
- **WHEN** an archive contains an absolute-path entry
- **THEN** restore is rejected before file-content extraction into restore-managed paths

#### Scenario: Symlink or hardlink entry is present
- **WHEN** an archive contains a symlink or hardlink entry
- **THEN** restore is rejected before file-content extraction into restore-managed paths

### Requirement: Archive Resource Bounds
The system SHALL enforce hard non-overridable bounded limits to prevent resource exhaustion during restore.

#### Scenario: Archive metadata exceeds hard security bounds
- **WHEN** archive metadata exceeds non-overridable hard security bounds (for example compression-ratio explosion or absolute parser ceilings)
- **THEN** restore is rejected as a blocking preflight failure
- **AND** active app data remains unchanged

#### Scenario: Oversized JSONL line breaches hard parser bound
- **WHEN** a history JSONL line exceeds hard parser safety bounds
- **THEN** restore is rejected as a blocking corruption failure
- **AND** active app data remains unchanged

### Requirement: Logical History Export
The system SHALL export history as logical payload records rather than raw live SQLite file copying.

#### Scenario: Export while app has active history database
- **WHEN** backup export runs
- **THEN** history rows and stats are serialized into payload files
- **AND** raw `history.db` is not copied into backup archive

### Requirement: Export Snapshot Consistency Fence
The system SHALL capture history, stats, and dictionary/settings payloads from one fenced point-in-time snapshot during export.

#### Scenario: Snapshot fence is brief and consistency-preserving
- **WHEN** backup export captures logical payload source data
- **THEN** the app briefly fences new history/settings writes for snapshot capture only
- **AND** the fence is released immediately after snapshot capture completes
- **AND** exported history/stats/dictionary/settings payloads come from the same captured snapshot point

### Requirement: Backup Scope Options (V1)
The system SHALL support exactly two backup scopes in v1, SHALL default export selection to `full`, and SHALL keep `lightweight` as an explicit user choice with clear consequences.

#### Scenario: Default export scope is full
- **WHEN** the user opens backup export
- **THEN** `full` is preselected as the default backup scope
- **AND** the UI labels `full` as the comprehensive backup option

#### Scenario: User explicitly chooses lightweight scope
- **WHEN** the user switches export scope from `full` to `lightweight`
- **THEN** the UI clearly states recordings/audio clips are excluded
- **AND** the UI clearly states history text, dictionary, and settings are still included
- **AND** export proceeds in `lightweight` mode only after explicit user choice

#### Scenario: Full scope selected
- **WHEN** export scope is `full`
- **THEN** backup includes history, dictionary, and available referenced recordings

#### Scenario: Lightweight scope selected
- **WHEN** export scope is `lightweight`
- **THEN** backup includes history, dictionary, and settings
- **AND** manifest sets `includes_recordings` to false

### Requirement: Missing Recording Handling During Export
The system SHALL tolerate missing referenced recording files during full export and SHALL record warnings.

#### Scenario: Full export has missing recording files
- **WHEN** a history row references a recording file that no longer exists
- **THEN** export still succeeds
- **AND** manifest warning metadata includes the missing filename
- **AND** the missing file is not added to archive

### Requirement: Restore Preflight Validation
The system SHALL run non-destructive preflight validation before any restore write and SHALL classify findings as blocking or recoverable.

#### Scenario: Preflight passes
- **WHEN** manifest, checksums, and compatibility gates are valid and no blocking findings exist
- **THEN** restore may proceed to staging import

#### Scenario: Preflight has recoverable findings only
- **WHEN** preflight finds recoverable issues and no blocking issues
- **THEN** the user is shown a concise issue summary (counts by category and plain-language impact)
- **AND** detailed per-item findings are available only behind an explicit `View details` action
- **AND** restore proceeds automatically when recoverable findings only affect optional recording files

#### Scenario: Preflight default summary includes anti-mistake context
- **WHEN** preflight returns a restore-ready summary
- **THEN** the default summary includes backup `created_at` and `history_entries`
- **AND** additional backup metadata remains available only behind explicit `View details`

#### Scenario: Preflight fails
- **WHEN** any blocking preflight validation check fails
- **THEN** restore is aborted
- **AND** active app data remains unchanged

#### Scenario: Optional recording files are missing or invalid
- **WHEN** preflight/import finds missing or invalid optional recording files and required payloads are valid
- **THEN** restore continues automatically
- **AND** skipped optional files are reported in a concise user-facing summary

### Requirement: Operation Concurrency Guard
The system SHALL allow at most one backup or restore operation at a time.

#### Scenario: Second operation is requested while one is running
- **WHEN** a backup or restore operation is already in progress
- **THEN** a new backup or restore request is rejected with a busy/operation-in-progress response

### Requirement: Restore Quiesce Mode
The system SHALL place the app in restore quiesce mode before apply-restore so active data is not concurrently modified.

#### Scenario: Restore starts while app is idle
- **WHEN** user confirms restore and no recording/transcription write is in progress
- **THEN** app enters restore quiesce mode
- **AND** new transcription/history-write operations are blocked until restore completes, fails, or rolls back

#### Scenario: Restore starts while a write is in flight
- **WHEN** restore begin is requested while recording/transcription persistence is still in progress
- **THEN** restore waits for the in-flight write to finish within timeout
- **AND** if timeout is exceeded, restore aborts with busy/retry guidance and leaves active data unchanged

#### Scenario: Blocked actions show clear status while restore is in progress
- **WHEN** user triggers transcription/history-write actions while restore quiesce mode is active
- **THEN** the action is rejected safely with a clear `restore_in_progress` message

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

#### Scenario: Destination filesystem constraint blocks export
- **WHEN** destination filesystem constraints prevent writing the estimated archive (for example single-file size limits)
- **THEN** export is blocked with a concise actionable error
- **AND** no partial backup file is left at destination

### Requirement: Export Destination Naming and Extension Guardrails
The system SHALL provide safe default naming and extension behavior when users choose export destination.

#### Scenario: Save dialog suggests Codictate default filename
- **WHEN** user opens export save flow
- **THEN** the save dialog pre-fills a default filename using `codictate-backup-YYYY-MM-DD_HH-mm.codictatebackup`

#### Scenario: Missing extension is auto-appended
- **WHEN** user enters a destination filename without `.codictatebackup`
- **THEN** export appends `.codictatebackup` before writing
- **AND** the UI confirms the final filename before export starts

#### Scenario: Existing destination file is selected
- **WHEN** the chosen destination already exists
- **THEN** the UI requires explicit overwrite confirmation before export proceeds
- **AND** if overwrite is declined, export is canceled without modifying existing file

### Requirement: Backup Format Compatibility Window
The system SHALL use payload-version migration support for all known versions within backup format major `1`, and SHALL require a documented deprecation runway before narrowing support in future major-format transitions.

#### Scenario: Supported v1 payload version restore
- **WHEN** backup format major is `1` and payload versions are within the known migration map
- **THEN** restore is allowed subject to other validations

#### Scenario: Too-old restore is rejected
- **WHEN** backup format major is older than supported compatibility policy
- **THEN** restore is rejected with unsupported-version guidance

#### Scenario: Forward-incompatible restore is rejected
- **WHEN** backup format major is newer than current supported major
- **THEN** restore is rejected with update-app guidance

#### Scenario: Deprecation runway for future major transition
- **WHEN** support for a prior format major is planned for removal
- **THEN** a deprecation notice is documented and shipped at least one stable release before removal

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
- **AND** the user receives success summary with restored and skipped counts

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

#### Scenario: Cancellation state is explicit in UI
- **WHEN** backup or restore is in a cancellable phase
- **THEN** the UI provides a visible cancel action
- **AND** after cancel is requested, UI shows cancel-in-progress/cleanup status until completion
- **AND** the final state explicitly confirms cancellation completed safely

#### Scenario: Cancellation unavailable during commit phase
- **WHEN** restore reaches the short non-cancellable commit phase near final swap
- **THEN** the UI disables cancel action for that phase
- **AND** the UI explains that cancellation will resume after commit completes or rollback begins

### Requirement: Crash and Interruption Recovery
The system SHALL recover safely from interruption during restore.

#### Scenario: App interruption during restore
- **WHEN** the app process exits unexpectedly during restore
- **THEN** next startup runs restore reconciliation
- **AND** active restore-managed data is either intact pre-restore data or fully restored data
- **AND** partial intermediate state is not left active

#### Scenario: Reconciliation from `active_moved` phase
- **WHEN** startup finds restore marker phase `active_moved`
- **THEN** reconciliation restores active data from rollback snapshot before allowing normal app writes
- **AND** pre-restore history/stats/dictionary state is preserved

#### Scenario: Reconciliation from `staged_activated` phase
- **WHEN** startup finds restore marker phase `staged_activated`
- **THEN** reconciliation finalizes committed restored state and clears stale rollback/in-progress artifacts safely
- **AND** active data remains non-partial and internally consistent

#### Scenario: Startup reconciliation outcome is shown to user
- **WHEN** startup reconciliation runs after an interrupted restore
- **THEN** the app shows a user-facing outcome summary (for example rolled back to pre-restore data or restore completed)
- **AND** the summary includes timestamp and next-step guidance

### Requirement: Missing Audio Behavior on Restore
The system SHALL preserve history text restore when recordings are absent and SHALL fail audio playback gracefully.

#### Scenario: Restore from lightweight backup
- **WHEN** backup has no recordings
- **THEN** history text, dictionary, and settings are restored
- **AND** attempts to play unavailable audio return a user-facing unavailable message

#### Scenario: Restore from full backup with partial audio
- **WHEN** some history rows reference recordings not present in archive
- **THEN** those history rows are restored
- **AND** unavailable audio is handled gracefully without app crash

### Requirement: Strict Core Payload Validation
The system SHALL treat malformed or invalid required payload content as blocking corruption and SHALL only treat optional recording-file issues as recoverable.

#### Scenario: Malformed required history row
- **WHEN** a required history payload row cannot be parsed or fails schema validation
- **THEN** restore is rejected as corrupted backup before active-data replacement

#### Scenario: Invalid required dictionary or settings payload
- **WHEN** required dictionary/settings payload cannot be parsed or fails schema validation
- **THEN** restore is rejected as corrupted backup before active-data replacement

#### Scenario: Optional recording issue
- **WHEN** optional recording files are missing or invalid while required payloads are valid
- **THEN** restore continues with warnings
- **AND** history text and settings restore remain successful

### Requirement: Operation Reports
The system SHALL return structured export and restore reports for UI display.

#### Scenario: Export report
- **WHEN** backup export completes
- **THEN** report includes scope, item counts, output path, and warning counts

#### Scenario: Restore report
- **WHEN** restore completes or fails
- **THEN** report includes restored item counts, skipped-item counts, missing-audio count, and error/warning details

### Requirement: Pragmatic User-Facing Outcome Summaries
The system SHALL present concise confidence-building summaries by default and keep detailed diagnostics optional.

#### Scenario: Export succeeds without warnings
- **WHEN** export completes with no warnings
- **THEN** the UI shows a concise success summary that confirms the Codictate backup is complete and ready to use
- **AND** the summary includes only key facts (scope, destination, size, created time)

#### Scenario: Restore succeeds with no or minor warnings
- **WHEN** restore completes
- **THEN** the UI shows a concise summary that confirms restore integrity checks and apply steps completed successfully
- **AND** if warnings exist, the summary shows warning counts and plain-language impact only
- **AND** detailed warning/file-level diagnostics are available behind an explicit `View details` action

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

### Requirement: Restore-Managed Dataset Contract
The system SHALL treat restore-managed active data as one explicit protection set for rollback, swap, and reconciliation.

#### Scenario: Restore-managed dataset is explicit
- **WHEN** rollback snapshot or destructive swap is prepared
- **THEN** the protected dataset includes active `history.db` (including stats tables), dictionary source data in `settings_store.json`, and restore-managed `recordings/`
- **AND** rollback and reconciliation operate over this exact dataset contract

### Requirement: Atomic Swap Durability
The system SHALL perform restore commit using same-volume atomic renames and SHALL fail safely before destructive actions when atomicity is not guaranteed.

#### Scenario: Same-volume atomic swap succeeds
- **WHEN** staging, active, and rollback paths are on the same filesystem
- **THEN** commit uses atomic rename operations only
- **AND** copy/delete swap semantics are not used

#### Scenario: Atomic swap cannot be guaranteed
- **WHEN** restore detects swap paths that cannot guarantee same-volume atomic rename
- **THEN** restore is aborted before destructive swap begins
- **AND** active restore-managed data remains unchanged

### Requirement: Pre-Swap Rollback Snapshot
The system SHALL preserve currently active restore-managed data in a rollback workspace before final restore swap.

#### Scenario: Rollback snapshot is created before destructive swap
- **WHEN** user confirms restore after preflight
- **THEN** the system creates a rollback snapshot of active restore-managed data
- **AND** restore proceeds to final swap only after snapshot validation succeeds

#### Scenario: Restore failure uses rollback snapshot
- **WHEN** restore fails after snapshot creation and before final success
- **THEN** rollback restores pre-restore active data from rollback snapshot
- **AND** user receives actionable failure summary

#### Scenario: Startup reconciliation uses rollback snapshot
- **WHEN** app is interrupted during restore
- **THEN** startup reconciliation restores a consistent final state using rollback snapshot or completed staged state
- **AND** partial intermediate state is never left active

#### Scenario: Rollback snapshot retention is bounded
- **WHEN** restore completes successfully
- **THEN** the latest pre-restore rollback snapshot is retained for 7 days
- **AND** snapshots older than the retention window are cleaned automatically on startup or post-restore cleanup

#### Scenario: Restore confirmation shows concise impact summary
- **WHEN** the user views the restore confirmation dialog
- **THEN** the UI shows a concise restore impact summary including backup `created_at` and `history_entries`
- **AND** detailed backup metadata and side-by-side counts are available only behind explicit `View details`
- **AND** the UI labels the artifact as a Codictate backup (`.codictatebackup`)
- **AND** the UI shows a `Will restore` section and a `Will not restore` section
- **AND** the `Will not restore` section explicitly lists excluded settings (API keys, selected devices, selected model)
- **AND** if recordings are absent (lightweight backup or missing files), the UI explicitly states affected audio clips will be unavailable after restore
- **AND** the UI warns that current data will be replaced

### Requirement: User Stats Recomputation on Restore
The system SHALL recompute user stats from imported history rows rather than importing the backup stats snapshot.

#### Scenario: Stats are recomputed after restore
- **WHEN** restore successfully imports history rows
- **THEN** `total_words`, `total_duration_ms`, `total_transcriptions`, `transcription_dates`, and `total_filler_words_removed` are recomputed from the imported rows
- **AND** recomputed stats accurately reflect the restored history

### Requirement: Selective Settings Backup
The system SHALL export user-configurable preferences excluding sensitive and device-specific fields.

#### Scenario: Settings export excludes API keys
- **WHEN** backup export runs
- **THEN** the `settings/settings.json` payload does not contain `post_process_api_keys`

#### Scenario: Settings export excludes device-specific fields
- **WHEN** backup export runs
- **THEN** the `settings/settings.json` payload does not contain `selected_microphone`, `clamshell_microphone`, `selected_output_device`, or `selected_model`

#### Scenario: Settings export includes user preferences
- **WHEN** backup export runs
- **THEN** the `settings/settings.json` payload includes user-configurable fields such as shortcuts, language, overlay, and paste preferences

### Requirement: Forward-Compatible Settings Restore
The system SHALL merge restored settings selectively, using defaults for any new fields and ignoring removed fields.

#### Scenario: Backup from older app version restored on newer version
- **WHEN** a backup is missing settings fields that exist in the current app version
- **THEN** those fields retain their current or default values
- **AND** fields present in both the backup and current schema are restored from the backup

#### Scenario: Backup contains fields removed in current version
- **WHEN** a backup contains settings fields that no longer exist in the current app version
- **THEN** those fields are silently ignored

### Requirement: Progress Reporting
The system SHALL emit incremental progress events during export and restore for frontend display.

#### Scenario: Export progress events
- **WHEN** backup export is in progress
- **THEN** the system emits progress events with phase, current count, and total count
- **AND** the frontend displays a progress bar
- **AND** ETA, when shown, is clearly best-effort

#### Scenario: Restore progress events
- **WHEN** restore is in progress
- **THEN** the system emits progress events with phase, current count, and total count
- **AND** the frontend displays a progress bar
- **AND** ETA, when shown, is clearly best-effort

### Requirement: Estimated Size Preview
The system SHALL display estimated archive size before export begins.

#### Scenario: Size preview shown before export
- **WHEN** the user selects backup export scope
- **THEN** the UI shows the estimated archive size
- **AND** after destination selection the UI runs destination-space check before final export confirmation

#### Scenario: Destination has insufficient space
- **WHEN** the selected destination has less free space than the estimated archive size
- **THEN** export is blocked with an actionable error message

### Requirement: Cross-Platform Universal Backup
The system SHALL ensure backups are portable across macOS, Windows, and Linux.

#### Scenario: Archive created on macOS restored on Windows
- **WHEN** a backup created on macOS is restored on Windows
- **THEN** all filenames are normalized to NFC Unicode form
- **AND** path separators are resolved correctly
- **AND** history and dictionary are restored successfully

#### Scenario: Archive contains invalid characters for target platform
- **WHEN** a required payload path contains characters invalid on the restoring platform
- **THEN** restore is rejected as blocking/unsafe before apply-restore begins

#### Scenario: Optional recording filename contains invalid characters
- **WHEN** a recording filename is invalid for the restoring platform
- **THEN** restore deterministically sanitizes and remaps references when safe
- **AND** if safe remap is not possible, that recording is skipped with recoverable warning while restore continues

### Requirement: Structured Logging for Backup/Restore
The system SHALL emit structured logs at every major backup/restore milestone.

#### Scenario: Export logging
- **WHEN** backup export runs
- **THEN** `info` logs are emitted at start, snapshot completion, recording scan, and export completion with counts and duration

#### Scenario: Restore logging
- **WHEN** restore runs
- **THEN** `info` logs are emitted at preflight start, rollback snapshot creation, migration, staging, swap completion, and restore completion with counts and duration
- **AND** `error` logs are emitted on any failure with step context

#### Scenario: Frontend logging
- **WHEN** the user initiates backup or restore from the UI
- **THEN** `logInfo` and `logError` are called with target `fe-backup`

### Requirement: Logging Redaction and Privacy
The system SHALL enforce redaction so backup/restore logs never expose sensitive user content.

#### Scenario: Restore logs are emitted
- **WHEN** backup/restore logs are written at any log level
- **THEN** logs include metadata only (counts, durations, phases, non-sensitive identifiers)
- **AND** logs do not include transcript text, prompt text, API keys, or raw payload bodies

### Requirement: Internationalization for Backup/Restore UI
The system SHALL use i18next translation keys for all backup/restore user-facing strings.

#### Scenario: All backup UI strings use translation keys
- **WHEN** backup/restore UI is rendered
- **THEN** all labels, warnings, confirmations, progress messages, and error messages use `t()` with keys under `settings.backup` namespace
- **AND** English translation keys are defined in `src/i18n/locales/en/translation.json`

### Requirement: Accessibility for Backup/Restore Flows
The system SHALL provide accessible backup/restore UX for keyboard and assistive-technology users.

#### Scenario: Dialogs are keyboard-operable and focus-managed
- **WHEN** backup/restore confirmation or warning dialogs are opened
- **THEN** dialogs are fully operable using keyboard only
- **AND** focus is trapped within the active dialog and restored on close
- **AND** destructive primary actions require explicit focused confirmation

#### Scenario: Progress and errors are announced to assistive tech
- **WHEN** backup/restore progress or error state changes
- **THEN** screen readers receive appropriate announcements for progress, warnings, and completion/failure
- **AND** severity cues are not color-only
- **AND** text/icons meet accessible contrast requirements
