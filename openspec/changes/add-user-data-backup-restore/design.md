## Context
Current user data is split across:
- `history.db` (SQLite history + stats)
- `recordings/` (WAV files referenced by history rows)
- `settings_store.json` (dictionary and user preferences inside app settings payload)

For local-only apps, users still need portable ownership of their data across uninstall/reinstall and machine changes. Directly copying live SQLite files is fragile, especially with journaling/WAL side files, so backup must be a controlled app workflow.

## V1 Implementation Rules
All implementation tasks MUST satisfy these rules.

- `BR-001` Archive type: backup output MUST be one `.codictatebackup` file using ZIP container format.
- `BR-002` Archive structure: archive MUST include `manifest.json`, `checksums.sha256`, `history/history.jsonl`, `history/user_stats.json`, `dictionary/dictionary.json`, `settings/settings.json`; `recordings/` is optional.
- `BR-003` Manifest contract: `manifest.json` MUST include `backup_format_version`, `created_at`, `created_with_app_version`, `platform`, `components`, `includes_recordings`, `estimated_size_bytes`, and per-component counts.
- `BR-004` Checksums: `checksums.sha256` MUST include SHA-256 for every payload file present in archive except the checksum file itself.
- `BR-005` Export format: history MUST be exported as logical records (JSONL) rather than raw SQLite file copy.
- `BR-006` Backup scopes: v1 MUST support exactly two export scopes: `full` (with recordings) and `lightweight` (without recordings). Both scopes MUST include history, dictionary, and settings payloads.
- `BR-007` Missing audio at export: if full backup is requested but some referenced audio files are missing, export MUST complete, record warning metadata, and exclude missing files.
- `BR-008` Preflight-first restore: restore MUST run non-destructive preflight before any live data write and MUST classify findings into `blocking` vs `recoverable`.
- `BR-009` Compatibility policy: restore MUST support all known payload versions within backup format major `1`; format-major transitions MUST support prior-major imports for at least 24 months with explicit deprecation runway.
- `BR-010` Migration policy: restore MUST run component migrations (history payload, dictionary payload, and settings payload) to current import schemas before staging import.
- `BR-011` Restore mode: v1 restore MUST be replace-only.
- `BR-012` Atomic safety: restore MUST stage all writes and only swap into active app data after staging validates.
- `BR-013` Rollback: if any step fails before final success, active data MUST remain unchanged.
- `BR-014` Audio restore semantics: text history restore MUST succeed even when archive has no recordings; missing audio playback must fail gracefully with user-facing message.
- `BR-015` Result reporting: export and restore MUST return machine-readable summary reports (counts, warnings, failures) plus concise user-facing summary text; detailed diagnostics are optional behind explicit UI action.
- `BR-016` Concurrency guard: backup/restore operations MUST acquire an app-level mutex so only one top-level operation runs at a time.
- `BR-017` Encryption scope: v1 backups MUST be unencrypted by app logic; UI MUST disclose this before export confirmation.
- `BR-018` Archive extraction safety: restore MUST validate archive entry metadata before extraction and reject archives containing absolute paths, parent-directory traversal (`..`), drive-prefixed paths, or symlink/hardlink entries.
- `BR-019` Storage capacity precheck: export and restore MUST estimate required free disk space for temp workspace **and destination** and fail fast with actionable error if insufficient.
- `BR-020` Cancellation safety: canceling backup/restore MUST clean temporary files and MUST NOT partially modify active restore-managed data. UI MUST expose phase-aware quit/cancel options and clearly indicate non-cancellable phases.
- `BR-021` Crash/interruption recovery: restore MUST persist an in-progress marker and reconcile on next app start to either complete rollback or clear safe staging leftovers.
- `BR-022` Strict core payload validation: restore MUST treat malformed/invalid required payload content (history rows, dictionary payload, settings payload) as blocking corruption and abort before active-data replacement. Optional recording-file issues remain recoverable with warnings.
- `BR-023` Path scope restriction: export and restore file operations MUST be restricted to user-selected archive paths and app-managed data directories.
- `BR-024` Pre-swap rollback snapshot: before swapping staged restore data into active app paths, the system MUST preserve current active restore-managed data in a local rollback workspace. On failure or interruption, reconciliation MUST restore this snapshot automatically. After successful restore and reconciliation, rollback workspace cleanup MUST follow a defined retention window.
- `BR-025` User stats restore: restored `user_stats` MUST be recomputed from the imported history rows (recalculating `total_words`, `total_duration_ms`, `total_transcriptions`, `transcription_dates`, `total_filler_words_removed`) rather than blindly importing the backup's stats snapshot, so that stats are always consistent with the actual restored history.
- `BR-026` Selective settings backup: backup MUST export user-configurable preferences from `AppSettings` (shortcuts, language, overlay position, audio feedback, paste method, etc.) but MUST exclude sensitive fields (`post_process_api_keys`) and device-specific fields (`selected_microphone`, `clamshell_microphone`, `selected_output_device`, `selected_model`). The exported settings payload MUST include a `settings_payload_version`.
- `BR-027` Forward-compatible settings restore: on restore, settings MUST be merged selectively into the current `AppSettings`. Fields present in the backup but absent in the current schema are ignored. Fields present in the current schema but absent in the backup retain their current (or default) values. This ensures backups from older app versions restore cleanly on newer versions.
- `BR-028` Progress reporting: export and restore operations MUST emit incremental progress events to the frontend with `{ phase, current, total }` at each major step (snapshot, file copy, staging, swap). `estimated_seconds_remaining` is optional best-effort metadata. The frontend MUST display a progress bar.
- `BR-029` Estimated size preview: before export begins, the system MUST calculate and display the estimated archive size based on current history row count, recording file sizes, and dictionary size. The user MUST confirm after seeing the estimate and destination-space check result. Export save flow MUST prefill a Codictate filename, auto-append `.codictatebackup` when omitted, and require overwrite confirmation.
- `BR-030` Cross-platform universal backup: archive entries MUST use forward-slash (`/`) path separators regardless of the creating platform. On restore, filenames MUST be normalized to NFC Unicode form. Manifest MUST record the source `platform` field. Invalid or unsafe names in required payload paths are blocking failures; invalid recording filenames are recoverable and MUST be deterministically sanitized or skipped with warning.
- `BR-031` Structured logging: every major export/restore milestone MUST emit structured `tracing` logs at `info!` level. All recoverable warnings MUST emit `warn!` logs. All failures MUST emit `error!` logs with context. Frontend backup/restore actions MUST log via `logInfo`/`logError` to the unified log target `fe-backup`.
- `BR-032` Internationalization: all user-facing backup/restore strings (UI labels, warnings, confirmations, progress messages, error messages) MUST use i18next translation keys. Keys MUST be added to `src/i18n/locales/en/translation.json` under a `settings.backup` namespace.
- `BR-033` Recoverable optional-file handling: if preflight/import detects recoverable issues limited to optional recording files, restore MUST continue automatically, summarize skipped items concisely, and keep detailed diagnostics behind optional explicit UI action.
- `BR-034` Restore quiesce mode: before apply-restore starts, the app MUST enter restore lock mode that blocks new transcription/history writes until restore completes, fails, or rolls back. Blocked actions MUST show clear user-facing status.
- `BR-035` Archive resource bounds (hard-only): restore MUST enforce hard non-overridable security bounds for unsafe/decompression-abuse risk (for example compression-ratio explosion and absolute upper parser bounds). All JSON/JSONL parsing MUST remain bounded streaming parsing.
- `BR-036` Integrity scope disclosure: v1 checksum verification MUST be documented and surfaced as corruption/tamper detection only; it MUST NOT claim trusted-origin authenticity because v1 does not include signatures or MAC.
- `BR-037` Log redaction and privacy: backup/restore logs MUST NOT include transcript text, prompt text, API keys, or raw payload bodies. Logs MUST use metadata-only fields (counts, durations, non-sensitive identifiers).
- `BR-038` Restore-managed dataset contract: rollback snapshot, destructive swap, and startup reconciliation MUST operate on an explicit dataset contract that includes `history.db` (including stats tables), dictionary source state in `settings_store.json`, and restore-managed `recordings/` paths.
- `BR-039` Atomic same-volume swap contract: destructive restore swap MUST use same-volume atomic renames between active, rollback, and staging paths. If same-volume atomic rename cannot be guaranteed, restore MUST fail before destructive phase. Copy/delete swap semantics are forbidden.
- `BR-040` Restore marker phase model: restore marker state MUST track deterministic phases `snapshot_ready`, `active_moved`, `staged_activated`, `completed`; startup reconciliation MUST map each phase to one deterministic recovery action.
- `BR-041` Export snapshot consistency fence: export MUST briefly quiesce history/settings writes while capturing a single point-in-time snapshot for history rows, stats, and dictionary/settings payload. The fence MUST be released immediately after snapshot capture.
- `BR-042` Rollback retention policy: latest pre-restore rollback snapshot MUST be retained for 7 days with best-effort cleanup at startup and after successful restore.

