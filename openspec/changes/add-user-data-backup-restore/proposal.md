# Change: Add User Data Backup and Restore

## Why
Users currently risk losing important local data (history text/audio and dictionary entries) when uninstalling or moving machines. The app needs a first-class export/import flow so users can own their data and restore a previous state after reinstall.

This change also needs a clear long-term compatibility policy because backup payloads outlive any single app version. Without an explicit schema/versioning strategy, restores become brittle as internal storage evolves.

## What Changes
- Add a user-driven backup export flow that writes one portable archive file (`.codictatebackup`) using file-save dialog selection.
- Add a user-driven restore flow that reads one backup archive selected by file-open dialog and restores replace-only in v1.
- Define a concrete v1 archive layout:
- `manifest.json`
- `checksums.sha256`
- `history/history.jsonl`
- `history/user_stats.json`
- `dictionary/dictionary.json`
- `recordings/` (optional, only when user includes audio)
- Add integrity controls:
- SHA-256 checksums for all payload files
- mandatory preflight validation before restore
- Add deterministic migration and compatibility rules:
- backup format semver with support window `current major + previous major`
- explicit block for too-old and forward-incompatible backups
- Add atomic restore with staging + rollback so active user data is unchanged on any restore failure.
- Add v1 lightweight backup behavior:
- user may exclude recordings
- history text still restores
- audio playback for missing files must fail gracefully with user-facing message
- Add explicit edge-case handling rules:
- reject unsafe archives (path traversal, absolute-path entries, symlink entries)
- block operations when disk space is insufficient
- support cancellation with cleanup and no partial active-data changes
- recover safely from interruption/crash during restore via staging + startup reconciliation
- resolve import conflicts (duplicate IDs and duplicate recording filenames) deterministically
- Keep backup encryption out of scope for v1 and disclose that exported archives are unencrypted.

## Impact
- **Affected specs**: `data-backup-restore` (new)
- **Affected code** (planned): `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/backup.rs`, `src-tauri/src/managers/history.rs`, `src-tauri/src/settings.rs`, `src/components/settings/history/HistorySettings.tsx` (or a dedicated backup settings component), `src/bindings.ts`
- **Behavioral impact**: users get a Codictate-branded portable archive backup, a safe restore path, deterministic compatibility behavior across app upgrades, and explicit handling for failure edge cases.

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
