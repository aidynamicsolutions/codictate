## 1. Implementation
- [x] 1.1 Define restore-time streak carry-over eligibility from backup stats and backup creation time.
- [x] 1.2 Restore the current streak in stats so the restored home stats reflect the eligible carried streak immediately after restore.
- [x] 1.3 Implement persisted or derivable restore carry-over state so home stats can preserve an eligible streak on restore day without inventing missed history dates.
- [x] 1.4 Update streak progression logic so the first post-restore transcription continues the preserved streak and the first missed post-restore local day expires it.
- [x] 1.5 Add backend tests for eligible restore carry-over, stale-backup no-carry-over, restore-day increment, and post-restore expiry behavior.
