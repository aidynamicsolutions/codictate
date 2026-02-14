## 1. OpenSpec and Rule Lock
- [ ] 1.1 Confirm v1 rules `BR-001` to `BR-023` are represented in proposal/design/spec files.
- [ ] 1.2 Validate OpenSpec change with `openspec validate add-user-data-backup-restore --strict`.

## 2. Backend Contracts and Command Surface
- [ ] 2.1 Add `src-tauri/src/commands/backup.rs` and register commands in `src-tauri/src/commands/mod.rs`.
- [ ] 2.2 Define typed request/response contracts in Rust and generated frontend bindings:
- [ ] `CreateBackupRequest` with scope (`full` or `lightweight`) and user-selected output path.
- [ ] `CreateBackupReport` with counts, warnings, and output path.
- [ ] `PreflightRestoreRequest` and `PreflightRestoreReport`.
- [ ] `ApplyRestoreRequest` and `ApplyRestoreReport`.
- [ ] 2.3 Add an app-level operation mutex so only one backup/restore runs concurrently (`BR-016`).
- [ ] 2.4 Add operation cancel token plumbing for export/restore flows (`BR-020`).
- [ ] 2.5 Add persistent restore-in-progress marker state for crash reconciliation (`BR-021`).

## 3. Export Pipeline Implementation
- [ ] 3.1 Implement archive workspace builder that writes exact layout required by `BR-002`.
- [ ] 3.2 Implement history logical export (`BR-005`):
- [ ] query `transcription_history` rows into `history/history.jsonl`.
- [ ] query `user_stats` into `history/user_stats.json`.
- [ ] do not copy `history.db` into backup artifact.
- [ ] 3.3 Implement dictionary export from settings payload into `dictionary/dictionary.json`.
- [ ] 3.4 Implement scope handling (`BR-006`):
- [ ] full scope copies available referenced recordings.
- [ ] lightweight scope omits `recordings/`.
- [ ] 3.5 Implement missing-recording warning behavior for full backups (`BR-007`).
- [ ] 3.6 Implement manifest generation with required fields (`BR-003`).
- [ ] 3.7 Implement SHA-256 checksum file generation for all payload files (`BR-004`).
- [ ] 3.8 Package workspace into `.codictatebackup` ZIP and write output atomically (`BR-001`).
- [ ] 3.9 Return `CreateBackupReport` with counts and warnings (`BR-015`).
- [ ] 3.10 Add export storage-capacity precheck and low-space error path (`BR-019`).
- [ ] 3.11 Add export cancellation checkpoints and temp/output cleanup behavior (`BR-020`).

## 4. Restore Preflight Implementation
- [ ] 4.1 Implement archive unpack to temp workspace.
- [ ] 4.2 Validate required files exist (`BR-002`).
- [ ] 4.3 Parse and validate manifest fields (`BR-003`).
- [ ] 4.4 Verify checksums before any write (`BR-004`, `BR-008`).
- [ ] 4.5 Enforce compatibility policy `current major + previous major` (`BR-009`).
- [ ] 4.6 Validate archive entry safety (reject traversal, absolute path, symlink/hardlink entries) (`BR-018`).
- [ ] 4.7 Add restore storage-capacity precheck and low-space error path (`BR-019`).
- [ ] 4.8 Build and return `PreflightRestoreReport` with:
- [ ] compatibility result
- [ ] payload counts
- [ ] warning list (including missing recordings declared by manifest)

## 5. Restore Apply Implementation (Replace-Only)
- [ ] 5.1 Implement component migration pipeline (`BR-010`) for history payload and dictionary payload versions.
- [ ] 5.2 Create staging restore area with fresh DB initialized through current migrations.
- [ ] 5.3 Import migrated history rows and user stats into staging DB.
- [ ] 5.4 Import dictionary payload into staged settings payload.
- [ ] 5.5 Copy recordings from archive when present.
- [ ] 5.6 Implement deterministic conflict resolution:
- [ ] duplicate history IDs are rekeyed deterministically.
- [ ] duplicate recording filenames are deterministically renamed and references remapped.
- [ ] 5.7 Run staging validation:
- [ ] SQLite integrity check passes.
- [ ] payload counts match import counts.
- [ ] 5.8 Execute replace-only swap (`BR-011`, `BR-012`):
- [ ] move active restore-managed data to rollback location.
- [ ] move staged data into active app data paths.
- [ ] 5.9 Implement rollback path on any failure (`BR-013`).
- [ ] 5.10 Emit state refresh events (history/settings) after successful restore.
- [ ] 5.11 Return `ApplyRestoreReport` with restored counts, warnings, and failures (`BR-015`).
- [ ] 5.12 Add restore cancellation checkpoints and cleanup behavior (`BR-020`).
- [ ] 5.13 Implement startup reconciliation using restore-in-progress marker (`BR-021`).

