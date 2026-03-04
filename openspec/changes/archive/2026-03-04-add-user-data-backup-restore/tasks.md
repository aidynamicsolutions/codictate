## 1. OpenSpec Validation
- [x] 1.1 Confirm proposal/design/spec are aligned to v1 core rules `BR-001` to `BR-028`.
- [x] 1.2 Run `openspec validate add-user-data-backup-restore --strict` and fix issues.

## 2. Backend Command Surface
- [x] 2.1 Add `src-tauri/src/commands/backup.rs` and register commands in `src-tauri/src/commands/mod.rs`.
- [x] 2.2 Add command contracts:
  - [x] `CreateBackupRequest` (`scope`, `output_path`)
  - [x] `CreateBackupReport` (counts, warnings, output path)
  - [x] `BackupEstimateReport` (complete/smaller estimate, delta, counts)
  - [x] `PreflightRestoreRequest` / `PreflightRestoreReport` (blocking + recoverable findings)
  - [x] `ApplyRestoreRequest` / `ApplyRestoreReport`
  - [x] `UndoLastRestoreRequest` / `UndoLastRestoreReport`
  - [x] `UndoLastRestoreAvailabilityReport`
  - [x] `BackupProgress` (`phase`, `current`, `total`)
- [x] 2.2.a Add `get_backup_estimate` command for pre-export scope sizing.
- [x] 2.3 Add operation mutex so only one backup/restore runs at a time (`BR-013`).
- [x] 2.4 Add maintenance mode gate for full operation duration and reject operation start while recording/transcription is active (`BR-017`).
- [x] 2.5 Add cancel token plumbing for export/restore cancellation and cleanup.
- [x] 2.6 Add restore marker persistence with states (`in_progress`, `committed`) and snapshot path (`BR-018`).
- [x] 2.7 Add one-slot undo checkpoint metadata store (path, created_at, expires_at) (`BR-028`).

## 3. Export Pipeline
- [x] 3.1 Implement workspace builder for required archive layout (`BR-002`), including `user/user_store.json`.
- [x] 3.2 Capture deterministic export cutoff + dictionary/user-store snapshots while in maintenance mode (`BR-017`).
- [x] 3.3 Implement streaming history logical export to `history/history.jsonl` (`BR-003`, `BR-022`).
- [x] 3.4 Implement dictionary export to `dictionary/dictionary.json` from `user_dictionary.json` state/file.
- [x] 3.5 Implement user-store export to `user/user_store.json`.
- [x] 3.6 Implement scope handling (`BR-004`):
  - [x] complete backup includes available recordings
  - [x] smaller backup excludes recordings
- [x] 3.7 Implement missing-recordings warning behavior for complete backups.
- [x] 3.8 Implement manifest generation (`backup_format_version`, `platform`, `estimated_size_bytes`, counts, component versions, warnings).
- [x] 3.9 Implement `checksums.sha256` generation for payload files (`BR-005`).
- [x] 3.10 Package to `.codictatebackup` ZIP with chunked I/O and write output atomically (`BR-001`, `BR-022`).
- [x] 3.11 Emit progress events at major export steps (`BR-014`).
- [x] 3.12 Implement export cancellation checks and cleanup.

## 4. Restore Preflight
- [x] 4.1 Validate archive entry metadata before extraction (`BR-012`):
  - [x] reject traversal paths (`..`)
  - [x] reject absolute paths
  - [x] reject windows drive-prefixed paths (e.g. `C:/...`)
  - [x] reject symlink/hardlink entries
  - [x] reject duplicate normalized relative paths
  - [x] reject case-fold collisions for case-insensitive filesystems
- [x] 4.2 Enforce practical hard bounds before parsing/import (`BR-023`):
  - [x] max archive size
  - [x] max payload file size
  - [x] max history row count (set above default history limit)
  - [x] max archive entry count
  - [x] max total uncompressed payload bytes
