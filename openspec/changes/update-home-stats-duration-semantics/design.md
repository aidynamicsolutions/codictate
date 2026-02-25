## Context
Home stats need to reflect reality for both short and long transcriptions. Existing behavior used a fixed `-900ms` adjustment to counter VAD overhead, but this conflated speech rate and recording elapsed time and depended on padded ASR input length.

## Goals / Non-Goals
- Goals:
  - Track true recording elapsed duration separately from speech-retained duration.
  - Keep public Home stats API shape stable.
  - Provide crash-safe, idempotent migration.
  - Preserve lifetime totals even when historical rows were pruned.
- Non-Goals:
  - Add user-facing duration-mode toggles.
  - Rebuild all historical durations with perfect fidelity (not possible for missing rows).

## Decisions
- Decision: Use hybrid metrics.
  - `WPM = total_words / total_speech_duration_minutes`
  - `TimeSaved = (total_words / 40) - total_recording_duration_minutes`
- Decision: Define speech duration as VAD-retained unpadded sample duration from recorder output.
- Decision: Keep short-clip ASR padding for transcription quality, but decouple stats from padded sample length.
- Decision: Introduce marker `duration_stats_semantics_version` to gate new semantics and prevent partial migration reads.
- Decision: Create backup artifacts before mutation:
  - filesystem JSON snapshot under `stats-backups/`
  - append-only row in `user_stats_migration_backup`

## Migration Plan
1. Apply schema migrations for new columns/table.
2. If `duration_stats_semantics_version >= 1`, no-op.
3. Snapshot current `user_stats` and write filesystem backup.
4. Start DB transaction.
5. Insert snapshot row into `user_stats_migration_backup`.
6. Compute:
   - `live_recording_sum_ms` from `transcription_history.duration_ms`
   - `live_speech_sum_ms` from `speech_duration_ms` fallback to `duration_ms`
   - `live_legacy_heuristic_sum_ms` using legacy formula per row
7. Compute residual:
   - `legacy_residual_ms = max(old_total_duration_ms - live_legacy_heuristic_sum_ms, 0)`
8. Write:
   - `total_duration_ms = live_recording_sum_ms + legacy_residual_ms`
   - `total_speech_duration_ms = live_speech_sum_ms + legacy_residual_ms`
   - `duration_stats_semantics_version = 1`
9. Commit transaction.

## Risks / Trade-offs
- Existing installs may observe a one-time metric shift after migration.
- Historical precision is best-effort for deleted/missing rows.
- Backup write failure blocks migration, intentionally preferring safety over silent partial conversion.

## Recovery
If migration/backfill fails, DB transaction rolls back. Operators can restore from:
- latest filesystem snapshot in `stats-backups/`
- latest row in `user_stats_migration_backup`
