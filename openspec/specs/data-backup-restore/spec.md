# data-backup-restore Specification

## Purpose
TBD - created by archiving change add-user-data-backup-restore. Update Purpose after archive.
## Requirements
### Requirement: Single-Archive Backup Artifact (V1)
The system SHALL export backups as a single `.codictatebackup` file.

#### Scenario: User exports backup
- **WHEN** the user confirms backup export
- **THEN** the system writes exactly one `.codictatebackup` archive file

### Requirement: Archive Layout Contract (V1)
The system SHALL produce a fixed v1 archive layout and SHALL consume that layout with documented recoverable exceptions for non-critical payload damage.

#### Scenario: Complete backup layout
- **WHEN** backup choice is complete backup
- **THEN** archive includes required core payloads: `manifest.json`, `checksums.sha256`, `history/history.jsonl`, `dictionary/dictionary.json`, `user/user_store.json`, and `recordings/`

#### Scenario: Smaller backup layout
- **WHEN** backup choice is smaller backup
- **THEN** archive includes all required payload files
- **AND** archive excludes `recordings/`

#### Scenario: Restore with damaged user-store payload
- **WHEN** restore input is missing or has malformed `user/user_store.json`
- **THEN** restore treats the issue as recoverable using local-preserve/default fallback behavior
- **AND** restore does not treat that condition as a required-core-file blocking failure

### Requirement: Manifest and Checksum Integrity
The system SHALL include a manifest and checksums in backups and SHALL verify checksums before restore writes.

#### Scenario: Manifest is present with required metadata
- **WHEN** export completes
- **THEN** `manifest.json` includes format version, creation timestamp, app version, `platform`, `estimated_size_bytes`, component versions, and component counts

#### Scenario: Checksum mismatch blocks restore
- **WHEN** any payload hash does not match `checksums.sha256`
- **THEN** restore is rejected before active-data replacement

#### Scenario: Missing checksum entry blocks restore
- **WHEN** any archive payload file is not listed in `checksums.sha256`
- **THEN** restore is rejected before active-data replacement

### Requirement: Integrity Scope Disclosure (V1)
The system SHALL disclose that checksums detect corruption evidence but do not provide trusted-origin authenticity in v1.

#### Scenario: Restore preflight messaging
- **WHEN** preflight is shown to user
- **THEN** default preflight messaging stays focused on user-facing restore outcome and safety status
- **AND** technical checksum/authenticity disclosure remains available in non-primary disclosure surfaces (for example optional details or docs)
- **AND** messaging does not claim origin authenticity

### Requirement: Logical History Export
The system SHALL export history as logical payload records instead of raw SQLite file copies.

#### Scenario: Export runs with active local database
- **WHEN** backup export runs
- **THEN** history is serialized to `history/history.jsonl`
- **AND** raw `history.db` is not copied into archive

### Requirement: Streaming Processing for Large Datasets
The system SHALL process history and file payloads in streaming/chunked form to keep memory bounded for large-user datasets.

#### Scenario: Export streams history rows
- **WHEN** backup export serializes history
- **THEN** rows are written incrementally to `history/history.jsonl`
- **AND** implementation does not require loading full history into memory

#### Scenario: Restore streams history rows
- **WHEN** restore imports `history/history.jsonl`
- **THEN** rows are processed incrementally
- **AND** implementation does not require loading full history payload into memory

#### Scenario: Archive I/O is chunked
- **WHEN** packaging or extracting large payload files
- **THEN** file I/O uses chunked reads/writes
- **AND** implementation does not require loading full payload files into memory

### Requirement: Backup Choice Options (V1)
The system SHALL support complete and smaller backup scopes in v1 and SHALL present scope selection through a single create-backup modal.

#### Scenario: Include recordings selected
- **WHEN** user opens create-backup modal and keeps `Include recordings` enabled
- **THEN** export includes history, dictionary, user-store data, and available recordings

#### Scenario: Include recordings deselected
- **WHEN** user opens create-backup modal and disables `Include recordings`
- **THEN** export includes history, dictionary, and user-store data
- **AND** recordings are excluded