- [x] 4.3 Validate required core files exist (`manifest.json`, `checksums.sha256`, `history/history.jsonl`, `dictionary/dictionary.json`) and classify missing/malformed `user/user_store.json` as recoverable preserve-local fallback warning (`BR-020`).
- [x] 4.4 Parse manifest (including `platform`) and enforce v1 compatibility contract (`BR-016`, `BR-021`, `BR-026`).
- [x] 4.5 Verify checksums before apply-restore and require checksum entries for all payload files (`BR-005`, `BR-006`).
- [x] 4.6 Add free-space feasibility check and block apply when available disk is insufficient (`BR-019`).
- [x] 4.7 Build preflight report with blocking/recoverable findings and plain-language summary.
- [x] 4.8 Convert malformed/unsafe archive parse and scan failures into structured blocking findings (avoid hard command failures for user-caused archive issues).
- [x] 4.9 Add local history integrity gate to preflight; block restore when current local history data is corrupted.

## 5. Restore Apply (Replace-Only)
- [x] 5.1 Enter maintenance mode before staging/import and always release it on exit (`BR-017`).
- [x] 5.2 Build staging restore workspace (`BR-008`).
- [x] 5.3 Import history rows with streaming row processing and validate required payloads (`BR-022`).
- [x] 5.4 Recompute stats from restored history rows (`BR-011`).
- [x] 5.5 Import dictionary payload via dictionary module/state (not settings payload).
- [x] 5.6 Preserve current install settings unchanged during restore (`BR-010`).
- [x] 5.7 Restore `user/user_store.json` payload; if missing/malformed, keep current local state when available else initialize defaults and emit recoverable warning (`BR-020`).
- [ ] 5.8 Copy recordings when present using chunked I/O and mark missing optional files as warnings.
- [x] 5.9 Run staging validation (SQLite integrity + count consistency).
- [x] 5.10 Create pre-swap rollback snapshot (`BR-009`).
- [x] 5.11 Persist marker `in_progress` before destructive swap and `committed` after successful swap (`BR-018`).
- [x] 5.12 Apply durable file/dir fsync boundaries for marker writes and destructive swap transitions (`BR-024`).
- [x] 5.13 Execute replace-only swap (`BR-007`, `BR-008`).
- [x] 5.14 On failure before commit, rollback from snapshot and clear marker (`BR-009`, `BR-018`).
- [x] 5.15 On success, retain pre-swap snapshot as one-slot undo checkpoint with 7-day expiry, cleanup staging artifacts, and clear marker (`BR-028`).
- [x] 5.16 Emit progress events during restore (`BR-014`).
- [x] 5.17 Emit refresh events so frontend updates history/dictionary/profile after success.
- [x] 5.18 Implement `Undo Last Restore` apply flow using the same replace-only staged swap boundaries and clear checkpoint after successful undo (`BR-028`).

## 6. Startup Reconciliation
- [x] 6.1 On app startup, detect restore marker state.
- [x] 6.2 If marker is `in_progress`, rollback from snapshot path and clear marker.
- [x] 6.3 If marker is `committed`, keep restored data, cleanup stale rollback artifacts, and clear marker.
- [ ] 6.4 Surface concise user-facing reconciliation outcome message.
- [x] 6.5 Prune expired or malformed undo-checkpoint metadata on startup (`BR-028`).

