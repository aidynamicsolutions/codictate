## Context
Current user data is stored across:
- `history.db` (history rows and stats)
- `recordings/` (WAV files referenced by history rows)
- `user_dictionary.json` (custom dictionary entries and aliases)
- `settings_store.json` (app preferences only; dictionary is no longer stored here)
- `user_store.json` (onboarding/profile/growth state)

Users need a first-party backup and restore flow that is safe for non-technical use. v1 prioritizes reliability and data safety while preserving a stable contract for future app/platform evolution.

## V1 Core Rules
All implementation tasks MUST satisfy these rules.

- `BR-001` Archive type: backup output MUST be one `.codictatebackup` ZIP file.
- `BR-002` Archive layout: backups created by v1 export MUST include `manifest.json`, `checksums.sha256`, `history/history.jsonl`, `dictionary/dictionary.json`, and `user/user_store.json`; `recordings/` is optional. `manifest.json` MUST include `estimated_size_bytes` and `platform`.
- `BR-003` History export: history MUST be exported as JSONL logical rows (not raw SQLite file copy).
- `BR-004` Backup choices: v1 MUST expose exactly two backup choices: complete backup (with recordings) and smaller backup (without recordings).
- `BR-005` Integrity checks: payload checksums MUST be generated and verified before restore writes, and every payload file MUST have a checksum entry.
- `BR-006` Preflight-first restore: restore MUST run non-destructive preflight and classify findings into blocking vs recoverable categories.
- `BR-007` Replace-only mode: v1 restore MUST be replace-only.
- `BR-008` Staged atomicity: restore MUST stage writes and only swap active data after staging validation succeeds.
- `BR-009` Rollback snapshot: restore MUST create a pre-swap snapshot of active data and rollback automatically on failure.
- `BR-010` Settings exclusion: v1 backup/restore MUST NOT include app settings payload; restore MUST keep current install settings unchanged.
- `BR-011` Stats handling: restore SHALL recompute stats from restored history rows in v1 (no stats snapshot import path).
- `BR-012` Archive path safety: restore MUST reject traversal, absolute-path, windows drive-prefixed path, control-character path segments, symlink, and hardlink entries, and MUST reject duplicate normalized paths (including case-fold collisions on case-insensitive filesystems).
- `BR-013` Operation mutex: only one backup/restore operation may run at a time.
- `BR-014` Progress reporting: export/restore MUST emit incremental progress events usable by a progress bar.
- `BR-015` i18n coverage: all user-facing backup/restore strings MUST use i18next keys under `settings.backup`.
- `BR-016` Compatibility contract: backups with format major `1` created by v1 app releases MUST remain restorable across later v1 app upgrades. Breaking payload changes MUST ship with explicit version bump plus migration path before release.
- `BR-017` Maintenance mode and operation gating: backup/restore MUST run in an exclusive app-maintenance mode that blocks new transcription/history/dictionary/profile writes for the full operation duration, and MUST reject operation start while recording/transcription is active.
- `BR-018` Commit-aware recovery marker: restore marker MUST distinguish uncommitted vs committed swap state so startup reconciliation only rolls back uncommitted restores.
- `BR-019` Free-space preflight gate: restore preflight MUST verify required free disk space for extraction/staging/snapshot + swap safety margin before apply.
- `BR-020` User-store fallback: if `user/user_store.json` is missing or malformed, restore SHALL keep current local onboarding/profile/growth state when available, else initialize defaults, and emit a recoverable warning.
- `BR-021` Platform compatibility scope: v1 restore compatibility is guaranteed for macOS-created backups; cross-platform restore remains best-effort in v1.
- `BR-022` Streaming and memory bounds: export/import SHALL process history and file payloads in streaming/chunked form and SHALL avoid loading full history or large payload files fully into memory.
- `BR-023` Extended restore bounds: restore preflight/import SHALL enforce hard bounds for archive entry count and total uncompressed payload bytes in addition to archive size/payload size/history-row bounds.
- `BR-024` Durable swap boundaries: marker updates and destructive swap boundaries MUST use durable file sync semantics (file + parent directory fsync) so startup reconciliation decisions are based on persisted state.
- `BR-025` Preflight summary UX: before destructive confirmation, restore SHALL show plain-language summary of backup identity and scope (created time, app version, platform, includes recordings, core counts) plus compatibility note.
- `BR-026` Additive schema tolerance: restore parsers for manifest/payload JSON MUST ignore unknown fields and default missing optional fields to preserve v1 forward compatibility.
- `BR-027` Least-privilege Tauri scope: backup/restore commands MUST be granted through a dedicated capability scoped to the `main` window only, and `tauri.conf.json` MUST explicitly list enabled capability identifiers.
- `BR-028` Undo last restore safety net: after successful restore commit, v1 SHALL attempt to retain one auto-checkpoint from the pre-swap snapshot for a short retention window and SHALL expose a one-click `Undo Last Restore` action while that checkpoint is available.