## Data Contracts

### Archive Layout
```text
<name>.codictatebackup
├── manifest.json
├── checksums.sha256
├── history/
│   ├── history.jsonl
│   └── user_stats.json
├── dictionary/
│   └── dictionary.json
├── settings/
│   └── settings.json
└── recordings/                    # present only when includes_recordings=true
    ├── codictate-1734959931.wav
    └── ...
```

### Restore-Managed Active Dataset (V1)
Rollback snapshot, restore swap, and startup reconciliation operate over this explicit active dataset:
- `history.db` (history rows plus stats tables)
- `settings_store.json` source-of-truth file that contains dictionary payload and user preferences
- restore-managed `recordings/` subtree referenced by history rows

Any destructive restore step MUST treat this dataset as one unit (`BR-038`).

### Restore Marker Phases (V1)
Restore marker uses deterministic phases (`BR-040`):
- `snapshot_ready`: rollback snapshot verified; destructive swap not started
- `active_moved`: active dataset moved to rollback location; staged data not yet activated
- `staged_activated`: staged dataset moved into active location
- `completed`: restore commit and post-commit checks completed

Startup reconciliation behavior:
- `snapshot_ready` -> restore/verify pre-restore active dataset from rollback snapshot
- `active_moved` -> restore active dataset from rollback snapshot
- `staged_activated` -> finalize committed staged dataset and clear rollback workspace safely
- `completed` -> no recovery action; cleanup-only path

