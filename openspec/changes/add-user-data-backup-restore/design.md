## Context
Current user data is split across:
- `history.db` (SQLite history + stats)
- `recordings/` (WAV files referenced by history rows)
- `settings_store.json` (dictionary is inside app settings payload)

For local-only apps, users still need portable ownership of their data across uninstall/reinstall and machine changes. Directly copying live SQLite files is fragile, especially with journaling/WAL side files, so backup must be a controlled app workflow.

## V1 Implementation Rules
All implementation tasks MUST satisfy these rules.

- `BR-001` Archive type: backup output MUST be one `.codictatebackup` file using ZIP container format.
- `BR-002` Archive structure: archive MUST include `manifest.json`, `checksums.sha256`, `history/history.jsonl`, `history/user_stats.json`, `dictionary/dictionary.json`; `recordings/` is optional.
- `BR-003` Manifest contract: `manifest.json` MUST include `backup_format_version`, `created_at`, `created_with_app_version`, `components`, `includes_recordings`, and per-component counts.
- `BR-004` Checksums: `checksums.sha256` MUST include SHA-256 for every payload file present in archive except the checksum file itself.
- `BR-005` Export format: history MUST be exported as logical records (JSONL) rather than raw SQLite file copy.
- `BR-006` Backup scopes: v1 MUST support exactly two export scopes: `full` (with recordings) and `lightweight` (without recordings).
- `BR-007` Missing audio at export: if full backup is requested but some referenced audio files are missing, export MUST complete, record warning metadata, and exclude missing files.
- `BR-008` Preflight-first restore: restore MUST run non-destructive preflight before any live data write.
- `BR-009` Compatibility policy: restore MUST support backup format major versions `current` and `previous`; it MUST reject older or newer unsupported majors.
- `BR-010` Migration policy: restore MUST run component migrations (history payload and dictionary payload) to current import schemas before staging import.
- `BR-011` Restore mode: v1 restore MUST be replace-only.
- `BR-012` Atomic safety: restore MUST stage all writes and only swap into active app data after staging validates.
- `BR-013` Rollback: if any step fails before final success, active data MUST remain unchanged.
- `BR-014` Audio restore semantics: text history restore MUST succeed even when archive has no recordings; missing audio playback must fail gracefully with user-facing message.
- `BR-015` Result reporting: export and restore MUST return machine-readable summary reports (counts, warnings, failures).
- `BR-016` Concurrency guard: backup/restore operations MUST acquire an app-level mutex so only one operation runs at a time.
- `BR-017` Encryption scope: v1 backups MUST be unencrypted by app logic; UI MUST disclose this before export confirmation.
- `BR-018` Archive extraction safety: restore MUST reject archives containing absolute paths, parent-directory traversal (`..`), drive-prefixed paths, or symlink/hardlink entries.
- `BR-019` Storage capacity precheck: export and restore MUST estimate required free disk space for temp workspace and fail fast with actionable error if insufficient.
- `BR-020` Cancellation safety: canceling backup/restore MUST clean temporary files and MUST NOT partially modify active restore-managed data.
- `BR-021` Crash/interruption recovery: restore MUST persist an in-progress marker and reconcile on next app start to either complete rollback or clear safe staging leftovers.
- `BR-022` Import conflict resolution: restore MUST resolve duplicate history IDs and duplicate recording filenames deterministically while preserving row-to-audio linkage.
- `BR-023` Path scope restriction: export and restore file operations MUST be restricted to user-selected archive paths and app-managed data directories.

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
  "includes_recordings": true,
  "components": {
    "history_payload_version": 1,
    "dictionary_payload_version": 1
  },
  "counts": {
    "history_entries": 312,
    "recording_files": 288,
    "dictionary_entries": 47
  },
  "warnings": {
    "missing_recordings": [
      "codictate-1734959931.wav"
    ]
  }
}
```

### Checksum File
`checksums.sha256` lines follow:
`<hex_sha256><two_spaces><relative_path>`

Example:
```text
0a...ff  manifest.json
b1...3d  history/history.jsonl
...
```

## Export Algorithm (V1)
1. Acquire backup mutex (`BR-016`).
2. Read app state snapshot:
- open SQLite read transaction for history export (`BR-005`)
- read dictionary from settings
- determine referenced recording filenames from exported rows
3. Estimate and verify disk capacity for temp workspace and output target (`BR-019`).
4. Build payload in temp workspace.
5. Write logical payload files:
- `history/history.jsonl`
- `history/user_stats.json`
- `dictionary/dictionary.json`
6. If scope is `full`, copy referenced audio files that exist into `recordings/` and track missing files as warnings (`BR-007`).
7. Write `manifest.json` with format/component versions and counts (`BR-003`).
8. Compute and write `checksums.sha256` (`BR-004`).
9. Package temp workspace into `.codictatebackup` ZIP (`BR-001`).
10. Write destination atomically (temp output then rename).
11. Support cancellation checkpoints and cleanup if canceled (`BR-020`).
12. Return export report with counts/warnings (`BR-015`).

## Restore Algorithm (V1)
1. Acquire backup mutex (`BR-016`).
2. Estimate and verify disk capacity for unpack + staging + rollback workspace (`BR-019`).
3. Unpack archive to temp workspace.
4. Validate extracted entry safety rules (no traversal/absolute/symlink/hardlink entries) (`BR-018`).
5. Persist restore-in-progress marker for crash recovery (`BR-021`).
6. Run preflight validation (`BR-008`):
- required files present (`BR-002`)
- manifest parse + version parse (`BR-003`)
- checksum verification (`BR-004`)
- compatibility gate (`BR-009`)
7. Run component migration pipeline to current payload schemas (`BR-010`).
8. Build staging target under temp dir:
- initialize fresh staging `history.db` via current migrations
- import history rows + user stats
- import dictionary into staged settings payload
- copy recordings if present
9. Resolve import conflicts deterministically (duplicate IDs and duplicate filenames) (`BR-022`).
10. Validate staging:
- SQLite `PRAGMA integrity_check`
- consistency check: every imported row either has a present audio file or is marked audio-unavailable at runtime (`BR-014`)
11. Swap data replace-only (`BR-011`, `BR-012`):
- move active data to rollback temp
- move staging data into active app data paths
12. If any step fails before completion, rollback active data (`BR-013`).
13. Support cancellation checkpoints and cleanup (`BR-020`).
14. Clear restore marker after success or completed rollback (`BR-021`).
15. Emit refresh events and return restore report (`BR-015`).

## Compatibility and Lifecycle
- Backup format semver starts at `1.0.0`.
- Supported restore range is fixed to `current_major` and `current_major - 1` (`BR-009`).
- Unsupported older backups must show migration guidance: restore in a compatible older app and re-export.
- Unsupported newer backups must show forward-compatibility guidance: update app first.
- If support for a previous major is removed in a future release, deprecation notice must be shipped at least one stable release earlier.

## Goals / Non-Goals
- Goals:
- Provide reliable user-owned local backup and restore.
- Preserve history text and dictionary in all backup scopes.
- Preserve audio when included and degrade gracefully when absent.
- Keep restore safe via staging and rollback.
- Non-Goals:
- Cloud backup/sync.
- Scheduled backups.
- Merge-mode restore.
- App-level encryption or password-protected archive format in v1.

## Risks / Trade-offs
- Large full backups can be slow and large.
- Mitigation: lightweight scope and explicit scope descriptions.

- Migration code increases long-term maintenance.
- Mitigation: strict compatibility window and migration fixture tests.

- Lightweight backups produce text-only history for audio.
- Mitigation: explicit UI warning and graceful missing-audio playback behavior.

## Edge Cases Explicitly Covered
- Unsafe archive payload paths and symlink/hardlink extraction attempts.
- Insufficient disk space for export temp output or restore staging.
- User cancellation at safe checkpoints.
- App/process interruption mid-restore with startup reconciliation.
- Duplicate imported IDs and duplicate recording filenames.
- Partial/missing recording files while preserving text history.
