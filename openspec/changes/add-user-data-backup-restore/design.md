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
- `BR-009` Compatibility policy: restore MUST support backup format major versions `current` and `previous`; it MUST reject older or newer unsupported majors.
- `BR-010` Migration policy: restore MUST run component migrations (history payload, dictionary payload, and settings payload) to current import schemas before staging import.
- `BR-011` Restore mode: v1 restore MUST be replace-only.
- `BR-012` Atomic safety: restore MUST stage all writes and only swap into active app data after staging validates.
- `BR-013` Rollback: if any step fails before final success, active data MUST remain unchanged.
- `BR-014` Audio restore semantics: text history restore MUST succeed even when archive has no recordings; missing audio playback must fail gracefully with user-facing message.
- `BR-015` Result reporting: export and restore MUST return machine-readable summary reports (counts, warnings, failures) plus concise user-facing summary text; detailed diagnostics are optional behind explicit UI action.
- `BR-016` Concurrency guard: backup/restore operations MUST acquire an app-level mutex so only one top-level operation runs at a time. Internal sub-phases (including mandatory pre-restore safety backup) MUST reuse the same operation context and MUST NOT reacquire the top-level mutex.
- `BR-017` Encryption scope: v1 backups MUST be unencrypted by app logic; UI MUST disclose this before export confirmation.
- `BR-018` Archive extraction safety: restore MUST validate archive entry metadata before extraction and reject archives containing absolute paths, parent-directory traversal (`..`), drive-prefixed paths, or symlink/hardlink entries.
- `BR-019` Storage capacity precheck: export and restore MUST estimate required free disk space for temp workspace **and destination** and fail fast with actionable error if insufficient.
- `BR-020` Cancellation safety: canceling backup/restore MUST clean temporary files and MUST NOT partially modify active restore-managed data. UI MUST expose phase-aware quit/cancel options and clearly indicate non-cancellable phases.
- `BR-021` Crash/interruption recovery: restore MUST persist an in-progress marker and reconcile on next app start to either complete rollback or clear safe staging leftovers.
- `BR-022` Recoverable import issue handling: restore MUST preserve valid data by deterministically rekeying duplicate history IDs, suffix-renaming duplicate recording filenames (e.g., `codictate-1234.wav` → `codictate-1234_1.wav`), and skipping only unrecoverable individual rows/files with warning details, while preserving row-to-audio linkage where possible.
- `BR-023` Path scope restriction: export and restore file operations MUST be restricted to user-selected archive paths and app-managed data directories.
- `BR-024` Pre-restore safety backup (full-first): before any restore overwrites active data, the system MUST automatically attempt a full safety snapshot (`full` scope, including recordings) to a `.codictatebackup` file in the app data directory. On low-space failure, the system MUST prune old automatic safety backups per policy and retry full once before fallback UI choices. If full safety backup still cannot complete, UI MUST require explicit user choice to retry, continue with lightweight safety backup, or cancel. Silent downgrade is forbidden. After restore, UI MUST let users explicitly save or discard the automatically-created safety backup.
- `BR-025` User stats restore: restored `user_stats` MUST be recomputed from the imported history rows (recalculating `total_words`, `total_duration_ms`, `total_transcriptions`, `transcription_dates`, `total_filler_words_removed`) rather than blindly importing the backup's stats snapshot, so that stats are always consistent with the actual restored history.
- `BR-026` Selective settings backup: backup MUST export user-configurable preferences from `AppSettings` (shortcuts, language, overlay position, audio feedback, paste method, etc.) but MUST exclude sensitive fields (`post_process_api_keys`) and device-specific fields (`selected_microphone`, `clamshell_microphone`, `selected_output_device`, `selected_model`). The exported settings payload MUST include a `settings_payload_version`.
- `BR-027` Forward-compatible settings restore: on restore, settings MUST be merged selectively into the current `AppSettings`. Fields present in the backup but absent in the current schema are ignored. Fields present in the current schema but absent in the backup retain their current (or default) values. This ensures backups from older app versions restore cleanly on newer versions.
- `BR-028` Progress and ETA reporting: export and restore operations MUST emit incremental progress events to the frontend with `{ phase, current, total, estimated_seconds_remaining }` at each major step (snapshot, file copy, staging, swap). The frontend MUST display a progress bar with estimated time remaining.
- `BR-029` Estimated size preview: before export begins, the system MUST calculate and display the estimated archive size based on current history row count, recording file sizes, and dictionary size. The user MUST confirm after seeing the estimate and destination-space check result. Export save flow MUST prefill a Codictate filename, auto-append `.codictatebackup` when omitted, and require overwrite confirmation.
- `BR-030` Cross-platform universal backup: archive entries MUST use forward-slash (`/`) path separators regardless of the creating platform. On restore, filenames MUST be normalized to NFC Unicode form. Manifest MUST record the source `platform` field. Invalid or unsafe names in required payload paths are blocking failures; invalid recording filenames are recoverable and MUST be deterministically sanitized or skipped with warning.
- `BR-031` Structured logging: every major export/restore milestone MUST emit structured `tracing` logs at `info!` level. All recoverable warnings MUST emit `warn!` logs. All failures MUST emit `error!` logs with context. Frontend backup/restore actions MUST log via `logInfo`/`logError` to the unified log target `fe-backup`.
- `BR-032` Internationalization: all user-facing backup/restore strings (UI labels, warnings, confirmations, progress messages, error messages) MUST use i18next translation keys. Keys MUST be added to `src/i18n/locales/en/translation.json` under a `settings.backup` namespace.
- `BR-033` Partial-restore consent: if preflight detects recoverable issues and no blocking issues, UI MUST present a concise issue summary with optional detailed drill-in and require explicit user choice to `continue_partial_restore` or `cancel`.
- `BR-034` Restore quiesce mode: before apply-restore starts, the app MUST enter restore lock mode that blocks new transcription/history writes until restore completes, fails, or rolls back. Blocked actions MUST show clear user-facing status and a path to open restore progress.
- `BR-035` Archive resource bounds (two-tier): restore MUST enforce:
  - hard security bounds (non-overridable) for unsafe/decompression-abuse risk (for example compression-ratio explosion and absolute upper parser bounds)
  - soft operational thresholds (generous defaults) for large-but-possibly-valid archives; soft threshold breaches MUST warn and require explicit user choice to continue once, update limits, or cancel
  All JSON/JSONL parsing MUST remain bounded streaming parsing regardless of override choice.