### Manifest (V1) Shape
```json
{
  "backup_format_version": "1.0.0",
  "created_at": "2026-02-14T21:55:00Z",
  "created_with_app_version": "0.9.3",
  "platform": "macos",
  "includes_recordings": true,
  "estimated_size_bytes": 52428800,
  "components": {
    "history_payload_version": 1,
    "dictionary_payload_version": 1,
    "settings_payload_version": 1
  },
  "counts": {
    "history_entries": 312,
    "recording_files": 288,
    "dictionary_entries": 47,
    "settings_fields": 25
  },
  "warnings": {
    "missing_recordings": [
      "codictate-1734959931.wav"
    ]
  }
}
```

### Settings Export Shape (V1)
Settings are exported selectively. The following fields are **included**:
- `bindings`, `audio_feedback`, `audio_feedback_volume`, `sound_theme`
- `start_hidden`, `autostart_enabled`, `update_checks_enabled`
- `always_on_microphone`, `translate_to_english`, `selected_language`, `saved_languages`
- `overlay_position`, `log_level`, `model_unload_timeout`
- `word_correction_threshold`, `word_correction_split_threshold`
- `history_limit`, `recording_retention_period`, `paste_method`, `clipboard_handling`
- `auto_submit`, `auto_submit_key`, `post_process_enabled`, `post_process_provider_id`
- `post_process_providers`, `post_process_models`, `post_process_prompts`, `post_process_selected_prompt_id`
- `auto_refine_enabled`, `mute_while_recording`, `append_trailing_space`, `app_language`
- `enable_filler_word_filter`, `enable_hallucination_filter`
- `show_tray_icon`, `show_unload_model_in_tray`
- `paste_delay_ms`, `paste_restore_delay_ms`, `typing_tool`

The following fields are **excluded**:
- `post_process_api_keys` (sensitive)
- `selected_microphone`, `clamshell_microphone`, `selected_output_device` (device-specific)
- `selected_model` (device-specific, depends on locally downloaded models)
- `debug_mode` (runtime state)

### Checksum File
`checksums.sha256` lines follow:
`<hex_sha256><two_spaces><relative_path>`

Example:
```text
0a...ff  manifest.json
b1...3d  history/history.jsonl
...
```

### Resource Limits (V1 Defaults)
Restore preflight/import uses hard bounded limits to prevent resource exhaustion.

Hard security bounds (non-overridable):
- `hard_max_archive_entries`: 2_000_000
- `hard_max_entry_uncompressed_bytes`: 34_359_738_368 (32 GiB)
- `hard_max_total_uncompressed_bytes`: 214_748_364_800 (200 GiB)
- `hard_max_compression_ratio`: 200x for any single entry
- `hard_max_history_jsonl_line_bytes`: 16_777_216 (16 MiB)
- `hard_max_history_rows`: 50_000_000