## Deferred (V1.1+)
The following items are intentionally deferred and not required for v1 implementation:
- Multi-phase restore marker model beyond `in_progress` and `committed`.
- Broad payload-version migration framework across many historical versions.
- Cross-platform filename remap/sanitization subsystem.
- Compression-ratio enforcement and formal resource-limit matrix.
- Expanded diagnostics taxonomy and multi-structure report contracts.
- Background backup/restore while core app actions remain available.

## Data Contracts

### Archive Layout
```text
<name>.codictatebackup
├── manifest.json
├── checksums.sha256
├── history/
│   └── history.jsonl
├── dictionary/
│   └── dictionary.json
├── user/
│   └── user_store.json
└── recordings/                    # only for complete backup
```

### Manifest (V1)
```json
{
  "backup_format_version": "1.0.0",
  "created_at": "2026-02-14T21:55:00Z",
  "created_with_app_version": "0.9.3",
  "platform": "macos",
  "includes_recordings": true,
  "estimated_size_bytes": 52428800,
  "counts": {
    "history_entries": 312,
    "recording_files": 288,
    "dictionary_entries": 47
  },
  "components": {
    "history_payload_version": 1,
    "dictionary_payload_version": 1,
    "user_store_payload_version": 1
  },
  "warnings": {
    "missing_recordings": []
  }
}
```

V1 manifest includes `platform` and `estimated_size_bytes` for forward compatibility and UX/reporting consistency.

### Dictionary Payload (V1)
`dictionary/dictionary.json` stores the serialized dictionary envelope from `user_dictionary.json`:
- `version` (currently `1`)
- `entries` (array of dictionary entries)

Restore MUST parse this payload and write dictionary data through the dictionary module/state path, not through app settings payloads.

### Settings Exclusion (V1)
Backups intentionally do not include app settings/preferences in v1.

Restore leaves the current install settings unchanged. This keeps v1 behavior simple and avoids settings-schema migration complexity in the initial rollout.

Dictionary is backed up/restored independently via `dictionary/dictionary.json` and does not depend on `settings_store.json`.

### User Store Payload (V1)
Backups produced by Codictate v1 include `user/user_store.json` so migration/restoration does not reset first-run UX.

If restore input is missing or malformed for this payload, restore keeps the current local `user_store.json` state when available and only falls back to defaults when no local state exists.

This preserves non-destructive restore behavior for partially damaged archives while keeping export output deterministic.

### Basic Safety Bounds (V1)
Use simple hard limits during restore preflight/import:
- `MAX_ARCHIVE_SIZE_BYTES = 10 GiB`
- `MAX_PAYLOAD_FILE_SIZE_BYTES = 512 MiB`
- `MAX_HISTORY_ROWS = 2_000_000`
- `MAX_ARCHIVE_ENTRIES = 2_100_000`
- `MAX_TOTAL_UNCOMPRESSED_BYTES = 20 GiB`

`MAX_HISTORY_ROWS` is intentionally set above the current product default history limit (`1_000_000`) to avoid blocking normal long-term users.

Bound breaches are blocking failures. These are practical v1 safeguards, not a full security-limits framework.

### Free-Space Gate (V1)
Restore preflight computes required free space before apply:
- extracted payload size estimate
- staging workspace size estimate
- rollback snapshot estimate
- fixed safety margin (for temp files and filesystem overhead)

If free space at app data root is below requirement, restore is blocked before maintenance-mode/swap with plain-language guidance:
- required space
- available space
- user action suggestions (free disk space, use smaller backup, or different volume)

### Checksum File
`checksums.sha256` lines:
`<sha256_hex><two_spaces><relative_path>`

Example:
```text
0a...ff  manifest.json
b1...3d  history/history.jsonl
```