- `BR-036` Integrity scope disclosure: v1 checksum verification MUST be documented and surfaced as corruption/tamper detection only; it MUST NOT claim trusted-origin authenticity because v1 does not include signatures or MAC.
- `BR-037` Log redaction and privacy: backup/restore logs MUST NOT include transcript text, prompt text, API keys, or raw payload bodies. Logs MUST use metadata-only fields (counts, durations, non-sensitive identifiers).

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
Restore preflight/import uses bounded limits to prevent resource exhaustion with a soft/hard model.

Hard security bounds (non-overridable):
- `hard_max_archive_entries`: 2_000_000
- `hard_max_entry_uncompressed_bytes`: 34_359_738_368 (32 GiB)
- `hard_max_total_uncompressed_bytes`: 214_748_364_800 (200 GiB)
- `hard_max_compression_ratio`: 200x for any single entry
- `hard_max_history_jsonl_line_bytes`: 16_777_216 (16 MiB)
- `hard_max_history_rows`: 50_000_000

Soft operational thresholds (user-overridable and user-configurable):
- `soft_max_archive_entries`: 250_000
- `soft_max_entry_uncompressed_bytes`: 8_589_934_592 (8 GiB)
- `soft_max_total_uncompressed_bytes`: 53_687_091_200 (50 GiB)
- `soft_max_history_jsonl_line_bytes`: 4_194_304 (4 MiB)
- `soft_max_history_rows`: 20_000_000

Behavior:
- hard-bound breach: blocking failure (`BR-035`)
- soft-threshold breach: warning with explicit choices:
  - continue once for this restore
  - update soft limit settings and continue
  - cancel
- bounded parsing is always enforced even when user overrides soft thresholds

## Export Algorithm (V1)
1. Acquire backup mutex (`BR-016`).
2. Calculate estimated archive size, prefill Codictate default filename, and display export confirmation (`BR-029`).
3. Read app state snapshot:
   - open SQLite read transaction for history export (`BR-005`)
   - read dictionary from settings
   - read selective settings (excluding sensitive/device-specific fields) (`BR-026`)
   - determine referenced recording filenames from exported rows
4. Estimate and verify disk capacity for temp workspace and output target (`BR-019`).
5. Build payload in temp workspace. Emit progress events (`BR-028`).
6. Write logical payload files:
   - `history/history.jsonl`
   - `history/user_stats.json`
   - `dictionary/dictionary.json`
   - `settings/settings.json`