Behavior:
- hard-bound breach: blocking failure (`BR-035`)
- bounded parsing is always enforced

### Operation Report Notes (V1)
- `PreflightRestoreReport` default summary MUST include `backup_created_at` and `backup_history_entries`; fuller metadata remains optional under explicit details view.
- `StartupRestoreReconciliationReport` SHOULD standardize `outcome` as one of `rolled_back`, `committed`, `no_action`, with phase/source context for diagnostics.

## Export Algorithm (V1)
1. Acquire backup mutex (`BR-016`).
2. Calculate estimated archive size, prefill Codictate default filename, and display export confirmation (`BR-029`).
3. Enter short export snapshot fence and block new history/settings writes (`BR-041`).
4. Capture one point-in-time snapshot:
   - open SQLite read transaction for history export (`BR-005`)
   - read dictionary source data from `settings_store.json`
   - read selective settings (excluding sensitive/device-specific fields) (`BR-026`)
   - determine referenced recording filenames from exported rows
5. Release snapshot fence immediately after snapshot capture (`BR-041`).
6. Estimate and verify disk capacity for temp workspace and output target (`BR-019`).
7. Build payload in temp workspace. Emit progress events (`BR-028`).
8. Write logical payload files:
   - `history/history.jsonl`
   - `history/user_stats.json`
   - `dictionary/dictionary.json`
   - `settings/settings.json`
9. If scope is `full`, copy referenced audio files that exist into `recordings/` and track missing files as warnings (`BR-007`). Emit per-file progress events.
10. Normalize all archive entry paths to forward-slash separators and NFC Unicode (`BR-030`).
11. Write `manifest.json` with format/component versions, platform, and counts (`BR-003`).
12. Compute and write `checksums.sha256` (`BR-004`).
13. Package temp workspace into `.codictatebackup` ZIP (`BR-001`).
14. Write destination atomically (temp output then rename).
15. Support cancellation checkpoints and cleanup if canceled (`BR-020`).
16. Log export completion with counts and duration (`BR-031`).
17. Return export report with counts/warnings (`BR-015`).

## Restore Algorithm (V1)
1. Acquire backup mutex (`BR-016`).
2. Enter restore quiesce mode:
   - block new recording/transcription/history-write operations (`BR-034`)
   - verify no in-flight write remains (or abort with busy/retry message)
3. Estimate and verify disk capacity for unpack + staging + rollback workspace (`BR-019`).
4. Preflight archive-entry safety scan (without extracting file contents):
   - reject blocking unsafe entries (`BR-018`)
   - enforce hard security bounds (`BR-035`): absolute parser ceilings and compression-ratio guard
   - classify recoverable name issues in optional recordings (`BR-008`, `BR-030`)
5. Unpack archive to temp workspace only after safety scan passes. Emit progress events (`BR-028`).
6. Normalize extracted filenames to NFC Unicode and validate cross-platform filename safety (`BR-030`).
7. Run preflight validation (`BR-008`):
   - required files present (`BR-002`)
   - manifest parse + version parse (`BR-003`)
   - checksum verification (`BR-004`)
   - include integrity-scope note in preflight result (`BR-036`): checksum validates corruption/tamper evidence only, not trusted origin
   - compatibility gate (`BR-009`)
   - classify findings into `blocking` vs `recoverable`
8. Show concise preflight summary with key impact counts and issue summary:
   - default summary includes `backup_created_at` and `backup_history_entries`
   - detailed metadata (current-vs-backup counts and backup identity) remains available behind explicit `View details`
   - if any `blocking` issue exists, abort restore
   - recoverable optional recording-file issues continue automatically and are summarized (`BR-033`)
9. Create pre-swap rollback snapshot (`BR-024`, `BR-038`):
   - preserve explicit restore-managed active dataset (`history.db`, `settings_store.json`, restore-managed `recordings/`) in rollback workspace
   - validate rollback snapshot can be restored
   - log rollback snapshot path and size (`BR-031`)
