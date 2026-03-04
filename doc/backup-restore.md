# Backup & Restore

Codictate supports local backup and restore to protect user data across reinstalls and machine moves.

## What Is Included (v1)

A backup is a single `.codictatebackup` ZIP archive with this layout:

```text
backup.codictatebackup
├── manifest.json
├── checksums.sha256
├── history/history.jsonl
├── history/user_stats.json      # optional in legacy archives; canonical in current archives
├── dictionary/dictionary.json
├── user/user_store.json
└── recordings/               # only for complete backups
```

Included data:
- History text (`history/history.jsonl`)
- Canonical productivity stats snapshot (`history/user_stats.json`) for exact WPM/word/time-saved restore when available
- Dictionary entries (`dictionary/dictionary.json`)
- User onboarding/profile/growth store (`user/user_store.json`)
- Optional recordings (`recordings/`) for complete backups

Not included in v1:
- App settings/preferences (`settings_store.json`)
- Any app-level encryption metadata

## Backup Scopes

- Complete backup: history + dictionary + user-store + recordings (when present)
- Smaller backup: history + dictionary + user-store (no recordings)

## Availability (v1)

- Backup and restore are currently enabled on macOS only.
- Non-macOS builds keep this feature disabled until platform-specific validation is completed.

## User Flow

### Create Backup

1. Open `Settings > General > Backup & Restore`.
2. Click `Create Backup` to open the create-backup modal.
3. Configure scope with `Include recordings`:
   - ON: complete backup (history + dictionary + user-store + recordings)
   - OFF: smaller backup (history + dictionary + user-store, no recordings)
4. Review estimated size in the modal (and approximate savings when recordings are excluded).
5. Confirm `Create Backup`, then pick destination path in the save dialog.
6. Wait for progress to finish.

Notes:
- Backup and restore run in maintenance mode.
- Core recording/transcription/write actions are temporarily blocked during operation.
- Starting a new transcription (standard shortcut, hands-free toggle, Fn key path, coordinator path) is rejected while backup/restore is active.
- Repeated blocked start attempts surface a throttled info notice (5-second cooldown) to avoid notification spam.
- You can request cancellation; long-running loops stop cooperatively at chunk boundaries, including export recordings copy, archive packaging, archive extraction, and restore recordings import (short phases still complete at their boundary).

### Concurrency Model

Backup/restore now uses two coordinated runtime gates to avoid write races:

- Operation gate (`operation_in_progress` + maintenance mode): serializes backup/restore operations and blocks new managed writes for operation lifetime.
- Write gate (`with_write_permit`): wraps restore-managed mutations (history, dictionary, user-profile updates) in a single critical section.

Behavioral guarantees:
- Operation start waits for already-entered write-critical sections to drain before entering maintenance mode.
- Once an operation is active, new managed writes fail fast with a maintenance-mode error.
- Command-triggered managed writes (for example profile/history/dictionary mutations) return explicit errors when blocked by maintenance mode.
- User profile read paths are side-effect free; missing/malformed profile payloads return defaults without writing during maintenance mode.
- Background best-effort persistence paths continue to skip writes during maintenance mode instead of forcing hard failures.

### Restore Backup

1. Click `Restore Backup`.
2. Select a `.codictatebackup` file in the open dialog.
3. Review preflight summary:
   - created time, app version, platform, includes recordings, counts
   - compatibility note (macOS v1 guaranteed; cross-platform best-effort)
   - compatibility note contract: machine-readable `compatibility_note_code` for i18n mapping, with `compatibility_note` text fallback for compatibility
   - blocking issues and recoverable warnings (plain-language categories)
4. Confirm destructive replace-only restore.

Restore behavior:
- Replace-only: managed data is replaced, not merged.
- Staged apply with rollback snapshot.
- Marker-based crash recovery (`in_progress` / `committed`).
- Startup reconciliation is fail-closed for invalid rollback metadata:
  - if `in_progress` marker snapshot path is invalid, stale marker is removed
  - if snapshot layout metadata is missing/invalid/incomplete, active data is left untouched and marker is kept for manual recovery/retry
- Stats import source:
  - Preferred: canonical stats payload (`history/user_stats.json`) when present and valid
  - Fallback (legacy/broken payload): recompute from history rows using runtime-consistent formulas
  - Fallback path emits recoverable warnings in preflight/apply UX
- Current install settings stay unchanged.