## 6. Missing Audio Runtime Behavior
- [ ] 6.1 Update audio lookup/playback flow to detect missing recording file and return typed error instead of crash (`BR-014`).
- [ ] 6.2 Add frontend handling for missing-audio error with clear user message (`BR-014`).
- [ ] 6.3 Ensure lightweight backup restores remain usable for history text even without recordings.

## 7. Frontend UX and Guardrails
- [ ] 7.1 Add backup UI controls in Settings with two scope options only (`full`, `lightweight`) (`BR-006`).
- [ ] 7.2 Add explicit unencrypted notice in export flow (`BR-017`).
- [ ] 7.3 Add restore flow with two steps:
- [ ] Step A: preflight summary view from `PreflightRestoreReport`.
- [ ] Step B: replace-only confirmation and apply action.
- [ ] 7.4 Show compatibility gate errors and remediation messages for unsupported versions (`BR-009`).
- [ ] 7.5 Show success/failure summary using structured reports (`BR-015`).

## 8. Permissions and Capabilities
- [ ] 8.1 Verify Tauri capabilities/permissions are minimal and sufficient for user-selected backup paths.
- [ ] 8.2 Ensure restore/export only operate on user-selected files plus app data paths (`BR-023`).

## 9. Test Matrix
- [ ] 9.1 Unit tests for manifest serializer/deserializer and checksum generation (`BR-003`, `BR-004`).
- [ ] 9.2 Unit tests for compatibility gate rules (`BR-009`).
- [ ] 9.3 Unit tests for migration dispatcher and missing migration failure (`BR-010`).
- [ ] 9.4 Integration test: full backup round-trip with recordings (`BR-001`, `BR-006`, `BR-012`).
- [ ] 9.5 Integration test: lightweight backup round-trip without recordings (`BR-006`, `BR-014`).
- [ ] 9.6 Integration test: preflight rejects corrupted archive (checksum mismatch) (`BR-004`, `BR-008`).
- [ ] 9.7 Integration test: preflight rejects too-old and forward-incompatible versions (`BR-009`).
- [ ] 9.8 Integration test: restore failure triggers rollback and leaves active data unchanged (`BR-013`).
- [ ] 9.9 Integration test: missing recordings in full export produce warnings but export succeeds (`BR-007`).
- [ ] 9.10 Frontend tests for unencrypted notice, preflight summary, replace-only warning, and missing-audio message.
- [ ] 9.11 Integration test: archive path traversal entry is rejected (`BR-018`).
- [ ] 9.12 Integration test: archive symlink/hardlink entry is rejected (`BR-018`).
- [ ] 9.13 Integration test: low-disk export and restore prechecks fail safely (`BR-019`).
- [ ] 9.14 Integration test: cancel export and restore clean up temp artifacts and keep active data intact (`BR-020`).
- [ ] 9.15 Integration test: crash/interruption during restore reconciles safely on next startup (`BR-021`).
- [ ] 9.16 Integration test: duplicate ID/filename payload imports preserve all rows with correct remapping (`BR-022`).
- [ ] 9.17 Integration test: concurrent backup/restore request is rejected while operation lock is held (`BR-016`).
- [ ] 9.18 Integration test: out-of-scope path operations are rejected (`BR-023`).

## 10. Documentation
- [ ] 10.1 Add backup/restore user guide with full and lightweight examples.
- [ ] 10.2 Document archive format, compatibility window, and migration guidance for unsupported versions.
- [ ] 10.3 Document that v1 backups are unencrypted and suggest external encryption options.