## 7. Frontend UX
- [x] 7.1 Move backup/restore UI to `Settings > General` after `Advanced` and remove it from History.
- [x] 7.2 Keep card surface uncluttered with exactly two primary actions: `Create backup` and `Restore backup`.
- [x] 7.3 Use a create-backup modal with one editable scope control (`Include recordings`) mapped to complete/smaller scope.
- [x] 7.4 Show maintenance-mode notice in create modal (not preemptive card banner) (`BR-017`).
- [x] 7.5 Show live estimated backup size and savings delta while toggling recordings inclusion.
- [x] 7.6 Show unencrypted backup disclosure as post-success informational notice (not preemptive card banner).
- [x] 7.7 Show preflight outcomes in plain language by default (`can't continue` vs `can continue with warnings`).
- [x] 7.8 Show restore preflight summary before destructive confirmation (backup date/version/platform/scope/counts) (`BR-025`).
- [x] 7.9 Show macOS-guaranteed vs cross-platform best-effort compatibility copy in restore flow (`BR-021`).
- [x] 7.10 Add progress bar for export and restore (`BR-014`).
- [x] 7.11 Add cancellation states (in progress, canceled safely, failed) for backup/restore operations.
- [x] 7.12 Add missing-audio user-facing handling after smaller/partial restores.
- [x] 7.13 Render `Undo Last Restore` action only when checkpoint is available (`BR-028`).
- [x] 7.14 Render restore findings as concise localized plain-language categories (do not show raw backend error text by default).
- [x] 7.15 Harmonize backup/restore surface styling with existing settings subsection patterns (heading + divided rows + neutral action emphasis).
- [x] 7.16 Remember last user-selected backup folder for create/save and restore/open dialog start locations.
- [x] 7.17 Add explicit restore file-selection guidance in UI and picker copy (`.codictatebackup` expectation).
- [x] 7.18 Keep restore confirm CTA stable as `Start Restore` (no conditional button labels).
- [x] 7.19 Add preflight inline impact panel for fresh-install vs replace-current messaging and explicit settings-exclusion disclosure.
- [x] 7.20 Add inline recording-removal warning when restoring a no-recordings backup over local recordings.
- [x] 7.21 Improve create-backup scope copy to explicitly describe complete vs smaller backup choices and consequences.
- [x] 7.22 Format undo availability time in localized human-readable form.

## 8. Permissions and Security Scope
- [x] 8.1 Add dedicated backup/restore capability file scoped to `windows: ["main"]` only (`BR-027`).
- [x] 8.1.a Constrain v1 backup/restore capability to macOS target platform.
- [x] 8.2 Assign backup/restore command permissions to that dedicated capability and reject non-main window invocation (`BR-027`).
- [x] 8.3 Configure required plugin permissions:
  - [x] `dialog:allow-save`
  - [x] `dialog:allow-open`
- [x] 8.4 Restrict file operations to selected archive files and app data directories.
- [x] 8.5 Explicitly list enabled capability identifiers in `tauri.conf.json` (`BR-027`).

## 9. Internationalization and Logging
- [x] 9.1 Add `settings.backup` translation keys to `src/i18n/locales/en/translation.json` (`BR-015`).
- [x] 9.2 Ensure all backup/restore UI strings use `t()` (`BR-015`).
- [x] 9.3 Add structured backend/frontend logging for major milestones only.
- [x] 9.4 Enforce log redaction: no transcript text, prompts, API keys, or raw payload bodies.