## One-Time Manual Repair (Known March 3, 2026 Regression)

If local stats are stuck at the known bad signature after restore:

- `total_words=46208`
- `total_duration_ms=22697871`
- `total_speech_duration_ms=3202691`
- `duration_stats_semantics_version=1`

apply this guarded patch once to restore the expected values:

- `total_words=43604`
- `total_duration_ms=22720677`
- `total_speech_duration_ms=22618064`
- `duration_stats_semantics_version=1`

Runbook SQL (`~/Library/Application Support/com.pais.codictate/history.db`):

```sql
-- 1) Inspect current signature
SELECT
  total_words,
  total_duration_ms,
  total_speech_duration_ms,
  duration_stats_semantics_version
FROM user_stats
WHERE id = 1;

-- 2) Guarded one-time repair
UPDATE user_stats
SET
  total_words = 43604,
  total_duration_ms = 22720677,
  total_speech_duration_ms = 22618064,
  duration_stats_semantics_version = 1
WHERE
  id = 1
  AND total_words = 46208
  AND total_duration_ms = 22697871
  AND total_speech_duration_ms = 3202691
  AND duration_stats_semantics_version = 1;

-- 3) Verify repaired WPM is approximately 115.67
SELECT
  total_words,
  total_duration_ms,
  total_speech_duration_ms,
  ROUND(total_words / (total_speech_duration_ms / 60000.0), 2) AS wpm
FROM user_stats
WHERE id = 1;
```

Post-check:
- Relaunch the app and verify Home stats show the expected baseline.

Optional observability hook:
- Start the app once with `HANDY_MANUAL_STATS_REPAIR_20260303=1` to emit structured log `restore_stats_manual_repair` with `outcome=applied|skipped` and reason fields.

## Undo Last Restore

After a successful restore, Codictate attempts to keep one undo checkpoint (best effort):

- Source: pre-swap snapshot from the restore
- Retention: 7 days
- Cardinality: one checkpoint only (new restore replaces old)
- UX: `Undo Last Restore` is available only while checkpoint is valid and checkpoint creation succeeds
- Progress UX: once started, Undo shows the same in-card progress surface style as backup/restore (status text, progress bar, and loading spinner on the undo action)
- Undo phases: prepare -> stage checkpoint -> snapshot current data -> swap -> cleanup -> finalize

If checkpoint creation fails (for example low disk space), restore still completes and reports a warning that undo is unavailable for that run.
If checkpoint data is expired, missing, incomplete, or corrupted, undo is unavailable and active data is left unchanged.

Operational cleanup:
- Snapshots created for failed/rolled-back restore attempts are deleted immediately after rollback succeeds.
- Stale backup/restore runtime artifacts are pruned automatically after the retention window.

## Integrity & Safety

- SHA-256 checksums are validated before restore apply.
- Restore preflight checks current local history integrity; if local history is corrupted, restore is blocked and users are instructed to repair local data first.
- Backup export validates history recording file references; unsafe row filenames are rejected so new backups remain restorable.
- Backup export enforces the same per-payload file-size limit as restore preflight, so export does not produce backups that this app version would later reject.
- Archive safety checks reject:
  - path traversal
  - absolute paths
  - control characters in archive path segments
  - symlink entries
  - duplicate normalized paths
  - case-colliding paths
- Managed recording/history filenames that include control characters are rejected during export/import validation to keep `checksums.sha256` line parsing unambiguous.
- Local snapshot/copy safety rejects symlinked source roots (for example, a symlinked `recordings/` directory) during restore/undo staging.
- Preflight enforces hard bounds:
  - max archive size: 10 GiB
  - max payload file: 512 MiB
  - max history rows: 2,000,000
  - max history JSONL line: 8 MiB
  - max archive entries: 2,100,000
  - max total uncompressed bytes: 20 GiB
- Restore extraction re-enforces payload and total uncompressed byte limits while streaming each ZIP entry, so limits are checked at read-time and not only from archive metadata.
- Free-space gate blocks restore when staging/snapshot/swap requirements are not met.

Important:
- Checksums detect corruption evidence but do not provide trusted-origin authenticity in v1.
- Backups are not encrypted in v1.

## Compatibility Contract (v1)

- Backups with format major `1` created on macOS v1 are restorable on later macOS v1 versions.
- Restore parsers tolerate additive unknown JSON fields and default missing optional fields.
- Cross-platform restore is best-effort in v1 (not guaranteed).
