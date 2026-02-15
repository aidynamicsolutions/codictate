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
The system SHALL enforce a two-tier bounded-limit model (hard security bounds and soft operational thresholds) to prevent resource exhaustion during restore.

#### Scenario: Archive metadata exceeds hard security bounds
- **WHEN** archive metadata exceeds non-overridable hard security bounds (for example compression-ratio explosion or absolute parser ceilings)
- **THEN** restore is rejected as a blocking preflight failure
- **AND** active app data remains unchanged

#### Scenario: Soft operational threshold is exceeded
- **WHEN** archive metadata exceeds configured soft thresholds (for example soft total size or soft entry-count limit) but does not exceed hard security bounds
- **THEN** preflight presents warning details and explicit choices to continue once, update soft limits and continue, or cancel
- **AND** restore proceeds only after explicit user confirmation

#### Scenario: User updates soft thresholds
- **WHEN** user chooses update-soft-limits during preflight warning
- **THEN** updated soft thresholds are persisted to settings
- **AND** restore re-evaluates against updated soft thresholds

#### Scenario: Oversized JSONL line during import within hard bounds
- **WHEN** a history JSONL line exceeds configured soft bounded-parser line-size threshold but remains within hard parser safety bounds
- **THEN** that row is skipped as a recoverable warning
- **AND** restore continues safely for remaining valid rows

### Requirement: Logical History Export
The system SHALL export history as logical payload records rather than raw live SQLite file copying.

#### Scenario: Export while app has active history database
- **WHEN** backup export runs
- **THEN** history rows and stats are serialized into payload files
- **AND** raw `history.db` is not copied into backup archive

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
- **AND** the user must explicitly choose continue-partial-restore or cancel
- **AND** restore proceeds only when the user confirms continue-partial-restore

#### Scenario: User cancels after recoverable preflight findings
- **WHEN** preflight finds recoverable issues and user chooses cancel
- **THEN** restore is aborted
- **AND** active app data remains unchanged

#### Scenario: Preflight fails
- **WHEN** any blocking preflight validation check fails
- **THEN** restore is aborted
- **AND** active app data remains unchanged

### Requirement: Operation Concurrency Guard
The system SHALL allow at most one backup or restore operation at a time.

#### Scenario: Second operation is requested while one is running
- **WHEN** a backup or restore operation is already in progress
- **THEN** a new backup or restore request is rejected with a busy/operation-in-progress response

#### Scenario: Mandatory safety backup does not deadlock restore
- **WHEN** restore runs its mandatory pre-restore safety backup
- **THEN** that safety backup runs as an internal restore phase under the same operation context
- **AND** the system does not attempt to acquire a second top-level backup/restore lock

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

#### Scenario: Blocked actions show clear status while restore runs in background
- **WHEN** user triggers transcription/history-write actions while restore quiesce mode is active
- **THEN** the action is rejected safely with a clear `restore_in_progress` message
- **AND** the UI offers an action to open the live restore status view

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

#### Scenario: Destination filesystem file-size limit blocks export
- **WHEN** estimated archive size exceeds the selected destination filesystem single-file limit (for example FAT32 4 GiB)
- **THEN** export is blocked with actionable guidance
- **AND** guidance includes explicit choices to pick another destination or switch to `lightweight`
- **AND** export does not start until the user makes a new explicit choice

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

### Requirement: Deterministic Import Conflict Handling
The system SHALL resolve duplicate imported identifiers and filename collisions deterministically while preserving valid data and skipping only unrecoverable items.

#### Scenario: Duplicate history IDs in payload
- **WHEN** imported history payload contains duplicate row identifiers
- **THEN** restore rekeys conflicting rows deterministically
- **AND** preserves all valid rows

#### Scenario: Duplicate recording filenames in payload
- **WHEN** imported recordings include filename collisions
- **THEN** restore applies deterministic rename strategy
- **AND** restored history rows reference the resolved filenames correctly

#### Scenario: Malformed history rows are present
- **WHEN** one or more history rows cannot be parsed or fail schema validation
- **THEN** those rows are skipped with recoverable warnings
- **AND** valid rows continue restoring

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

### Requirement: Pre-Restore Safety Backup
The system SHALL use a full-first safety-backup policy before any restore overwrites active data.

#### Scenario: Full safety backup succeeds
- **WHEN** the user confirms restore after preflight
- **THEN** the system creates a full safety backup of current state in the app data directory
- **AND** the safety backup path is logged
- **AND** restore proceeds only after safety backup succeeds

#### Scenario: Full safety backup cannot complete
- **WHEN** full safety backup creation fails or cannot complete (for example insufficient disk space or user-soft-threshold constraints)
- **THEN** the UI presents explicit choices: retry full backup, continue with lightweight backup, or cancel restore
- **AND** restore proceeds only after explicit user confirmation

#### Scenario: Low-space failure triggers prune-and-retry before fallback options
- **WHEN** full safety backup fails due to insufficient space
- **THEN** the system prunes old automatic safety backups per retention policy and retries full safety backup once
- **AND** fallback options (`retry full`, `continue lightweight`, `cancel`) are shown only if the retry still fails