10. Persist restore marker in phase `snapshot_ready` (`BR-040`).
11. Run component migration pipeline to current payload schemas (`BR-010`).
12. Build staging target under temp dir:
    - initialize fresh staging `history.db` via current migrations
    - import history rows with strict core validation (`BR-022`)
      - malformed rows and oversize lines in required payload are blocking corruption errors
    - recompute `user_stats` from imported history rows (`BR-025`)
    - import dictionary into staged settings payload
    - merge settings selectively into staged settings, applying forward-compatible defaults (`BR-027`)
    - copy recordings with recoverable optional-file handling (`BR-022`, `BR-030`)
      - invalid/missing optional recording files are skipped with warnings
      - row-to-audio references for skipped files are marked unavailable at runtime
13. Validate staging:
    - SQLite `PRAGMA integrity_check`
    - consistency check: every imported row either has a present audio file or is marked audio-unavailable at runtime (`BR-014`)
14. Validate same-volume atomic swap preconditions (`BR-039`):
   - active, rollback, and staging roots must be on same filesystem
   - if atomic rename cannot be guaranteed, abort before destructive swap
15. Swap data replace-only using atomic renames (`BR-011`, `BR-012`, `BR-039`):
   - rename active dataset to rollback location, then set marker phase `active_moved`
   - rename staged dataset to active location, then set marker phase `staged_activated`
16. If any step fails before completion, run deterministic recovery from marker phase and rollback snapshot (`BR-013`, `BR-040`). Log rollback/recovery (`BR-031`).
17. On successful commit, set marker phase `completed`, clear in-progress marker, and retain rollback snapshot per 7-day policy (`BR-021`, `BR-042`).
18. Support cancellation checkpoints and cleanup (`BR-020`).
19. Exit restore quiesce mode (`BR-034`).
20. Run best-effort rollback workspace cleanup for snapshots older than retention window (`BR-042`).
21. Log restore completion with restored + skipped counts and duration (`BR-031`).
22. Emit refresh events and return restore report (`BR-015`).

## Logging Plan

### Rust Backend (`tracing`)
```rust
// Export milestones
info!(scope = %scope, "Backup export started");
debug!(history_count = count, dict_count = dict_count, settings_fields = fields, "Snapshot read complete");
info!(recordings_found = found, recordings_missing = missing, "Recording scan complete");
info!(archive_size_bytes = size, duration_ms = elapsed, "Backup export completed");

// Restore milestones
info!(archive = %path, "Restore preflight started");
debug!(format_version = %version, app_version = %app_ver, platform = %platform, "Manifest parsed");
warn!(blocking_count = blocking, recoverable_count = recoverable, "Preflight findings classified");
error!(file = %name, "Checksum mismatch detected during preflight");
info!(rollback_snapshot_path = %path, rollback_snapshot_bytes = size, "Pre-swap rollback snapshot created");
info!(marker_phase = "snapshot_ready", "Restore marker advanced");
info!(migration_from = from, migration_to = to, "Running payload migration");
info!(marker_phase = "active_moved", "Active dataset moved to rollback");
info!(marker_phase = "staged_activated", "Staged dataset activated");
info!(history_restored = count, recordings_restored = rec_count, skipped_rows = skipped_rows, skipped_recordings = skipped_files, duration_ms = elapsed, "Restore completed successfully");
error!(step = %step, err = %e, "Restore failed, initiating rollback");
info!("Rollback completed, active data unchanged");

// Crash recovery
info!(marker = %path, "Restore-in-progress marker found at startup");
info!(action = %action, outcome = %outcome, phase = %phase, "Startup reconciliation completed");

// Concurrency
warn!("Backup/restore operation rejected: another operation in progress");
```

Redaction rules (`BR-037`):
- never log transcript text, prompt text, API keys, raw JSON payload lines, or full archive contents
- log only metadata (counts, durations, phase names, stable non-sensitive identifiers)
- when an error references a row/file, include row index or sanitized file token, not full user content

### TypeScript Frontend (`logInfo`/`logError`)
```typescript
logInfo("Backup export initiated", "fe-backup");
logInfo(`Export completed: ${report.counts.history_entries} entries`, "fe-backup");
logError(`Export failed: ${error}`, "fe-backup");
logInfo("Restore preflight initiated", "fe-backup");
logInfo(`Preflight passed: ${report.history_entries} entries`, "fe-backup");
logError(`Restore failed: ${error}`, "fe-backup");
```