#### Scenario: Scope change updates size estimate
- **WHEN** user toggles `Include recordings` in create-backup modal
- **THEN** UI updates the estimated backup size for the current scope
- **AND** UI shows approximate size savings for smaller backup

### Requirement: Backup/Restore Settings Placement and Surface
The system SHALL place backup/restore controls in General settings and keep the default card surface uncluttered.

#### Scenario: Placement in General settings
- **WHEN** user opens the Settings page
- **THEN** backup/restore controls appear in `General` after the `Advanced` group
- **AND** History page does not render backup/restore controls

#### Scenario: Primary actions are minimal
- **WHEN** backup/restore card is rendered in idle state
- **THEN** card shows only `Create backup` and `Restore backup` as primary actions
- **AND** maintenance and encryption notices are not shown preemptively

#### Scenario: Visual style matches settings subsections
- **WHEN** backup/restore controls are rendered in General settings
- **THEN** layout follows existing settings subsection patterns (section heading + divided rows)
- **AND** primary action buttons use balanced neutral emphasis consistent with neighboring settings controls

### Requirement: Backup Dialog Start Location Memory
The system SHALL remember the last user-selected backup folder and reuse it as dialog start location for create and restore flows.

#### Scenario: First-time restore opens with OS default
- **WHEN** user opens restore dialog without any remembered backup folder
- **THEN** system does not force a custom start path
- **AND** native dialog uses OS default location

#### Scenario: Remembered folder is reused
- **WHEN** user selects a backup file path or save path in folder `X`
- **THEN** system remembers folder `X`
- **AND** the next create-backup save dialog and restore-backup open dialog start in folder `X`

#### Scenario: Invalid remembered folder does not block flow
- **WHEN** remembered folder data is missing, malformed, or unavailable
- **THEN** system falls back to safe default dialog behavior
- **AND** backup/restore operations continue without hard failure

### Requirement: Missing Recording Handling During Export
The system SHALL tolerate missing referenced recordings during complete backup export and SHALL record warnings.

#### Scenario: Missing recording file
- **WHEN** a referenced recording file is absent during complete backup export
- **THEN** export completes successfully
- **AND** missing files are reported in warnings

### Requirement: Restore Preflight Validation
The system SHALL run non-destructive preflight before restore apply and SHALL classify findings as blocking or recoverable.

#### Scenario: Blocking findings exist
- **WHEN** preflight finds any blocking issue
- **THEN** restore apply is not allowed
- **AND** active data remains unchanged

#### Scenario: Recoverable findings only
- **WHEN** preflight finds only recoverable issues
- **THEN** restore may proceed
- **AND** default user-facing output uses plain-language summary

#### Scenario: Current local history data is corrupted
- **WHEN** preflight detects current local history data corruption
- **THEN** restore apply is blocked to avoid overwriting potentially recoverable local data
- **AND** user-facing output instructs user to repair local data before retrying restore

### Requirement: Restore Preflight Summary Confirmation
The system SHALL show a plain-language backup summary and user-impact explanation before restore confirmation.

#### Scenario: User reviews restore summary
- **WHEN** preflight succeeds and user is about to confirm replace-only restore
- **THEN** UI shows a plain-language summary of what will be restored (history, dictionary, recordings counts) and backup created time
- **AND** app version and source platform are available in optional details that are hidden by default
- **AND** compatibility/integrity technical notes are not shown as primary content in the default preflight view
- **AND** UI clearly states that app settings are not part of backups and current settings remain unchanged

#### Scenario: Fresh install impact copy
- **WHEN** local app data is empty and user opens restore preflight
- **THEN** preflight messaging states that restore will set up the app from backup data
- **AND** messaging does not imply replacement of existing local data

#### Scenario: Backup excludes recordings while local recordings exist
- **WHEN** preflight summary indicates recordings are excluded and local recordings exist
- **THEN** preflight shows an inline warning that local recordings will be removed if restore proceeds
- **AND** warning includes the local recording file count when available

#### Scenario: Restore confirmation CTA remains stable
- **WHEN** restore preflight is ready for confirmation
- **THEN** confirm action label remains stable as `Start Restore`
- **AND** destructive impact details are communicated in preflight body copy rather than conditional button labels