## 10. Test Matrix (V1 Core)
- [x] 10.1 Integration: complete backup round-trip restores history/dictionary/user-store/recordings.
- [x] 10.2 Integration: smaller backup round-trip restores history/dictionary/user-store without recordings.
- [x] 10.3 Integration: checksum mismatch is rejected in preflight.
- [x] 10.3.a Integration: preflight rejects restore when a payload file is missing from `checksums.sha256`.
- [x] 10.4 Integration: injected restore failure triggers rollback and preserves pre-restore data.
- [x] 10.5 Integration: startup reconciliation rolls back safely when marker is `in_progress`.
- [x] 10.6 Integration: startup reconciliation keeps restored data when marker is `committed`.
- [x] 10.7 Unit: archive entry path safety checks reject traversal/absolute/symlink/hardlink/duplicate/case-collision paths.
- [x] 10.7.a Unit: archive entry path safety checks reject windows drive-prefixed paths.
- [x] 10.7.b Unit: archive/checksum path safety checks reject control-character paths; export rejects control-character history filenames.
- [ ] 10.8 Unit: manifest serialization/deserialization (including `platform` and `estimated_size_bytes`) and v1 version gate.
- [ ] 10.9 Unit: parser compatibility ignores unknown additive fields and defaults missing optional fields (`BR-026`).
- [ ] 10.10 Unit: backup layout excludes app settings payload and restore keeps current install settings unchanged while restoring dictionary via dedicated dictionary payload path.
- [ ] 10.11 Unit: stats handling recomputes from restored history rows.
- [ ] 10.12 Integration: operation mutex rejects concurrent backup/restore request.
- [ ] 10.13 Integration: operation start is blocked while recording/transcription is active; backup/restore maintenance mode blocks new writes for duration.
- [ ] 10.14 Integration: cancellation cleans temp artifacts and leaves active data unchanged.
- [ ] 10.15 Integration: full backup with absent recording files succeeds and emits missing-recordings warnings.
- [ ] 10.16 Unit: checksum generation correctness matches expected SHA-256 output for fixed test fixtures.
- [ ] 10.17 Unit: history row bound allows at least the product default-scale dataset and blocks only above configured hard limit.
- [ ] 10.18 Integration: preflight blocks restore when free disk space is insufficient and reports required vs available space.
- [x] 10.19 Integration: restore with missing/malformed `user/user_store.json` preserves current local user-store state when available, else initializes defaults, and emits recoverable warning.
- [ ] 10.20 Unit/Integration: preflight entry-count and total-uncompressed-byte hard bounds reject oversized archives before apply.
- [ ] 10.21 Integration: crash simulation around marker/swap boundaries preserves committed restore and rolls back only uncommitted restore (`BR-018`, `BR-024`).
- [ ] 10.22 Integration: successful restore publishes at most one undo checkpoint and replaces prior checkpoint when publish succeeds (`BR-028`).
- [x] 10.23 Integration: undo restore reverts active data when checkpoint is available and clears checkpoint after success (`BR-028`).
- [ ] 10.24 Integration: undo request after checkpoint expiry/replacement returns plain-language unavailable without data changes (`BR-028`).
- [x] 10.25 Integration: backup/restore command invocation from non-main window is rejected (`BR-027`).
- [x] 10.26 Integration: preflight on non-ZIP archive returns blocking finding in report instead of hard command failure.
- [x] 10.27 Integration: preflight blocks restore when current local history DB is corrupted and surfaces explicit blocking finding.
- [x] 10.28 Unit: backup dialog pathing helpers normalize parent directories across Unix/Windows paths and compose save/open defaults with safe fallback behavior.
- [x] 10.29 Frontend unit: restore preflight helper derives fresh-install and recording-removal impact state from local counts + backup summary.
- [x] 10.30 Frontend unit: friendly date formatting fallback covers preflight created-at and undo expiration labels.

## 11. Documentation
- [x] 11.1 Add `doc/backup-restore.md` with:
  - [x] feature overview and user steps
  - [x] archive format and scope behavior (complete vs smaller)
  - [x] maintenance-mode behavior during backup/restore and plain-language operation timing expectations
  - [x] undo-last-restore behavior (single checkpoint, 7-day retention, replace-on-new-restore)
  - [x] settings exclusion behavior, dedicated dictionary payload, and user-store preserve-local fallback behavior
  - [x] compatibility contract for v1 upgrades
  - [x] known v1 limitations (unencrypted archive, replace-only restore, cross-platform best-effort)
- [x] 11.2 Add short backup/restore checklist entry in `doc/prodRelease.md`.

## 12. Post-Review Hardening
- [x] 12.1 Add a shared write-permit gate around restore-managed write paths to close maintenance-mode race windows.
- [x] 12.2 Add concurrency tests for write-permit vs operation-start serialization and fast-fail behavior while an operation is active.
- [x] 12.3 Split `src-tauri/src/backup_restore.rs` into focused modules under `src-tauri/src/backup_restore/`.
- [x] 12.4 Document the operation-gate/write-gate concurrency model in `doc/backup-restore.md`.
