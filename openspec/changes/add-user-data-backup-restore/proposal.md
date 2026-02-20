# Change: Add User Data Backup and Restore

## Why
Users currently risk losing important local data (history text/audio and dictionary entries) when uninstalling or moving machines. The app needs a first-class export/import flow so users can own their data and restore a previous state after reinstall.

This change also needs a clear long-term compatibility policy because backup payloads outlive any single app version. Without an explicit schema/versioning strategy, restores become brittle as internal storage evolves.

## What Changes
- Add a user-driven backup export flow that writes one portable archive file (`.codictatebackup`) using file-save dialog selection.
- Add a user-driven restore flow that reads one backup archive selected by file-open dialog and restores replace-only in v1.
- Add explicit no-local-data-loss contract for existing users: restore failure/cancel/interruption MUST preserve pre-restore local `history`, `stats`, and `dictionary` state.
- Add mandatory pre-swap rollback snapshot: before swapping staged restore data into active paths, preserve current active data in a local rollback workspace so failure/interruption can automatically restore prior state.
- Add rollback snapshot retention policy: keep latest pre-restore rollback snapshot for 7 days with automatic cleanup and no extra user decision flow.
- Define a concrete v1 archive layout:
  - `manifest.json`
  - `checksums.sha256`
  - `history/history.jsonl`
  - `history/user_stats.json`
  - `dictionary/dictionary.json`
  - `settings/settings.json` (selective app settings export)
  - `recordings/` (optional, only when user includes audio)
- Add selective settings backup: export user-configurable preferences (shortcuts, language, overlay, audio feedback, etc.) but exclude sensitive data (API keys) and runtime/device-specific values. On restore, merge selectively into current settings using forward-compatible defaults for any new fields added since the backup was created.
- Add integrity controls:
  - SHA-256 checksums for all payload files
  - mandatory preflight validation before restore
  - explicit v1 integrity scope disclosure: checksums detect corruption but do not provide authenticity/signature guarantees
- Add deterministic migration and compatibility rules:
  - payload-version migration support for all known payload versions within format major `1`
  - format-major transition policy: support prior major imports for at least 24 months with explicit deprecation runway
  - explicit block for too-old and forward-incompatible backups
- Add atomic restore with staging + rollback so active user data is unchanged on any restore failure.
- Add progress reporting: emit incremental progress events during export/restore so the frontend can display a progress bar (ETA is optional best-effort).
- Add estimated archive size preview before export so users know the approximate output size before selecting a destination.
- Add short export snapshot consistency fence: briefly block writes only while capturing history/stats/dictionary snapshot, then continue export without long lock.
- Add v1 lightweight backup behavior:
  - user may exclude recordings
  - history text still restores
  - audio playback for missing files must fail gracefully with user-facing message
- Add explicit edge-case handling rules:
  - classify restore findings into **blocking** (must stop) and **recoverable** (auto-handle optional issues with concise summaries)
  - reject unsafe archives (path traversal, absolute-path entries, symlink entries)
  - enforce hard non-overridable archive security bounds (path safety and decompression abuse defense)
  - block operations when disk space is insufficient (both source and destination)
  - support cancellation with cleanup and no partial active-data changes
  - recover safely from interruption/crash during restore via staging + startup reconciliation
  - reject malformed required payload data as corruption; auto-skip only optional recording file issues with warnings
- Add cross-platform universal backup support: normalize filenames and paths at archive creation and restore to ensure backups created on macOS work on Windows/Linux and vice versa. Unsafe/invalid names in required payload paths are blocking; invalid recording filenames are recoverable and sanitized or skipped with warnings.
- Add structured logging at every major backup/restore milestone for debugging, including blocking vs recoverable preflight findings and skip counts, with strict redaction rules (no transcript text, prompts, or API keys in logs).
- Add i18n translation keys for all backup/restore UI strings.
- Keep backup encryption out of scope for v1 and disclose that exported archives are unencrypted.
- Add concise preflight/restore summaries for non-technical users, with optional diagnostics behind explicit `View details`.

## Impact
- **Affected specs**: `data-backup-restore` (new)
- **Affected code** (planned): `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/backup.rs`, `src-tauri/src/managers/history.rs`, `src-tauri/src/settings.rs`, `src/components/settings/history/HistorySettings.tsx` (or a dedicated backup settings component), `src/bindings.ts`, `src-tauri/capabilities/*.json`, `src/i18n/locales/*/translation.json`
- **New documentation**: `doc/backup-restore.md` (feature docs, ADR, error catalog), `doc/prodRelease.md` (backup compatibility section)
- **Behavioral impact**: users get a Codictate-branded portable archive backup, a safe restore path using staging + automatic rollback snapshot, simple progress display (ETA optional), long-lived payload-version compatibility, selective settings restore, cross-platform portability, and concise non-technical summaries.

## Best-Practice References
- Tauri capability model and least-privilege permissions: https://v2.tauri.app/security/capabilities/
- Tauri file-system scope control (restrict allowed paths): https://v2.tauri.app/plugin/file-system/
- Tauri dialog APIs for explicit user file selection: https://v2.tauri.app/reference/javascript/dialog/
- SQLite online backup API for consistent snapshots: https://sqlite.org/backup.html
- SQLite `VACUUM INTO` as a compact backup approach: https://sqlite.org/lang_vacuum.html#vacuuminto
- SQLite corruption guidance (copy WAL/SHM side files or use backup API): https://sqlite.org/howtocorrupt.html
- SQLite integrity verification (`PRAGMA integrity_check`): https://sqlite.org/pragma.html#pragma_integrity_check
- SQLite WAL guidance: https://sqlite.org/wal.html
- Apple app file management guidance (Application Support for app-owned data; open/save panels for user-chosen locations): https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/AppFileMgmt/Tasks/AccessingFilesandDirectories.html
- Apple App Sandbox user-selected file entitlements: https://developer.apple.com/documentation/xcode/enabling-app-sandbox
- Microsoft app data versioning guidance: https://learn.microsoft.com/en-us/windows/apps/design/app-settings/store-and-retrieve-app-data
- XDG Base Directory spec for Linux data/state locations: https://specifications.freedesktop.org/basedir-spec/latest/
- BagIt manifest/checksum model reference (packaging pattern): https://www.rfc-editor.org/rfc/rfc8493.html