#### Scenario: User receives explicit file-selection guidance
- **WHEN** restore action is available in backup/restore settings
- **THEN** UI provides plain-language guidance that restore expects a `.codictatebackup` file
- **AND** native file picker copy explicitly indicates `.codictatebackup` extension pattern

### Requirement: Archive Extraction Safety
The system SHALL reject unsafe archive entries before extraction.

#### Scenario: Path traversal is present
- **WHEN** archive contains parent traversal paths
- **THEN** restore is rejected before extraction

#### Scenario: Absolute path is present
- **WHEN** archive contains absolute paths
- **THEN** restore is rejected before extraction

#### Scenario: Windows drive-prefixed path is present
- **WHEN** archive contains drive-prefixed paths such as `C:/...`
- **THEN** restore is rejected before extraction

#### Scenario: Control-character path segment is present
- **WHEN** archive contains path segments with control characters (for example newline or DEL)
- **THEN** restore is rejected before extraction

#### Scenario: Symlink or hardlink is present
- **WHEN** archive contains symlink or hardlink entries
- **THEN** restore is rejected before extraction

#### Scenario: Duplicate or case-collision path is present
- **WHEN** archive contains duplicate normalized relative paths or case-fold-colliding paths
- **THEN** restore is rejected before extraction

### Requirement: Basic Restore Resource Bounds
The system SHALL enforce practical hard bounds during restore preflight/import.

#### Scenario: Archive or payload exceeds hard size limits
- **WHEN** archive size or payload file size exceeds configured hard bounds
- **THEN** restore is rejected as blocking
- **AND** active data remains unchanged

#### Scenario: History row count exceeds hard limit
- **WHEN** imported history rows exceed configured hard bound
- **THEN** restore is rejected as blocking
- **AND** active data remains unchanged

#### Scenario: Archive entry count exceeds hard limit
- **WHEN** archive entry count exceeds configured hard bound
- **THEN** restore is rejected as blocking
- **AND** active data remains unchanged

#### Scenario: Total uncompressed payload bytes exceed hard limit
- **WHEN** total uncompressed payload bytes exceed configured hard bound
- **THEN** restore is rejected as blocking
- **AND** active data remains unchanged

#### Scenario: Bound supports current product defaults
- **WHEN** restore input size matches currently supported product-scale history limits
- **THEN** restore is not rejected by row-count bound alone

### Requirement: Restore Free-Space Preflight Gate
The system SHALL verify available disk space before restore apply begins.

#### Scenario: Not enough free space for restore apply
- **WHEN** required free space for extraction/staging/snapshot exceeds available disk space
- **THEN** restore is rejected as blocking before any active-data writes
- **AND** preflight output includes plain-language required vs available disk values

### Requirement: Compatibility Contract for V1 Upgrades
The system SHALL guarantee format-major `1` restore compatibility across macOS v1 app upgrades and treat cross-platform restore as best-effort in v1.

#### Scenario: Backup from earlier v1 release
- **WHEN** a backup is created by an earlier macOS v1 app release with supported payload versions
- **THEN** a later macOS v1 app release can restore it subject to normal validation

#### Scenario: Cross-platform restore attempt
- **WHEN** a backup is restored on a different OS platform than it was created on
- **THEN** restore is treated as best-effort in v1
- **AND** user-facing messaging does not claim guaranteed cross-platform compatibility

#### Scenario: Breaking payload change introduced
- **WHEN** a release introduces a breaking payload schema change
- **THEN** the release includes explicit version bump and migration path before shipping

### Requirement: Additive Schema Tolerance (V1)
The system SHALL keep v1 restore forward-tolerant for additive JSON changes.

#### Scenario: Backup includes unknown additive fields
- **WHEN** manifest or payload JSON includes unknown additive fields
- **THEN** restore ignores those fields and continues normal validation

#### Scenario: Backup omits optional fields
- **WHEN** optional manifest or payload fields are absent
- **THEN** restore applies documented defaults and continues normal validation

### Requirement: Maintenance Mode During Backup/Restore
The system SHALL run backup and restore as exclusive maintenance operations and SHALL communicate temporary app unavailability.

#### Scenario: Operation start while recording/transcribing
- **WHEN** user starts backup/restore while recording or transcription is active
- **THEN** operation is blocked with plain-language guidance to stop active work first