### Undo Checkpoint (V1)
To protect non-technical users from accidental wrong-version restore, v1 attempts to keep one short-lived post-restore checkpoint:
- source: pre-swap snapshot already created for rollback
- retention: 7 days after successful restore commit
- cardinality: one checkpoint only when checkpoint publish succeeds (new successful restore replaces prior checkpoint)
- UX: show `Undo Last Restore` only while checkpoint exists and is not expired

If checkpoint publish fails (for example low disk), restore still succeeds and returns a warning that undo is unavailable for that run.
If checkpoint metadata is unavailable, corrupted, or expired, restore success remains valid and UI shows `Undo Last Restore` as unavailable with plain-language guidance.

## Export Algorithm (V1)
1. Acquire operation mutex (`BR-013`).
2. Verify app is idle (no active recording/transcription); if not idle, return blocking guidance (`BR-017`).
3. Enter maintenance mode and notify UI that core actions are temporarily unavailable (`BR-017`).
4. Resolve backup choice (complete/smaller) and output path.
5. Capture export consistency marker and payload snapshots required for deterministic export.
6. Build workspace payload files required by `BR-002`.
7. Stream history rows up to captured cutoff into `history/history.jsonl` (`BR-003`, `BR-022`).
8. If choice is complete backup, copy referenced recordings when present; missing files become warnings.
9. Generate `manifest.json` (including `platform`, `estimated_size_bytes`) and `checksums.sha256` (`BR-005`, `BR-026`).
10. Package workspace into `.codictatebackup` ZIP using chunked file reads/writes (`BR-001`, `BR-022`).
11. Emit progress events at major steps (`BR-014`).
12. On cancel/failure, clean temp artifacts and avoid partial output.
13. Exit maintenance mode in all outcomes (`BR-017`).
14. Return summary report (counts/warnings/path).

## Restore Algorithm (V1)
1. Acquire operation mutex (`BR-013`).
2. Verify app is idle (no active recording/transcription); if not idle, return blocking guidance (`BR-017`).
3. Run preflight before active-data writes (`BR-006`):
   - archive entry path safety checks (`BR-012`)
   - basic hard-bound checks (archive/payload size, row count, entry count, total uncompressed bytes) (`BR-023`)
   - required core files present (`manifest.json`, `checksums.sha256`, `history/history.jsonl`, `dictionary/dictionary.json`) and `user/user_store.json` handled via recoverable fallback policy (`BR-020`)
   - manifest parse + v1 compatibility gate (`BR-016`) and platform scope handling (`BR-021`)
   - checksum verification (`BR-005`)
   - free-space feasibility gate (`BR-019`)
4. If blocking findings exist, abort with no active-data changes.
5. Show preflight summary UX and obtain destructive confirmation (`BR-025`).
6. Enter maintenance mode (`BR-017`) to block new transcription/history/dictionary/profile writes.
7. Prepare staging workspace and import payloads.
8. Import history rows from `history/history.jsonl` using streaming row processing (`BR-022`).
9. Recompute stats from imported history rows (`BR-011`).
10. Restore dictionary payload from `dictionary/dictionary.json`.
11. Preserve current install settings unchanged (`BR-010`).
12. Restore `user/user_store.json` payload; if missing/malformed, keep current local user-store if available, else initialize defaults, and record warning (`BR-020`).
13. Validate staging (SQLite integrity and basic count checks).
14. Create rollback snapshot of active dataset (`BR-009`).
15. Write marker `{ "state": "in_progress", "snapshot_path": "..." }` and durably sync marker (`BR-018`, `BR-024`).
16. Perform replace-only swap from staging to active paths (`BR-007`, `BR-008`) with durable sync boundaries (`BR-024`).
17. Update marker to committed state `{ "state": "committed", ... }` and durably sync marker (`BR-018`, `BR-024`).
18. Publish undo checkpoint metadata from the rollback snapshot with 7-day expiry (`BR-028`).
19. Cleanup staging artifacts and clear marker (checkpoint retained until expiry/undo/new restore).
20. On failure before commit, rollback from snapshot, clear marker, and return failure summary.
21. Exit maintenance mode in all outcomes (`BR-017`).
22. Emit progress events at major steps (`BR-014`).

## Undo Last Restore Algorithm (V1)
1. Acquire operation mutex (`BR-013`).
2. Validate checkpoint metadata exists and is not expired (`BR-028`).
3. Enter maintenance mode (`BR-017`).
4. Stage checkpoint payload as restore source and validate staging integrity.
5. Execute replace-only swap using the same durable marker/sync boundaries as restore apply (`BR-018`, `BR-024`).
6. Clear used checkpoint on successful undo (single-use behavior).
7. Exit maintenance mode and emit concise success/failure summary.

