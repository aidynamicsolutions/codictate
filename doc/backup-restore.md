# Backup & Restore

Feature documentation for user data backup and restore in Codictate.

## Overview

Users can export their local data (history, dictionary, settings, and optionally audio recordings) to a single portable `.codictatebackup` archive file, and restore from such archives. This enables data portability across uninstall/reinstall, machine migration, and disaster recovery.

### Backup Scopes
- **Full**: history + dictionary + settings + audio recordings
- **Lightweight**: history + dictionary + settings only (no audio files)

### Key Safety Features
- **Pre-restore safety backup**: current state is automatically saved before any restore overwrites data
- **Atomic staging + rollback**: restore writes to a staging area first; active data is unchanged on failure
- **Crash recovery**: restore-in-progress marker enables safe recovery on next app start

---

## Architecture Decision Records

### Why JSONL over raw SQLite copy?
Directly copying `history.db` is fragile because SQLite uses journaling/WAL side files that may be in an inconsistent state during a live database operation. Logical export (reading rows and serializing to JSONL) produces a clean, self-contained snapshot that is immune to filesystem-level database corruption. JSONL also enables forward migration: each line is an independent record that can be individually versioned and migrated.

### Why ZIP over tar.gz?
ZIP is the most universally supported archive format across all platforms. It supports random access to individual entries (important for preflight checksums without full extraction), is natively supported on macOS/Windows/Linux, and has built-in per-file compression. tar.gz requires sequential extraction and lacks per-file access. The `.codictatebackup` extension is used for branding while the container is standard ZIP.

### Why replace-only over merge in v1?
Merge-mode requires complex conflict resolution for every data type: duplicate history entries, conflicting dictionary definitions, diverged settings. Replace-only is safe, predictable, and simple to implement and test. The mandatory pre-restore safety backup mitigates the destructiveness of replace. Merge-mode can be added as a v2 enhancement once the replace-only foundation is proven stable.

### Why selective settings backup instead of full settings?
`AppSettings` contains ~40 fields, including sensitive API keys and device-specific values (microphone, installed models). Backing up API keys in an unencrypted archive is a security risk. Device-specific values (e.g., `selected_microphone`) are meaningless when restoring on a different machine. The selective approach exports only user-configurable preferences and uses forward-compatible defaults for fields added after the backup was created.

### Why recompute user_stats instead of importing the backup's snapshot?
`user_stats` contains cumulative counters (`total_words`, `total_transcriptions`, `transcription_dates`). In replace-only mode, the stats must be consistent with the restored history rows. Importing the backup's stats snapshot could produce inconsistencies if the backup was created on a system with different counting logic or if the stats were corrupted. Recomputing guarantees consistency.

---

## Error Catalog

| Error Code | Condition | User Message |
|---|---|---|
| `BACKUP_MUTEX_LOCKED` | Another backup/restore operation is in progress | "A backup operation is already running. Please wait for it to complete." |
| `EXPORT_DISK_FULL` | Insufficient disk space for temp workspace or output | "Not enough disk space to create backup. Need approximately {size}. Free up space and try again." |
| `EXPORT_DEST_FULL` | Insufficient space at export destination (e.g., USB drive) | "The selected destination does not have enough space ({available} available, {needed} needed)." |
| `RESTORE_DISK_FULL` | Insufficient disk space for unpack + staging + safety backup | "Not enough disk space to restore this backup. Need approximately {size}." |
| `ARCHIVE_CORRUPT` | ZIP cannot be opened or is malformed | "This backup file is corrupted or not a valid Codictate backup." |
| `CHECKSUM_MISMATCH` | SHA-256 checksum verification failed | "Backup integrity check failed. The backup file may be corrupted or modified." |
| `VERSION_TOO_OLD` | Backup format major version is below supported range | "This backup was created with an older version of Codictate. To restore it, install Codictate v{required} first, then re-export the backup." |
| `VERSION_TOO_NEW` | Backup format major version is above current | "This backup was created with a newer version of Codictate. Please update the app to restore this backup." |
| `UNSAFE_ARCHIVE` | Archive contains path traversal, absolute paths, or symlinks | "This backup file contains unsafe entries and cannot be restored." |
| `SAFETY_BACKUP_FAILED` | Pre-restore safety backup creation failed | "Could not create a safety backup of your current data. Restore has been cancelled to protect your data." |
| `RESTORE_ROLLBACK` | Restore failed mid-operation, rollback executed | "Restore failed during {step}. Your data has been restored to its previous state." |
| `RESTORE_CRASH_RECOVERY` | App restarted after interrupted restore | "A previous restore was interrupted. Your data has been safely recovered." |
| `MISSING_AUDIO_PLAYBACK` | Playback requested for a recording that doesn't exist | "Audio recording not available. This entry was restored without its audio file." |

---

## Archive Format Spec

### Container
Standard ZIP format with `.codictatebackup` extension.

### Required Files
| Path | Description |
|---|---|
| `manifest.json` | Format version, app version, platform, counts, warnings |
| `checksums.sha256` | SHA-256 hashes for all payload files |
| `history/history.jsonl` | One JSON object per line, one per history entry |
| `history/user_stats.json` | User statistics snapshot |
| `dictionary/dictionary.json` | Custom word entries with aliases |
| `settings/settings.json` | Selective app settings (user preferences only) |

### Optional
| Path | Description |
|---|---|
| `recordings/*.wav` | Audio recordings (only in full backup scope) |

### Compatibility Window
- Supported restore range: `current_major` and `current_major - 1`
- Current format version: `1.0.0`
- Patch/minor bumps within a major are always compatible
- Deprecation notice required at least one stable release before dropping a major version

### Migration Guidance for Unsupported Versions
- **Too old**: "Install Codictate v{old_version}, restore the backup there, then re-export with the current format."
- **Too new**: "Update Codictate to restore this backup."

---

## Cross-Platform Portability
- Archive paths use forward-slash (`/`) separators regardless of creation platform
- Filenames are normalized to NFC Unicode on restore
- Characters invalid on any platform (`<>:"/\|?*`) are rejected at export time
- Manifest records the source `platform` field for diagnostics
