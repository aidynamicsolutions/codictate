//! Backup/restore public API surface, shared types, and module wiring.
//!
//! The command handlers and other crate modules interact with this module path
//! (`crate::backup_restore::*`), while detailed implementation lives in focused
//! sibling modules.

use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::user_dictionary::{self, CustomWordEntry};
use chrono::{DateTime, Duration, Local, Utc};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration as StdDuration, SystemTime};
use tauri::{AppHandle, Emitter, Manager};
#[cfg(not(test))]
use tauri_plugin_store::StoreExt;
use tracing::{error, info, warn};
use zip::CompressionMethod;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const BACKUP_FORMAT_VERSION: &str = "1.0.0";
const BACKUP_FILE_EXTENSION: &str = "codictatebackup";
const BACKUP_PROGRESS_EVENT: &str = "backup-progress";
const HISTORY_FILE: &str = "history/history.jsonl";
const HISTORY_USER_STATS_FILE: &str = "history/user_stats.json";
const DICTIONARY_FILE: &str = "dictionary/dictionary.json";
const USER_STORE_FILE: &str = "user/user_store.json";
const MANIFEST_FILE: &str = "manifest.json";
const CHECKSUM_FILE: &str = "checksums.sha256";

const HISTORY_DB_FILE: &str = "history.db";
const RECORDINGS_DIR: &str = "recordings";
const USER_DICTIONARY_FILE: &str = "user_dictionary.json";
const USER_STORE_DB_FILE: &str = "user_store.json";

const BACKUP_RUNTIME_DIR: &str = "backup-restore";
const MARKER_FILE_NAME: &str = "restore-marker.json";
const UNDO_CHECKPOINT_META_FILE_NAME: &str = "undo-checkpoint.json";

const MAX_ARCHIVE_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const MAX_PAYLOAD_FILE_SIZE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_HISTORY_ROWS: u64 = 2_000_000;
const MAX_HISTORY_JSONL_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: u64 = 2_100_000;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const SAFETY_MARGIN_BYTES: u64 = 512 * 1024 * 1024;

const HISTORY_PAYLOAD_VERSION: u32 = 1;
const USER_STATS_PAYLOAD_VERSION: u32 = 1;
const DICTIONARY_PAYLOAD_VERSION: u32 = 1;
const USER_STORE_PAYLOAD_VERSION: u32 = 1;
const UNDO_RETENTION_DAYS: i64 = 7;
const WRITES_BLOCKED_MESSAGE: &str =
    "Backup or restore is running. Core write operations are temporarily unavailable.";

#[derive(Default)]
pub struct BackupRestoreRuntime {
    operation_in_progress: AtomicBool,
    maintenance_mode: AtomicBool,
    cancel_requested: AtomicBool,
    write_gate: Mutex<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum BackupScope {
    Complete,
    Smaller,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBackupRequest {
    pub scope: BackupScope,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct BackupCounts {
    pub history_entries: u64,
    pub recording_files: u64,
    pub dictionary_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBackupReport {
    pub output_path: String,
    pub counts: BackupCounts,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackupEstimateReport {
    pub complete_estimated_size_bytes: u64,
    pub smaller_estimated_size_bytes: u64,
    pub difference_bytes: u64,
    pub recording_files: u64,
    pub history_entries: u64,
    pub dictionary_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PreflightRestoreRequest {
    pub archive_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RestoreFinding {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct PreflightSummary {
    pub backup_format_version: String,
    pub created_at: String,
    pub created_with_app_version: String,
    pub platform: String,
    pub includes_recordings: bool,
    pub counts: BackupCounts,
    pub estimated_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightCompatibilityNoteCode {
    V1MacosGuaranteedCrossPlatformBestEffort,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PreflightRestoreReport {
    pub can_apply: bool,
    pub blocking_findings: Vec<RestoreFinding>,
    pub recoverable_findings: Vec<RestoreFinding>,
    pub summary: Option<PreflightSummary>,
    pub compatibility_note_code: PreflightCompatibilityNoteCode,
    pub compatibility_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ApplyRestoreRequest {
    pub archive_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ApplyRestoreReport {
    pub warnings: Vec<String>,
    pub counts: BackupCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct UndoLastRestoreRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UndoLastRestoreReport {
    pub restored: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UndoLastRestoreAvailabilityReport {
    pub available: bool,
    pub expires_at: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackupProgress {
    pub operation: String,
    pub phase: String,
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct BackupManifest {
    backup_format_version: String,
    created_at: String,
    created_with_app_version: String,
    platform: String,
    includes_recordings: bool,
    estimated_size_bytes: u64,
    counts: ManifestCounts,
    components: ManifestComponents,
    warnings: ManifestWarnings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct ManifestCounts {
    history_entries: u64,
    recording_files: u64,
    dictionary_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct ManifestComponents {
    history_payload_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_stats_payload_version: Option<u32>,
    dictionary_payload_version: u32,
    user_store_payload_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct ManifestWarnings {
    missing_recordings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct HistoryRowV1 {
    id: i64,
    file_name: String,
    timestamp: i64,
    saved: bool,
    title: String,
    transcription_text: String,
    post_processed_text: Option<String>,
    inserted_text: Option<String>,
    post_process_prompt: Option<String>,
    duration_ms: i64,
    speech_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct UserStatsPayloadV1 {
    version: u32,
    total_words: i64,
    total_duration_ms: i64,
    total_transcriptions: i64,
    first_transcription_date: Option<i64>,
    last_transcription_date: Option<i64>,
    transcription_dates: Vec<String>,
    total_filler_words_removed: i64,
    total_speech_duration_ms: i64,
    duration_stats_semantics_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct DictionaryPayload {
    version: u32,
    entries: Vec<CustomWordEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct RestoreMarker {
    state: String,
    snapshot_path: String,
    updated_at: String,
    snapshot_layout: Option<UndoCheckpointSnapshotLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct UndoCheckpointMeta {
    snapshot_path: String,
    created_at: String,
    expires_at: String,
    snapshot_layout: Option<UndoCheckpointSnapshotLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct UndoCheckpointSnapshotLayout {
    history_db: bool,
    user_dictionary: bool,
    user_store: bool,
    recordings_dir: bool,
}

#[derive(Debug, Clone)]
struct ArchiveEntryInventory {
    entries: HashMap<String, usize>,
    oversized_entries: BTreeSet<String>,
    total_uncompressed_bytes: u64,
}

#[derive(Debug, Clone)]
struct PreflightContext {
    report: PreflightRestoreReport,
}

mod backup;
mod fs_utils;
mod preflight;
mod restore;
mod runtime;
mod undo;

#[cfg(test)]
mod tests;

pub use backup::{create_backup, get_backup_estimate};
pub use preflight::preflight_restore;
pub use restore::{apply_restore, reconcile_startup};
#[cfg(test)]
pub use runtime::assert_writes_allowed;
pub use runtime::{
    can_start_transcription, ensure_transcription_start_allowed, request_cancel,
    with_write_permit,
};
pub use undo::{undo_last_restore, undo_last_restore_availability};

use fs_utils::*;
use preflight::*;
use restore::*;
use runtime::*;
use undo::*;