#### Scenario: Backup or restore in progress
- **WHEN** backup/restore is running
- **THEN** new transcription/history/dictionary/profile writes are blocked until operation finalization
- **AND** create-backup modal explains that core app actions are temporarily unavailable during operations

### Requirement: Replace-Only Staged Restore (V1)
The system SHALL restore through staging and replace active data only after staging validation succeeds.

#### Scenario: Successful staged restore
- **WHEN** staging import and validation succeed
- **THEN** staged data replaces active restore-managed data

#### Scenario: Staging failure
- **WHEN** restore fails before successful swap
- **THEN** active restore-managed data remains unchanged

### Requirement: Pre-Swap Rollback Snapshot
The system SHALL create a rollback snapshot before destructive swap and SHALL use it on failure.

#### Scenario: Snapshot created before swap
- **WHEN** restore enters commit path
- **THEN** a rollback snapshot of active data is created and validated first

#### Scenario: Failure after snapshot
- **WHEN** restore fails after snapshot creation and before final success
- **THEN** rollback restores pre-restore active data from snapshot

### Requirement: Startup Reconciliation (V1)
The system SHALL reconcile interrupted restores using a commit-aware marker model.

#### Scenario: Uncommitted restore marker found
- **WHEN** startup finds a restore marker in `in_progress` state
- **THEN** system rolls back from snapshot path
- **AND** clears the marker before normal operation continues

#### Scenario: Committed restore marker found
- **WHEN** startup finds a restore marker in `committed` state
- **THEN** system keeps restored active data
- **AND** clears marker and stale rollback artifacts without rolling back

### Requirement: Undo Last Restore Safety Net (V1)
The system SHALL attempt to retain one short-lived post-restore checkpoint and SHALL allow one-click undo while that checkpoint is available.

#### Scenario: Successful restore keeps one checkpoint when publish succeeds
- **WHEN** restore commit succeeds
- **THEN** the system attempts to retain the pre-swap snapshot as the `Undo Last Restore` checkpoint
- **AND** any older undo checkpoint is replaced

#### Scenario: Checkpoint publish failure does not invalidate restore
- **WHEN** restore commit succeeds but checkpoint publish fails
- **THEN** restore remains successful
- **AND** UI receives a warning that undo is unavailable for that restore run

#### Scenario: User undoes restore within retention window
- **WHEN** user triggers `Undo Last Restore` while checkpoint is present and not expired
- **THEN** system applies replace-only restore from that checkpoint
- **AND** checkpoint is cleared after successful undo

#### Scenario: Undo operation shows progress feedback
- **WHEN** user triggers `Undo Last Restore` while checkpoint is present and not expired
- **THEN** UI immediately shows an in-progress status in the Backup & Restore card
- **AND** UI shows a progress indicator and phase updates until undo completes or fails

#### Scenario: Undo checkpoint unavailable
- **WHEN** user requests undo after checkpoint expiry, replacement, or checkpoint corruption
- **THEN** system keeps current active data unchanged
- **AND** UI hides the undo action from default card surface

### Requirement: Durable Restore Commit Boundaries
The system SHALL use durable sync boundaries around restore marker transitions and destructive swap.

#### Scenario: Marker state is durably persisted
- **WHEN** restore writes `in_progress` or `committed` marker state
- **THEN** marker file and parent directory are synced before proceeding to the next destructive step

#### Scenario: Crash near swap boundary
- **WHEN** process interruption occurs near marker/swap boundary
- **THEN** startup reconciliation behavior is determined by durably persisted marker state

### Requirement: Settings Excluded from Backup (V1)
The system SHALL exclude app settings/preferences from backup payloads in v1, and restore SHALL keep current install settings unchanged.

#### Scenario: Backup excludes settings payload
- **WHEN** backup export writes archive payload files
- **THEN** archive does not include `settings/settings.json` or equivalent settings payload

#### Scenario: Restore preserves current settings
- **WHEN** backup restore is applied
- **THEN** app settings/preferences remain those of the current install

### Requirement: Dictionary Backup Is Independent of Settings
The system SHALL back up and restore dictionary data from the dedicated dictionary payload and SHALL NOT route dictionary restore through app settings payload.