7. If scope is `full`, copy referenced audio files that exist into `recordings/` and track missing files as warnings (`BR-007`). Emit per-file progress events.
8. Normalize all archive entry paths to forward-slash separators and NFC Unicode (`BR-030`).
9. Write `manifest.json` with format/component versions, platform, and counts (`BR-003`).
10. Compute and write `checksums.sha256` (`BR-004`).
11. Package temp workspace into `.codictatebackup` ZIP (`BR-001`).
12. Write destination atomically (temp output then rename).
13. Support cancellation checkpoints and cleanup if canceled (`BR-020`).
14. Log export completion with counts and duration (`BR-031`).
15. Return export report with counts/warnings (`BR-015`).

## Restore Algorithm (V1)
1. Acquire backup mutex (`BR-016`).
2. Enter restore quiesce mode:
   - block new recording/transcription/history-write operations (`BR-034`)
   - verify no in-flight write remains (or abort with busy/retry message)
3. Estimate and verify disk capacity for unpack + staging + safety backup + rollback workspace (`BR-019`).
4. Preflight archive-entry safety scan (without extracting file contents):
   - reject blocking unsafe entries (`BR-018`)
   - enforce hard security bounds (`BR-035`): absolute parser ceilings and compression-ratio guard
   - evaluate soft operational thresholds (`BR-035`) and prepare warning/override choices when exceeded
   - classify recoverable name issues in optional recordings (`BR-008`, `BR-030`)
5. Unpack archive to temp workspace only after safety scan passes. Emit progress events (`BR-028`).
6. Normalize extracted filenames to NFC Unicode and validate cross-platform filename safety (`BR-030`).
7. Persist restore-in-progress marker for crash recovery (`BR-021`).
8. Run preflight validation (`BR-008`):
   - required files present (`BR-002`)
   - manifest parse + version parse (`BR-003`)
   - checksum verification (`BR-004`)
   - include integrity-scope note in preflight result (`BR-036`): checksum validates corruption/tamper evidence only, not trusted origin
   - compatibility gate (`BR-009`)
   - classify findings into `blocking` vs `recoverable`
9. Show preflight summary with current-vs-backup counts, backup identity metadata, and concise issue summary (optional details behind explicit action):
   - if any `blocking` issue exists, abort restore
   - if soft thresholds are exceeded, require explicit user choice: continue once, update soft limits and continue, or cancel (`BR-035`)
   - if only `recoverable` issues exist, require explicit user choice to continue partial restore or cancel (`BR-033`)
10. Auto-create pre-restore safety backup (`BR-024`):
    - first attempt full safety backup (includes recordings)
    - if full safety backup fails from low space, prune old automatic safety backups and retry full once
    - if full safety backup still fails, require explicit user choice: retry full, continue with lightweight safety backup, or cancel
    - never silently downgrade from full to lightweight
    - log safety backup path and scope (`BR-031`)
11. Run component migration pipeline to current payload schemas (`BR-010`).
12. Build staging target under temp dir:
    - initialize fresh staging `history.db` via current migrations
    - import history rows with row-level recovery (`BR-022`)
      - duplicate IDs are deterministically rekeyed (IDs in backup are non-authoritative)
      - malformed rows and oversize lines are skipped and recorded as warnings using bounded JSONL parsing (`BR-035`)
    - recompute `user_stats` from imported history rows (`BR-025`)
    - import dictionary into staged settings payload
    - merge settings selectively into staged settings, applying forward-compatible defaults (`BR-027`)
    - copy recordings with recoverable handling (`BR-022`, `BR-030`)
      - duplicate filenames are suffix-renamed
      - invalid filenames are sanitized when safe; otherwise skipped with warnings
      - row-to-audio references are remapped to resolved filenames
13. Validate staging:
    - SQLite `PRAGMA integrity_check`
    - consistency check: every imported row either has a present audio file or is marked audio-unavailable at runtime (`BR-014`)
14. Swap data replace-only (`BR-011`, `BR-012`):
    - move active data to rollback temp
    - move staging data into active app data paths
15. If any step fails before completion, rollback active data (`BR-013`). Log rollback (`BR-031`).
16. Support cancellation checkpoints and cleanup (`BR-020`).
17. Clear restore marker after success or completed rollback (`BR-021`).
18. Exit restore quiesce mode (`BR-034`).
19. Log restore completion with restored + skipped counts and duration (`BR-031`).
20. Emit refresh events and return restore report (`BR-015`).
21. After restore completion, present `Save safety backup` vs `Discard safety backup` disposition for automatically-created safety backup (`BR-024`).

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
warn!(soft_limit_name = %limit_name, observed = observed, configured = configured, "Soft operational threshold exceeded");
info!(decision = %decision, "User decision for soft-threshold warning");
error!(file = %name, "Checksum mismatch detected during preflight");
info!(safety_backup_path = %path, safety_scope = %scope, "Pre-restore safety backup created");
info!(migration_from = from, migration_to = to, "Running payload migration");
info!(history_restored = count, recordings_restored = rec_count, skipped_rows = skipped_rows, skipped_recordings = skipped_files, duration_ms = elapsed, "Restore completed successfully");
error!(step = %step, err = %e, "Restore failed, initiating rollback");
info!("Rollback completed, active data unchanged");