#### Scenario: User chooses lightweight fallback
- **WHEN** full safety backup cannot complete and user explicitly chooses lightweight fallback
- **THEN** system creates lightweight safety backup
- **AND** UI indicates reduced rollback fidelity (recordings may not be recoverable from safety backup)
- **AND** restore proceeds only after lightweight safety backup succeeds

#### Scenario: Safety backup path fails without fallback confirmation
- **WHEN** full safety backup fails and user does not explicitly choose lightweight fallback
- **THEN** restore is aborted
- **AND** active app data remains unchanged

#### Scenario: Restore confirmation shows current-vs-backup counts
- **WHEN** the user views the restore confirmation dialog
- **THEN** the UI shows current data counts alongside backup data counts
- **AND** the UI shows backup identity metadata (created timestamp, created-with-app version, source platform, includes-recordings flag, archive size)
- **AND** the UI labels the artifact as a Codictate backup (`.codictatebackup`)
- **AND** the UI shows a `Will restore` section and a `Will not restore` section
- **AND** the `Will not restore` section explicitly lists excluded settings (API keys, selected devices, selected model)
- **AND** if recordings are absent (lightweight backup or missing files), the UI explicitly states affected audio clips will be unavailable after restore
- **AND** the UI warns that current data will be replaced

### Requirement: Safety Backup Visibility and Retention
The system SHALL make automatically-created pre-restore safety backups discoverable and manageable for users.

#### Scenario: Safety backup details are shown when created
- **WHEN** pre-restore safety backup succeeds
- **THEN** the UI shows safety backup path, created timestamp, scope (`full` or `lightweight`), and approximate size
- **AND** the UI provides actions to reveal the file location and copy the path

#### Scenario: User manages stored safety backups
- **WHEN** user opens backup/restore settings
- **THEN** the UI shows recent safety backups with path, timestamp, scope, and size
- **AND** the UI allows explicit user deletion of selected safety backups

#### Scenario: Automatic retention prunes old safety backups
- **WHEN** a new safety backup is created
- **THEN** the system applies a default automatic retention policy that keeps the newest 3 automatic safety backups
- **AND** older automatic safety backups are pruned first when creating a new automatic safety backup or when low-space precheck runs
- **AND** explicitly saved safety backups are preserved and never auto-pruned
- **AND** retention behavior is disclosed in backup/restore settings

#### Scenario: User chooses whether to keep newly-created automatic safety backup
- **WHEN** restore completes and an automatic safety backup exists
- **THEN** the UI offers explicit choices `Save safety backup` and `Discard safety backup`
- **AND** the default action is `Save safety backup`

#### Scenario: User discards automatic safety backup
- **WHEN** user chooses `Discard safety backup`
- **THEN** the app asks for explicit confirmation before deletion
- **AND** on confirmation, the automatic safety backup is deleted and the UI confirms deletion

### Requirement: Background Operation Continuation
The system SHALL continue backup/restore operations when the settings window closes and SHALL provide clear lifecycle feedback.

#### Scenario: Operation continues when settings window closes
- **WHEN** a backup or restore is running and the user closes/minimizes settings
- **THEN** the operation continues in background
- **AND** the app provides ongoing status and completion/failure notification via tray or desktop notification

#### Scenario: Progress reattaches after reopening settings
- **WHEN** the user reopens settings during an in-progress operation
- **THEN** the backup/restore UI reattaches to live progress state without restarting the operation

#### Scenario: Tray icon hidden still preserves status visibility
- **WHEN** backup/restore is active in background and tray icon is disabled
- **THEN** the app provides desktop notification with an explicit action to open restore/backup status
- **AND** operation progress/completion is not silently hidden

#### Scenario: User attempts app quit during cancellable phase
- **WHEN** the user attempts to quit while backup/restore is active in a cancellable phase
- **THEN** the app requires explicit choice to keep running in background or cancel-and-quit
- **AND** the app does not silently terminate the operation

#### Scenario: User attempts app quit during non-cancellable commit/rollback phase
- **WHEN** the user attempts to quit while restore is in non-cancellable commit or rollback phase
- **THEN** the app requires explicit choice to keep running in background or quit automatically when the phase becomes safe
- **AND** `cancel-and-quit` is not offered for that phase

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

### Requirement: Progress and ETA Reporting
The system SHALL emit incremental progress events during export and restore for frontend display.

#### Scenario: Export progress events
- **WHEN** backup export is in progress
- **THEN** the system emits progress events with phase, current count, total count, and estimated seconds remaining
- **AND** the frontend displays a progress bar with ETA

#### Scenario: Restore progress events
- **WHEN** restore is in progress
- **THEN** the system emits progress events with phase, current count, total count, and estimated seconds remaining
- **AND** the frontend displays a progress bar with ETA

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
- **THEN** `info` logs are emitted at preflight start, safety backup creation, migration, staging, swap completion, and restore completion with counts and duration
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