## Startup Reconciliation (V1)
- If marker state is `in_progress`:
  - rollback from `snapshot_path`
  - clear marker
  - emit concise user-facing outcome message
- If marker state is `committed`:
  - keep active restored dataset
  - run cleanup for stale rollback artifacts
  - clear marker
  - emit concise user-facing outcome message
- Independently of marker state, startup prunes expired undo checkpoints (`BR-028`).

This commit-aware model avoids rolling back successfully swapped restores after post-commit crashes.

## Compatibility Contract (V1)
- A backup with `backup_format_version` major `1` produced by a macOS v1 app release MUST be restorable by later macOS v1 releases.
- Cross-platform restore (e.g. macOS backup restored on Windows/Linux) is best-effort in v1 and not guaranteed.
- Restore parsers MUST ignore unknown additive JSON fields and default missing optional JSON fields for v1 payloads (`BR-026`).
- If a new payload schema version is introduced in v1, the release MUST include migration from prior supported v1 payload versions before shipping.
- If a change cannot satisfy prior v1 compatibility, the backup format major MUST be incremented.

## Logging Plan
- Emit structured backend logs for major milestones only.
- Use metadata-only fields (counts, durations, step names).
- Never log transcript text, prompt text, API keys, or raw payload bodies.

## Tauri Capabilities Required
- Dedicated `backup-restore-main` capability bound to `windows: ["main"]` only (`BR-027`).
- For v1 target release, that dedicated capability is constrained to macOS platform only.
- Backup/restore command permissions assigned only to that capability (`BR-027`).
- Explicit `app.security.capabilities` list in `tauri.conf.json` to avoid implicit capability enablement (`BR-027`).
- Minimal plugin permissions for backup UI flow:
  - `dialog:allow-save`
  - `dialog:allow-open`

Backup/restore read/write operations are performed in Rust command handlers under application-controlled path validation and staging/swap boundaries.

## Goals / Non-Goals
- Goals:
  - Safe and practical local backup/restore for v1.
  - No local data loss on restore failure.
  - Preserve user experience state (onboarding/profile/growth) across migrations.
  - Simple user flow and clear operation progress.
- Non-Goals:
  - Cloud backup/sync.
  - Merge restore.
  - App-level backup encryption.
  - Enterprise-grade multi-year migration framework in v1.

## Risks / Trade-offs
- Smaller backups omit recordings.
  - Mitigation: explicit UI copy and graceful missing-audio handling.

- Backups do not include preferences/settings.
  - Mitigation: explicit restore copy that current settings remain unchanged and may need manual reconfiguration.

- No authenticity signatures in v1.
  - Mitigation: clearly disclose checksums detect corruption, not trusted origin.

- App is unavailable for core actions during backup/restore.
  - Mitigation: explicit pre-start notice, progress visibility, cancellation support, and an up-front estimate from manifest/size metadata when available.

- Cross-platform restore is best-effort in v1.
  - Mitigation: message this explicitly in docs and restore preflight output.

- Retaining one undo checkpoint temporarily increases disk usage.
  - Mitigation: single-checkpoint policy, fixed 7-day expiry, and plain-language storage messaging.

## Edge Cases Covered in V1
- Missing recordings during complete backup (warning, not failure).
- Restore does not overwrite current install settings (by design).
- Corrupted required payloads (blocking restore failure).
- Unsafe archive entries (blocking restore failure).
- Duplicate/case-collision archive paths (blocking restore failure before extraction).
- Insufficient disk space for staging/snapshot (blocking restore failure before apply).
- Operation start while recording/transcription is active (blocking with plain-language guidance).
- Cancellation before commit (cleanup, active data unchanged).
- Interruption during restore before commit (startup rollback from snapshot).
- Interruption after commit before marker clear (startup keeps restored data and performs cleanup).
- Unsupported backup version (preflight rejection).
- Missing or malformed `user/user_store.json` (recoverable warning with preserve-local fallback, else defaults).
- Restore of backups with large but expected history volumes up to practical hard bounds.
- User requests undo after checkpoint expiry or after it was replaced by newer restore (plain-language unavailable state).
- Startup finds corrupted undo checkpoint metadata (checkpoint pruned, restore remains successful).
- Backup/restore invocation from non-main window (blocked as unauthorized).
