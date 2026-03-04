# Change: Add User Data Backup and Restore

## Why
Users can lose local data (history text/audio, dictionary, onboarding/profile state) when reinstalling the app or moving to a new machine. Codictate needs a built-in export/restore flow that protects user data without manual file/database steps.

The v1 plan should stay practical, but it also needs a few guardrails to avoid future breakage: compatibility expectations across v1 upgrades, write isolation during backup/restore, and metadata contracts that won’t require format-breaking changes later.

## What Changes
- Add user-driven backup export to a single `.codictatebackup` ZIP archive selected with native save dialog.
- Add user-driven restore from a selected `.codictatebackup` file using native open dialog.
- Keep v1 restore mode replace-only (no merge behavior).
- Define v1 archive layout:
  - `manifest.json`
  - `checksums.sha256`
  - `history/history.jsonl`
  - `dictionary/dictionary.json`
  - `user/user_store.json` (onboarding/profile/growth state)
  - optional `recordings/` for full backups
- Include `estimated_size_bytes` and `platform` in `manifest.json` so archive metadata remains stable as product/platform scope expands.
- Export history as JSONL logical rows (never raw SQLite file copy).
- Place Backup/Restore in `Settings > General` after the `Advanced` section (not on History screen).
- Harmonize Backup/Restore visual design with existing settings subsection patterns (section heading + divided row layout, balanced neutral action emphasis).
- Keep one primary `Create backup` action that opens a configuration modal.
- In the create modal, keep core data categories always included and expose one editable choice:
  - `Include recordings` ON => complete backup (includes recordings when available)
  - `Include recordings` OFF => smaller backup (text/profile data, no recordings)
- Show live estimated-size delta in the create modal when toggling recordings inclusion.
- Remember the last user-selected backup folder and use it as the next start location for both create-backup save dialog and restore-backup open dialog; use OS default location on first use.
- Add integrity checks for v1:
  - SHA-256 checksums for payload files
  - preflight rejects restore if any payload file is missing from `checksums.sha256`
  - restore preflight before any active-data writes
  - archive entry safety rejection (traversal, absolute paths, windows drive-prefixed paths, control-character paths, symlink/hardlink, duplicate normalized paths, and case-collision paths)
  - practical hard size/count bounds for restore safety (archive size, payload size, row count, entry count, and total uncompressed bytes)
  - required free-disk check before restore staging/swap begins
- Harden backup/restore command surface with least-privilege Tauri scope:
  - backup/restore commands are invokable from the main window only
  - v1 capability is constrained to macOS target platform
  - capabilities are explicitly listed in `tauri.conf.json` (no implicit capability enablement)
  - backup/restore file access is restricted to app-managed data paths and user-selected archive paths
- Add simple operation lock and app-unavailable mode:
  - backup/restore runs as an exclusive maintenance operation
  - app blocks new transcription/history/dictionary/profile writes for full backup/restore duration
  - create-backup modal informs users they cannot use core app actions until operation completes
- Add bounded-memory streaming I/O requirements for large-user datasets:
  - stream history export/import row-by-row
  - stream ZIP read/write and recording copy in chunks
- Add pre-swap rollback snapshot and staged restore swap:
  - stage restore data first
  - preserve active dataset snapshot before destructive swap
  - rollback automatically on failure
- Add one-click restore safety net for non-technical users:
  - retain one post-restore auto-checkpoint from the pre-swap snapshot for a short window
  - provide a simple `Undo Last Restore` action while checkpoint is available
  - expire/prune the checkpoint automatically after the retention window
- Add commit-aware startup reconciliation marker:
  - `in_progress` before swap
  - `committed` after successful swap but before final cleanup
  - startup rollback only when marker indicates uncommitted restore
  - marker and swap boundaries use durable file/dir sync so crash recovery decisions are based on persisted state
- Keep app preferences out of v1 backup/restore:
  - backups do not include settings payload
  - restore keeps current install settings unchanged
  - users reconfigure preferences after restore if needed
- Keep `user_store` restore resilient for non-technical users:
  - backups created by Codictate include `user/user_store.json`
  - if backup `user/user_store.json` is missing or malformed, preserve current local `user_store` state when available
  - only initialize defaults when no local `user_store` exists
- Keep restore stats simple in v1: always recompute stats from restored history rows after import (no stats snapshot import path).
- Add minimal compatibility contract for v1 upgrades:
  - v1-format backups remain restorable across v1 app upgrades
  - restore parsers accept unknown additive fields and default missing optional fields
  - breaking payload changes require explicit version bump + migration path before release
- Guarantee restore compatibility for macOS backups in v1; cross-platform restore remains best-effort in v1.
- Add progress events and concise non-technical user summaries for export/restore.
- Add restore preflight summary UX before destructive confirmation (backup date/version/platform/scope/counts).
- Keep backups unencrypted in v1 and disclose this contextually after successful backup export and in docs.
- Add i18n keys under `settings.backup` for all user-facing strings.

## Deferred to V1.1+
- Multi-phase restore marker state machine beyond `in_progress`/`committed`.
- Broad migration framework for many historical payload versions.
- Cross-platform filename remap/sanitization framework.
- Formal resource-limit framework with compression-ratio controls.
- Expanded diagnostics taxonomy and power-user report model.
- Background backup/restore while core app actions remain available.

## Impact
- **Affected specs**: `data-backup-restore` (new)
- **Affected code** (planned):
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/commands/backup.rs`
  - `src-tauri/src/managers/history.rs`
  - `src-tauri/src/user_dictionary.rs`
  - `src-tauri/src/user_profile.rs`
  - `src-tauri/src/growth.rs`
  - `src/components/settings/general/GeneralSettings.tsx`
  - `src/components/settings/history/HistorySettings.tsx`
  - `src/components/settings/backup/BackupRestoreCard.tsx`
  - `src/bindings.ts`
  - `src-tauri/capabilities/*.json`
  - `src/i18n/locales/*/translation.json`
- **Documentation**: add concise `doc/backup-restore.md` and small release-note entry in `doc/prodRelease.md`