## Tauri Capabilities Required
The following Tauri v2 capabilities must be configured for backup/restore:
- `dialog:allow-save` — file-save dialog for export destination
- `dialog:allow-open` — file-open dialog for restore source
- `fs:allow-read` — scoped to app data dir + user-selected archive paths
- `fs:allow-write` — scoped to app data dir + user-selected archive paths
- `fs:allow-mkdir` — for temp workspace creation
- `fs:allow-remove` — for temp workspace cleanup
- Backup/restore permissions SHOULD be defined in a dedicated capability bound to the `main` window only (not overlay windows), with least-privilege scope limited to app-managed data paths and explicit user-selected archive paths.

## Compatibility and Lifecycle
- Backup format semver starts at `1.0.0`.
- Supported restore range for v1 is payload-version based within format major `1` (`BR-009`).
- For future format-major transitions, prior-major imports MUST remain supported for at least 24 months with explicit deprecation notice before removal.
- Unsupported older backups must show migration guidance and an explicit compatibility cutoff explanation.
- Unsupported newer backups must show forward-compatibility guidance: update app first.
- If support for a prior major is removed in a future release, deprecation notice must be shipped at least one stable release earlier and documented in release notes.

## Goals / Non-Goals
- Goals:
  - Provide reliable user-owned local backup and restore.
  - Preserve history text, dictionary, and user settings in all backup scopes.
  - Preserve audio when included and degrade gracefully when absent.
  - Keep restore safe via rollback snapshot, staging, and rollback.
  - Provide progress visibility during long operations (ETA optional best-effort).
  - Ensure backups are universally portable across macOS, Windows, and Linux.
- Non-Goals:
  - Cloud backup/sync.
  - Scheduled backups.
  - Merge-mode restore.
  - App-level encryption or password-protected archive format in v1.

## Risks / Trade-offs
- Large full backups can be slow and large.
  - Mitigation: lightweight scope, explicit scope descriptions, estimated size preview, and progress bar.

- Migration code increases long-term maintenance.
  - Mitigation: payload-version contracts, migration fixture tests, and documented deprecation runway.

- Lightweight backups produce text-only history for audio.
  - Mitigation: explicit UI warning and graceful missing-audio playback behavior.

- Selective settings backup may confuse users about what is/isn't restored.
  - Mitigation: restore confirmation UI shows what will be restored and what won't. Device-specific settings (microphone, model) are clearly marked as excluded.

- Restore is destructive by design.
  - Mitigation: explicit destructive confirmation plus automatic rollback snapshot before final swap.
  - Mitigation: default confirmation always shows backup `created_at` and `history_entries` to reduce wrong-backup restores, while detailed metadata remains optional.
  - Mitigation: keep one confirmation dialog only (no additional decision dialogs).

- Optional recording-file issues can produce incomplete audio playback after restore.
  - Mitigation: auto-skip optional recording file issues with concise summary and graceful missing-audio runtime messaging.

- Checksums without signatures can’t prove trusted origin.
  - Mitigation: explicit integrity-scope disclosure in UI/docs; reserve signed authenticity for a future version.

## Edge Cases Explicitly Covered
- Unsafe archive payload paths and symlink/hardlink extraction attempts.
- Insufficient disk space for export temp output, destination, or restore staging.
- User cancellation at safe checkpoints.
- App/process interruption mid-restore with startup reconciliation.
- Interruption at restore marker phases (`snapshot_ready`, `active_moved`, `staged_activated`) with deterministic reconciliation.
- Corrupted required payload rows/files (blocking restore with actionable errors).
- Invalid or missing optional recording filenames/files (skipped with warnings while preserving text restore).
- Archive resource-exhaustion attempts (oversized entry count, single-entry size, total size, compression ratio, oversized JSONL lines).
- Partial/missing recording files while preserving text history.
- Cross-platform filename characters invalid on target OS.
- Unicode normalization differences between macOS (NFD) and other platforms (NFC).
- Settings schema evolution: new fields added after backup was created get defaults on restore.
- Settings schema evolution: removed fields in backup are silently ignored on restore.
- Empty history export (0 entries produces valid archive with empty JSONL).
- Corrupted SQLite at export time (export fails with actionable error).
- Cross-volume restore swap topologies where atomic rename cannot be guaranteed (blocking before destructive phase).
- Concurrent export writes causing cross-store drift (prevented by short snapshot fence).
- Destination filesystem constraints at export destination (including single-file size limits).
- Save dialog filename/extension mistakes (default Codictate filename + auto-append + overwrite confirmation).
- High warning volume during preflight/restore (concise default summaries with optional detailed drill-in).
- Rollback snapshot retention cleanup after successful restore.
