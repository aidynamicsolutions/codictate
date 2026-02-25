## 1. Backend Duration Model
- [x] 1.1 Return structured stop-recording payload with `samples_for_transcription`, `recording_duration_ms`, and `speech_duration_ms`.
- [x] 1.2 Stop deriving persisted duration from padded sample length in action flow.
- [x] 1.3 Persist `duration_ms` as recording elapsed duration and `speech_duration_ms` as VAD-retained unpadded duration.

## 2. Stats Aggregation and Rollback
- [x] 2.1 Update stats writes to accumulate both `total_duration_ms` and `total_speech_duration_ms`.
- [x] 2.2 Update undo contribution and rollback to reverse both duration counters.
- [x] 2.3 Update `get_home_stats()` to compute WPM from speech duration and Time Saved from recording duration.

## 3. Migration and Safety
- [x] 3.1 Add schema columns: `transcription_history.speech_duration_ms`, `user_stats.total_speech_duration_ms`, `user_stats.duration_stats_semantics_version`.
- [x] 3.2 Add append-only `user_stats_migration_backup` table.
- [x] 3.3 Implement pre-mutation stats backup snapshot to filesystem and DB.
- [x] 3.4 Implement atomic, marker-gated backfill preserving lifetime totals via residual strategy.

## 4. Docs and UX Copy
- [x] 4.1 Update tooltip copy for WPM/Time Saved semantics.
- [x] 4.2 Update `doc/stats.md` with dual-duration formulas, migration note, and backup/recovery notes.

## 5. Validation
- [x] 5.1 Run targeted Rust tests for updated stats and undo behavior.
- [x] 5.2 Run `openspec validate update-home-stats-duration-semantics --strict`.