#### Scenario: Dictionary restore path
- **WHEN** restore applies `dictionary/dictionary.json`
- **THEN** dictionary entries are restored from that payload
- **AND** current install settings remain unchanged

### Requirement: User Store Preservation
The system SHALL back up `user_store.json` and SHALL restore onboarding/profile/growth state from it when valid.

#### Scenario: User migrates to a new machine
- **WHEN** backup is restored on a new install
- **THEN** onboarding/profile/growth state from `user/user_store.json` is restored

#### Scenario: User-store payload is missing or malformed
- **WHEN** restore input has missing or malformed `user/user_store.json`
- **THEN** restore preserves current local onboarding/profile/growth state when available
- **AND** restore uses default onboarding/profile/growth state only when local state is absent
- **AND** restore reports a recoverable warning in plain language

### Requirement: User Stats Handling on Restore
The system SHALL recompute stats from restored history rows in v1.

#### Scenario: Stats are rebuilt from history
- **WHEN** restore imports `history/history.jsonl`
- **THEN** stats are recomputed from restored history rows before active-data replacement

### Requirement: Operation Concurrency Guard
The system SHALL allow at most one backup/restore operation at a time.

#### Scenario: Concurrent operation requested
- **WHEN** a backup or restore is already running
- **THEN** a second backup/restore request is rejected as busy

### Requirement: Cancellation Safety
The system SHALL support cancellation without corrupting active data.

#### Scenario: Export canceled
- **WHEN** user cancels export
- **THEN** temp/output partial artifacts are cleaned up safely

#### Scenario: Restore canceled before commit
- **WHEN** user cancels restore before final swap completes
- **THEN** active restore-managed data remains unchanged
- **AND** staging artifacts are cleaned up safely

### Requirement: Missing Audio Behavior on Restore
The system SHALL preserve text restore even when recordings are absent and SHALL fail audio playback gracefully.

#### Scenario: Restore without recordings
- **WHEN** restore source has no recordings
- **THEN** history text, dictionary, and user-store state are restored
- **AND** unavailable audio playback returns a user-facing unavailable message

### Requirement: Progress Reporting
The system SHALL emit incremental progress events for export and restore.

#### Scenario: Export progress
- **WHEN** export is running
- **THEN** progress events include step progress suitable for a progress bar

#### Scenario: Restore progress
- **WHEN** restore is running
- **THEN** progress events include step progress suitable for a progress bar

### Requirement: Unencrypted Backups in V1
The system SHALL keep backups unencrypted in v1 and SHALL disclose this in user-facing flow.

#### Scenario: Export flow disclosure
- **WHEN** backup export succeeds
- **THEN** UI shows informational notice that backup archives are not app-encrypted in v1

### Requirement: Path Scope Restriction
The system SHALL restrict backup/restore file operations to user-selected archive paths and app-managed data directories.

#### Scenario: Out-of-scope write attempt
- **WHEN** backup or restore resolves a write target outside allowed scope
- **THEN** operation is rejected as unsafe

### Requirement: Least-Privilege Tauri Scope
The system SHALL constrain backup/restore command access to least privilege in Tauri.

#### Scenario: Command invocation from non-main window
- **WHEN** backup/restore command is invoked from a window other than `main` (for example `recording_overlay`)
- **THEN** invocation is rejected as unauthorized

#### Scenario: Explicit capability identifier list
- **WHEN** app security configuration is built
- **THEN** `tauri.conf.json` explicitly lists enabled capability identifiers
- **AND** backup/restore permissions are assigned through a dedicated main-window capability

#### Scenario: macOS target capability permissions
- **WHEN** backup/restore capability is configured for the v1 target release
- **THEN** capability scope is constrained to macOS main-window usage
- **AND** plugin permissions are limited to native open/save dialog access for the UI flow

### Requirement: Internationalization for Backup/Restore UI
The system SHALL use i18next translation keys for backup/restore user-facing strings.

#### Scenario: UI strings are translated
- **WHEN** backup/restore UI is rendered
- **THEN** labels, warnings, confirmations, and status messages use `t()` keys under `settings.backup`

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

