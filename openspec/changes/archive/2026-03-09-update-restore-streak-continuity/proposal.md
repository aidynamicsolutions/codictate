# Change: Preserve active streak continuity across restore

## Why
Restoring a valid backup can make an active daily streak appear broken solely because wall-clock time passed between backup creation and restore. The current behavior is technically consistent with historical `transcription_dates`, but it is poor UX because restore feels like it punishes the user for recovering their own data.

## What Changes
- Preserve an active streak from backup as restore carry-over state when the streak was still active at backup creation time.
- Continue that streak from the restore date forward without synthesizing missing historical transcription days.
- Expire the preserved streak only after the user misses a post-restore local day.
- Add tests covering eligible carry-over, first post-restore transcription, and stale backups that should not resurrect a broken streak.

## Impact
- Affected specs: `data-backup-restore`, `transcription-history`
- Affected code: `src-tauri/src/backup_restore/restore.rs`, `src-tauri/src/managers/history.rs`, backup/restore tests, home stats tests