// Crash recovery
info!(marker = %path, "Restore-in-progress marker found at startup");
info!(action = %action, "Startup reconciliation completed");

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

## Compatibility and Lifecycle
- Backup format semver starts at `1.0.0`.
- Supported restore range is fixed to `current_major` and `current_major - 1` (`BR-009`).
- Unsupported older backups must show migration guidance: restore in a compatible older app and re-export.
- Unsupported newer backups must show forward-compatibility guidance: update app first.
- If support for a previous major is removed in a future release, deprecation notice must be shipped at least one stable release earlier.

## Goals / Non-Goals
- Goals:
  - Provide reliable user-owned local backup and restore.
  - Preserve history text, dictionary, and user settings in all backup scopes.
  - Preserve audio when included and degrade gracefully when absent.
  - Keep restore safe via automatic safety backup, staging, and rollback.
  - Provide progress visibility and ETA during long operations.
  - Ensure backups are universally portable across macOS, Windows, and Linux.
- Non-Goals:
  - Cloud backup/sync.
  - Scheduled backups.
  - Merge-mode restore.
  - App-level encryption or password-protected archive format in v1.

## Risks / Trade-offs
- Large full backups can be slow and large.
  - Mitigation: lightweight scope, explicit scope descriptions, estimated size preview, and progress bar with ETA.

- Migration code increases long-term maintenance.
  - Mitigation: strict compatibility window and migration fixture tests.

- Lightweight backups produce text-only history for audio.
  - Mitigation: explicit UI warning and graceful missing-audio playback behavior.

- Selective settings backup may confuse users about what is/isn't restored.
  - Mitigation: restore confirmation UI shows what will be restored and what won't. Device-specific settings (microphone, model) are clearly marked as excluded.

- Pre-restore full safety backup adds disk usage and time.
  - Mitigation: full-first with prune-and-retry on low space, explicit fallback choices (retry full, lightweight fallback, cancel), and post-restore `Save`/`Discard` choice for the automatic safety backup.

- Partial restore can skip problematic rows/files.
  - Mitigation: require explicit continue/cancel choice, show concise preflight issue summary by default, and provide optional detailed skipped-item diagnostics.

- Background operations can feel hidden when settings is closed.
  - Mitigation: operation reattach state, tray/desktop notifications, and explicit desktop fallback when tray icon is disabled.

- Large valid backups may exceed default soft thresholds.
  - Mitigation: generous defaults, explicit warning UI, continue-once override, and user-configurable soft thresholds.

- Checksums without signatures can’t prove trusted origin.
  - Mitigation: explicit integrity-scope disclosure in UI/docs; reserve signed authenticity for a future version.

## Edge Cases Explicitly Covered
- Unsafe archive payload paths and symlink/hardlink extraction attempts.
- Insufficient disk space for export temp output, destination, or restore staging.
- User cancellation at safe checkpoints.
- App/process interruption mid-restore with startup reconciliation.
- Duplicate imported IDs (deterministic rekey preserving valid rows) and duplicate recording filenames (resolved by suffix rename).
- Recoverable malformed rows and invalid optional recording filenames (skipped with warnings during partial restore).
- Soft-threshold limit overruns with user override/update flow.
- Archive resource-exhaustion attempts (oversized entry count, single-entry size, total size, compression ratio, oversized JSONL lines).
- Partial/missing recording files while preserving text history.
- Cross-platform filename characters invalid on target OS.
- Unicode normalization differences between macOS (NFD) and other platforms (NFC).
- Settings schema evolution: new fields added after backup was created get defaults on restore.
- Settings schema evolution: removed fields in backup are silently ignored on restore.
- Empty history export (0 entries produces valid archive with empty JSONL).
- Corrupted SQLite at export time (export fails with actionable error).
- FAT32/exFAT filesystem size limits at export destination.
- Save dialog filename/extension mistakes (default Codictate filename + auto-append + overwrite confirmation).
- Tray-disabled background operations (desktop notification fallback keeps status visible).
- High warning volume during preflight/restore (concise default summaries with optional detailed drill-in).
- Post-restore automatic safety backup disposition (`Save` vs `Discard`) and retention interactions.
