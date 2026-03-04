use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use tracing::{debug, error, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

use crate::audio_toolkit::save_wav_file;

/// Database migrations for transcription history.
/// Each migration is applied in order. The library tracks which migrations
/// have been applied using SQLite's user_version pragma.
///
/// Note: For users upgrading from tauri-plugin-sql, migrate_from_tauri_plugin_sql()
/// converts the old _sqlx_migrations table tracking to the user_version pragma,
/// ensuring migrations don't re-run on existing databases.
static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL
        );",
    ),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_processed_text TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_prompt TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN duration_ms INTEGER DEFAULT 0;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN inserted_text TEXT;"),
    M::up(
        "CREATE TABLE IF NOT EXISTS user_stats (
            id INTEGER PRIMARY KEY DEFAULT 1,
            total_words INTEGER DEFAULT 0,
            total_duration_ms INTEGER DEFAULT 0,
            total_transcriptions INTEGER DEFAULT 0,
            first_transcription_date INTEGER,
            last_transcription_date INTEGER,
            transcription_dates TEXT DEFAULT '[]'
        );",
    ),
    // Migration 6: Add filler word tracking for smart tile stats
    M::up("ALTER TABLE user_stats ADD COLUMN total_filler_words_removed INTEGER DEFAULT 0;"),
    // Migration 7: Persist VAD-retained speech duration per history entry
    M::up("ALTER TABLE transcription_history ADD COLUMN speech_duration_ms INTEGER DEFAULT 0;"),
    // Migration 8: Track total speech duration for WPM (distinct from recording elapsed duration)
    M::up("ALTER TABLE user_stats ADD COLUMN total_speech_duration_ms INTEGER DEFAULT 0;"),
    // Migration 9: Mark completion of duration semantic backfill
    M::up("ALTER TABLE user_stats ADD COLUMN duration_stats_semantics_version INTEGER DEFAULT 0;"),
    // Migration 10: Keep append-only snapshots for stats migration safety
    M::up(
        "CREATE TABLE IF NOT EXISTS user_stats_migration_backup (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            backup_path TEXT,
            payload_json TEXT NOT NULL
        );",
    ),
];

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    pub post_processed_text: Option<String>,
    pub inserted_text: Option<String>,
    pub effective_text: String,
    pub raw_text: String,
    pub post_process_prompt: Option<String>,
    pub duration_ms: i64,
    pub file_path: String,
    pub audio_file_exists: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryStats {
    pub total_size_bytes: u64,
    pub total_entries: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HomeStats {
    pub total_words: i64,
    pub total_duration_minutes: f64,
    pub wpm: f64,
    pub time_saved_minutes: f64,
    pub streak_days: i64,
    pub faster_than_typing_percentage: f64,
    pub total_filler_words_removed: i64,
    pub filler_filter_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct StatsContribution {
    pub word_count: i64,
    pub recording_duration_ms: i64,
    pub speech_duration_ms: i64,
    pub filler_words_removed: i64,
    pub date_added_to_streak_list: bool,
    pub date_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserStatsBackupSnapshot {
    created_at_epoch_ms: i64,
    duration_stats_semantics_version_before: i64,
    total_words: i64,
    total_duration_ms: i64,
    total_speech_duration_ms: i64,
    total_transcriptions: i64,
    first_transcription_date: Option<i64>,
    last_transcription_date: Option<i64>,
    transcription_dates: String,
    total_filler_words_removed: i64,
}

impl UserStatsBackupSnapshot {
    fn matches_content(&self, other: &Self) -> bool {
        self.duration_stats_semantics_version_before
            == other.duration_stats_semantics_version_before
            && self.total_words == other.total_words
            && self.total_duration_ms == other.total_duration_ms
            && self.total_speech_duration_ms == other.total_speech_duration_ms
            && self.total_transcriptions == other.total_transcriptions
            && self.first_transcription_date == other.first_transcription_date
            && self.last_transcription_date == other.last_transcription_date
            && self.transcription_dates == other.transcription_dates
            && self.total_filler_words_removed == other.total_filler_words_removed
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct SavedTranscription {
    pub entry_id: i64,
    pub contribution: StatsContribution,
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
}

const DURATION_SEMANTICS_VERSION_V1: i64 = 1;

fn compute_effective_text(
    inserted_text: Option<&str>,
    post_processed_text: Option<&str>,
    transcription_text: &str,
) -> String {
    inserted_text
        .or(post_processed_text)
        .unwrap_or(transcription_text)
        .to_string()
}

fn compute_duration_metrics(
    total_words: i64,
    total_duration_ms: i64,
    total_speech_duration_ms: i64,
    duration_stats_semantics_version: i64,
) -> (f64, f64, f64, f64) {
    let total_duration_minutes = total_duration_ms as f64 / 60000.0;
    let speech_duration_minutes = if duration_stats_semantics_version >= DURATION_SEMANTICS_VERSION_V1
    {
        total_speech_duration_ms as f64 / 60000.0
    } else {
        // Backfill incomplete: preserve legacy semantics until marker flips.
        total_duration_minutes
    };

    let wpm = if speech_duration_minutes > 0.001 {
        total_words as f64 / speech_duration_minutes
    } else {
        0.0
    };

    let time_to_type_minutes = total_words as f64 / 40.0;
    let time_saved_minutes = time_to_type_minutes - total_duration_minutes;

    (
        total_duration_minutes,
        speech_duration_minutes,
        wpm,
        time_saved_minutes,
    )
}

fn normalize_runtime_durations(recording_duration_ms: i64, speech_duration_ms: i64) -> (i64, i64) {
    let mut normalized_recording = recording_duration_ms.max(0);
    let mut normalized_speech = speech_duration_ms.max(0);

    // If elapsed recording time is unavailable but speech time exists, preserve useful duration.
    if normalized_recording == 0 && normalized_speech > 0 {
        normalized_recording = normalized_speech;
    }

    if normalized_recording > 0 {
        normalized_speech = normalized_speech.min(normalized_recording);
    } else {
        normalized_speech = 0;
    }

    (normalized_recording, normalized_speech)
}

fn map_history_entry(
    row: &rusqlite::Row<'_>,
    recordings_dir: &PathBuf,
) -> rusqlite::Result<HistoryEntry> {
    let file_name: String = row.get("file_name")?;
    let file_path = recordings_dir.join(&file_name);
    let transcription_text: String = row.get("transcription_text")?;
    let post_processed_text: Option<String> = row.get("post_processed_text")?;
    let inserted_text: Option<String> = row.get("inserted_text")?;
    let effective_text = compute_effective_text(
        inserted_text.as_deref(),
        post_processed_text.as_deref(),
        &transcription_text,
    );

    Ok(HistoryEntry {
        id: row.get("id")?,
        file_name,
        timestamp: row.get("timestamp")?,
        saved: row.get("saved")?,
        title: row.get("title")?,
        raw_text: transcription_text.clone(),
        transcription_text,
        post_processed_text,
        inserted_text,
        effective_text,
        post_process_prompt: row.get("post_process_prompt")?,
        duration_ms: row.get("duration_ms")?,
        file_path: file_path.to_string_lossy().to_string(),
        audio_file_exists: file_path.exists(),
    })
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = app_handle.path().app_data_dir()?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
        };

        // Initialize database and run migrations synchronously
        manager.init_database()?;

        Ok(manager)
    }

    fn init_database(&self) -> Result<()> {
        info!("Initializing database at {:?}", self.db_path);

        let mut conn = Connection::open(&self.db_path)?;

        // Handle migration from tauri-plugin-sql to rusqlite_migration
        // tauri-plugin-sql used _sqlx_migrations table, rusqlite_migration uses user_version pragma
        self.migrate_from_tauri_plugin_sql(&conn)?;

        // Some upgraded databases can have schema columns already present while user_version
        // is behind (for example from legacy/manual migration paths). Reconcile this first to
        // avoid duplicate-column migration failures without touching existing data.
        Self::reconcile_legacy_schema_state(&conn)?;

        // Create migrations object and run to latest version
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        // Validate migrations in debug builds
        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid migrations");

        // Get current version before migration
        let version_before: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        debug!("Database version before migration: {}", version_before);

        // Apply any pending migrations
        migrations.to_latest(&mut conn)?;

        // Get version after migration
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version_after > version_before {
            info!(
                "Database migrated from version {} to {}",
                version_before, version_after
            );
        } else {
            debug!("Database already at latest version {}", version_after);
        }

        // Initialize user stats if needed
        self.initialize_user_stats(&conn)?;
        self.ensure_duration_stats_semantics(&mut conn)?;

        Ok(())
    }

    fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
        Ok(conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                params![table_name],
                |row| row.get(0),
            )
            .unwrap_or(false))
    }

    /// Check whether a column exists in a given table.
    ///
    /// # Safety
    /// `table_name` MUST be a hardcoded literal — PRAGMA does not support parameterized identifiers.
    fn column_exists(conn: &Connection, table_name: &str, column_name: &str) -> Result<bool> {
        let pragma = format!("PRAGMA table_info({})", table_name);
        let mut stmt = conn.prepare(&pragma)?;
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let existing_column_name: String = row.get(1)?;
            if existing_column_name == column_name {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Idempotently add a column to a table if it does not already exist.
    ///
    /// # Safety
    /// All arguments MUST be hardcoded literals — `ALTER TABLE` does not support
    /// parameterized identifiers. Never pass user-supplied input.
    fn ensure_column_exists(
        conn: &Connection,
        table_name: &str,
        column_name: &str,
        column_definition: &str,
    ) -> Result<bool> {
        if Self::column_exists(conn, table_name, column_name)? {
            return Ok(false);
        }

        let alter = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table_name, column_name, column_definition
        );
        conn.execute(&alter, [])?;
        info!(
            "Reconciled legacy schema: added missing column '{}.{}'",
            table_name, column_name
        );
        Ok(true)
    }

    fn reconcile_legacy_schema_state(conn: &Connection) -> Result<()> {
        let current_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if current_version == 0 {
            // Fresh databases should follow normal migration flow.
            return Ok(());
        }

        let mut schema_changed = false;

        if !Self::table_exists(conn, "transcription_history")? {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS transcription_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_name TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    saved BOOLEAN NOT NULL DEFAULT 0,
                    title TEXT NOT NULL,
                    transcription_text TEXT NOT NULL
                );",
            )?;
            schema_changed = true;
            info!("Reconciled legacy schema: created missing table 'transcription_history'");
        }

        schema_changed |= Self::ensure_column_exists(
            conn,
            "transcription_history",
            "post_processed_text",
            "TEXT",
        )?;
        schema_changed |= Self::ensure_column_exists(
            conn,
            "transcription_history",
            "post_process_prompt",
            "TEXT",
        )?;
        schema_changed |= Self::ensure_column_exists(
            conn,
            "transcription_history",
            "duration_ms",
            "INTEGER DEFAULT 0",
        )?;
        schema_changed |= Self::ensure_column_exists(
            conn,
            "transcription_history",
            "inserted_text",
            "TEXT",
        )?;
        schema_changed |= Self::ensure_column_exists(
            conn,
            "transcription_history",
            "speech_duration_ms",
            "INTEGER DEFAULT 0",
        )?;

        if !Self::table_exists(conn, "user_stats")? {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS user_stats (
                    id INTEGER PRIMARY KEY DEFAULT 1,
                    total_words INTEGER DEFAULT 0,
                    total_duration_ms INTEGER DEFAULT 0,
                    total_transcriptions INTEGER DEFAULT 0,
                    first_transcription_date INTEGER,
                    last_transcription_date INTEGER,
                    transcription_dates TEXT DEFAULT '[]'
                );",
            )?;
            schema_changed = true;
            info!("Reconciled legacy schema: created missing table 'user_stats'");
        }

        schema_changed |= Self::ensure_column_exists(
            conn,
            "user_stats",
            "total_filler_words_removed",
            "INTEGER DEFAULT 0",
        )?;
        schema_changed |= Self::ensure_column_exists(
            conn,
            "user_stats",
            "total_speech_duration_ms",
            "INTEGER DEFAULT 0",
        )?;
        schema_changed |= Self::ensure_column_exists(
            conn,
            "user_stats",
            "duration_stats_semantics_version",
            "INTEGER DEFAULT 0",
        )?;

        if !Self::table_exists(conn, "user_stats_migration_backup")? {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS user_stats_migration_backup (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at INTEGER NOT NULL,
                    backup_path TEXT,
                    payload_json TEXT NOT NULL
                );",
            )?;
            schema_changed = true;
            info!("Reconciled legacy schema: created missing table 'user_stats_migration_backup'");
        }

        let history_complete = Self::table_exists(conn, "transcription_history")?
            && Self::column_exists(conn, "transcription_history", "post_processed_text")?
            && Self::column_exists(conn, "transcription_history", "post_process_prompt")?
            && Self::column_exists(conn, "transcription_history", "duration_ms")?
            && Self::column_exists(conn, "transcription_history", "inserted_text")?
            && Self::column_exists(conn, "transcription_history", "speech_duration_ms")?;
        let stats_complete = Self::table_exists(conn, "user_stats")?
            && Self::column_exists(conn, "user_stats", "total_filler_words_removed")?
            && Self::column_exists(conn, "user_stats", "total_speech_duration_ms")?
            && Self::column_exists(conn, "user_stats", "duration_stats_semantics_version")?
            && Self::table_exists(conn, "user_stats_migration_backup")?;

        let target_version = MIGRATIONS.len() as i32;
        if history_complete && stats_complete && current_version < target_version {
            conn.pragma_update(None, "user_version", target_version)?;
            info!(
                "Reconciled migration state from version {} to {} (schema already satisfied)",
                current_version, target_version
            );
        } else if schema_changed {
            debug!(
                "Legacy schema reconciliation changed tables/columns but migration version remained {}",
                current_version
            );
        }

        Ok(())
    }

    fn initialize_user_stats(&self, conn: &Connection) -> Result<()> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM user_stats",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if count == 0 {
            conn.execute(
                "INSERT INTO user_stats (id, total_words, total_duration_ms, total_transcriptions, transcription_dates) VALUES (1, 0, 0, 0, '[]')",
                [],
            )?;
            debug!("Initialized empty user stats");
        }

        Ok(())
    }

    fn legacy_effective_duration_ms(duration_ms: i64) -> i64 {
        if duration_ms <= 0 {
            return 0;
        }

        let vad_overhead_ms = 900;
        if duration_ms > vad_overhead_ms {
            duration_ms - vad_overhead_ms
        } else {
            std::cmp::max(duration_ms / 2, 100)
        }
    }

    fn snapshot_user_stats(
        conn: &Connection,
        duration_stats_semantics_version_before: i64,
    ) -> Result<UserStatsBackupSnapshot> {
        let snapshot = conn.query_row(
            "SELECT
                COALESCE(total_words, 0),
                COALESCE(total_duration_ms, 0),
                COALESCE(total_speech_duration_ms, 0),
                COALESCE(total_transcriptions, 0),
                first_transcription_date,
                last_transcription_date,
                COALESCE(transcription_dates, '[]'),
                COALESCE(total_filler_words_removed, 0)
             FROM user_stats WHERE id = 1",
            [],
            |row| {
                Ok(UserStatsBackupSnapshot {
                    created_at_epoch_ms: Utc::now().timestamp_millis(),
                    duration_stats_semantics_version_before,
                    total_words: row.get(0).unwrap_or(0),
                    total_duration_ms: row.get(1).unwrap_or(0),
                    total_speech_duration_ms: row.get(2).unwrap_or(0),
                    total_transcriptions: row.get(3).unwrap_or(0),
                    first_transcription_date: row.get(4).unwrap_or(None),
                    last_transcription_date: row.get(5).unwrap_or(None),
                    transcription_dates: row.get(6).unwrap_or_else(|_| "[]".to_string()),
                    total_filler_words_removed: row.get(7).unwrap_or(0),
                })
            },
        )?;
        Ok(snapshot)
    }

    fn write_user_stats_backup_snapshot(
        snapshot: &UserStatsBackupSnapshot,
        backup_root: &Path,
    ) -> Result<PathBuf> {
        let payload_json = serde_json::to_string_pretty(snapshot)?;
        fs::create_dir_all(&backup_root)?;

        let file_name = format!(
            "user_stats-pre-duration-v1-{}.json",
            snapshot.created_at_epoch_ms
        );
        let backup_path = backup_root.join(file_name);
        fs::write(&backup_path, payload_json)?;
        Ok(backup_path)
    }

    fn find_existing_backup_row_for_snapshot(
        conn: &Connection,
        snapshot: &UserStatsBackupSnapshot,
    ) -> Result<Option<(i64, Option<String>)>> {
        let mut stmt = conn.prepare(
            "SELECT id, backup_path, payload_json FROM user_stats_migration_backup ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (id, backup_path, payload_json) = row?;
            let parsed_snapshot: UserStatsBackupSnapshot =
                match serde_json::from_str(&payload_json) {
                    Ok(parsed) => parsed,
                    Err(_) => continue,
                };

            if parsed_snapshot.matches_content(snapshot) {
                return Ok(Some((id, backup_path)));
            }
        }

        Ok(None)
    }

    fn ensure_duration_stats_semantics(&self, conn: &mut Connection) -> Result<()> {
        let backup_root = self.app_handle.path().app_data_dir()?.join("stats-backups");
        Self::ensure_duration_stats_semantics_with_backup_root(conn, backup_root.as_path())
    }

    fn ensure_duration_stats_semantics_with_backup_root(
        conn: &mut Connection,
        backup_root: &Path,
    ) -> Result<()> {
        let semantics_version: i64 = conn
            .query_row(
                "SELECT COALESCE(duration_stats_semantics_version, 0) FROM user_stats WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if semantics_version >= DURATION_SEMANTICS_VERSION_V1 {
            return Ok(());
        }

        let snapshot = Self::snapshot_user_stats(conn, semantics_version)?;
        let (backup_path, backup_row_id) = if let Some((existing_row_id, existing_backup_path)) =
            Self::find_existing_backup_row_for_snapshot(conn, &snapshot)?
        {
            let mut resolved_path = existing_backup_path
                .map(PathBuf::from)
                .unwrap_or_else(|| backup_root.join("missing-backup-path.json"));
            if !resolved_path.exists() {
                resolved_path = Self::write_user_stats_backup_snapshot(&snapshot, backup_root)?;
                conn.execute(
                    "UPDATE user_stats_migration_backup SET backup_path = ?1 WHERE id = ?2",
                    params![resolved_path.to_string_lossy().to_string(), existing_row_id],
                )?;
            }
            (resolved_path, existing_row_id)
        } else {
            let backup_path = Self::write_user_stats_backup_snapshot(&snapshot, backup_root)?;
            let payload_json = serde_json::to_string_pretty(&snapshot)?;
            conn.execute(
                "INSERT INTO user_stats_migration_backup (created_at, backup_path, payload_json)
                 VALUES (?1, ?2, ?3)",
                params![
                    Utc::now().timestamp(),
                    backup_path.to_string_lossy().to_string(),
                    payload_json
                ],
            )?;
            (backup_path, conn.last_insert_rowid())
        };

        let tx = conn.transaction()?;

        let old_total_duration_ms: i64 = tx.query_row(
            "SELECT COALESCE(total_duration_ms, 0) FROM user_stats WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        let mut live_recording_sum_ms = 0_i64;
        let mut live_speech_sum_ms = 0_i64;
        let mut live_legacy_heuristic_sum_ms = 0_i64;
        let mut malformed_speech_duration_rows = 0_i64;

        {
            let mut stmt = tx.prepare(
                "SELECT COALESCE(duration_ms, 0), COALESCE(speech_duration_ms, 0) FROM transcription_history",
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let recording_duration_ms: i64 = row.get(0).unwrap_or(0);
                let stored_speech_duration_ms: i64 = row.get(1).unwrap_or(0);
                let normalized_recording = recording_duration_ms.max(0);
                let normalized_speech = if stored_speech_duration_ms > 0 {
                    if stored_speech_duration_ms > normalized_recording {
                        malformed_speech_duration_rows =
                            malformed_speech_duration_rows.saturating_add(1);
                    }
                    stored_speech_duration_ms
                        .max(0)
                        .min(normalized_recording)
                } else {
                    normalized_recording
                };

                live_recording_sum_ms = live_recording_sum_ms.saturating_add(normalized_recording);
                live_speech_sum_ms = live_speech_sum_ms.saturating_add(normalized_speech);
                live_legacy_heuristic_sum_ms = live_legacy_heuristic_sum_ms
                    .saturating_add(Self::legacy_effective_duration_ms(normalized_recording));
            }
        }

        let legacy_residual_ms = old_total_duration_ms
            .saturating_sub(live_legacy_heuristic_sum_ms)
            .max(0);
        let recomputed_recording_total = live_recording_sum_ms.saturating_add(legacy_residual_ms);
        let recomputed_speech_total = live_speech_sum_ms.saturating_add(legacy_residual_ms);

        tx.execute(
            "UPDATE user_stats SET
                total_duration_ms = ?1,
                total_speech_duration_ms = ?2,
                duration_stats_semantics_version = ?3
             WHERE id = 1",
            params![
                recomputed_recording_total,
                recomputed_speech_total,
                DURATION_SEMANTICS_VERSION_V1
            ],
        )?;

        tx.commit()?;

        info!(
            backup_path = %backup_path.to_string_lossy(),
            backup_row_id = backup_row_id,
            old_total_duration_ms = old_total_duration_ms,
            live_recording_sum_ms = live_recording_sum_ms,
            live_speech_sum_ms = live_speech_sum_ms,
            live_legacy_heuristic_sum_ms = live_legacy_heuristic_sum_ms,
            malformed_speech_duration_rows = malformed_speech_duration_rows,
            legacy_residual_ms = legacy_residual_ms,
            recomputed_recording_total = recomputed_recording_total,
            recomputed_speech_total = recomputed_speech_total,
            "Backfilled duration semantics and stored pre-migration stats backup"
        );

        Ok(())
    }

    /// Migrate from tauri-plugin-sql's migration tracking to rusqlite_migration's.
    /// tauri-plugin-sql used a _sqlx_migrations table, while rusqlite_migration uses
    /// SQLite's user_version pragma. This function checks if the old system was in use
    /// and sets the user_version accordingly so migrations don't re-run.
    fn migrate_from_tauri_plugin_sql(&self, conn: &Connection) -> Result<()> {
        // Check if the old _sqlx_migrations table exists
        let has_sqlx_migrations: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_sqlx_migrations {
            return Ok(());
        }

        // Check current user_version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if current_version > 0 {
            // Already migrated to rusqlite_migration system
            return Ok(());
        }

        // Get the highest version from the old migrations table
        let old_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if old_version > 0 {
            info!(
                "Migrating from tauri-plugin-sql (version {}) to rusqlite_migration",
                old_version
            );

            // Set user_version to match the old migration state
            conn.pragma_update(None, "user_version", old_version)?;

            // Optionally drop the old migrations table (keeping it doesn't hurt)
            // conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", [])?;

            info!(
                "Migration tracking converted: user_version set to {}",
                old_version
            );
        }

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn with_write_permit<T>(&self, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        crate::backup_restore::with_write_permit(&self.app_handle, || {
            operation().map_err(|error| error.to_string())
        })
        .map_err(anyhow::Error::msg)
    }

    fn update_inserted_text_by_id_with_conn(
        conn: &Connection,
        id: i64,
        inserted_text: String,
    ) -> Result<()> {
        conn.execute(
            "UPDATE transcription_history SET inserted_text = ?1 WHERE id = ?2",
            params![inserted_text, id],
        )?;

        Ok(())
    }

    /// Update the refine output for a specific history entry.
    ///
    /// Does NOT modify `inserted_text` — per the transcript-insertion spec,
    /// `inserted_text` is only updated on paste success so that `effective_text`
    /// continues to reflect the last text actually pasted into the target app.
    fn update_refine_output_by_id_with_conn(
        conn: &Connection,
        id: i64,
        post_processed_text: String,
        post_process_prompt: Option<String>,
    ) -> Result<()> {
        conn.execute(
            "UPDATE transcription_history SET post_processed_text = ?1, post_process_prompt = ?2 WHERE id = ?3",
            params![post_processed_text, post_process_prompt, id],
        )?;

        Ok(())
    }

    pub fn update_inserted_text_by_id(&self, id: i64, inserted_text: String) -> Result<()> {
        self.with_write_permit(|| {
            let conn = self.get_connection()?;
            Self::update_inserted_text_by_id_with_conn(&conn, id, inserted_text)?;

            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!(
                    "Failed to emit history-updated event after inserted_text update: {}",
                    e
                );
            }

            Ok(())
        })
    }

    pub fn update_refine_output_by_id(
        &self,
        id: i64,
        post_processed_text: String,
        post_process_prompt: Option<String>,
    ) -> Result<()> {
        self.with_write_permit(|| {
            let conn = self.get_connection()?;
            Self::update_refine_output_by_id_with_conn(
                &conn,
                id,
                post_processed_text,
                post_process_prompt,
            )?;

            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!(
                    "Failed to emit history-updated event after refine output update: {}",
                    e
                );
            }

            Ok(())
        })
    }

    /// Save a transcription to history (both database and WAV file)
    pub fn save_transcription(
        &self,
        audio_samples: Vec<f32>,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
        recording_duration_ms: i64,
        speech_duration_ms: i64,
        filler_words_removed: i64,
    ) -> Result<SavedTranscription> {
        self.with_write_permit(|| {
            let timestamp = Utc::now().timestamp();
            let file_name = format!("codictate-{}.wav", timestamp);
            let title = self.format_timestamp_title(timestamp);

            // Save WAV file
            let file_path = self.recordings_dir.join(&file_name);
            save_wav_file(file_path, &audio_samples)?;

            // Save to database
            let contribution = self.save_to_database(
                file_name,
                timestamp,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                recording_duration_ms,
                speech_duration_ms,
                filler_words_removed,
            )?;

            // Clean up old entries
            self.cleanup_old_entries_impl()?;

            // Emit history updated event
            info!("Emitting history-updated event");
            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!("Failed to emit history-updated event: {}", e);
            } else {
                debug!("Successfully emitted history-updated event");
            }

            Ok(contribution)
        })
    }

    fn save_to_database(
        &self,
        file_name: String,
        timestamp: i64,
        title: String,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
        recording_duration_ms: i64,
        speech_duration_ms: i64,
        filler_words_removed: i64,
    ) -> Result<SavedTranscription> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;
        let (normalized_recording_duration_ms, normalized_speech_duration_ms) =
            normalize_runtime_durations(recording_duration_ms, speech_duration_ms);

        if normalized_recording_duration_ms != recording_duration_ms
            || normalized_speech_duration_ms != speech_duration_ms
        {
            debug!(
                input_recording_duration_ms = recording_duration_ms,
                input_speech_duration_ms = speech_duration_ms,
                normalized_recording_duration_ms = normalized_recording_duration_ms,
                normalized_speech_duration_ms = normalized_speech_duration_ms,
                "Normalized transcription duration pair before persisting stats"
            );
        }

        // 1. Insert into transcription_history
        tx.execute(
            "INSERT INTO transcription_history (file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, duration_ms, speech_duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                file_name,
                timestamp,
                false,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                normalized_recording_duration_ms,
                normalized_speech_duration_ms
            ],
        )?;
        let entry_id = tx.last_insert_rowid();

        // 2. Update user_stats
        // Calculate word count
        let text_content = post_processed_text.clone().unwrap_or(transcription_text.clone());
        let word_count = if text_content.trim().is_empty() {
            0
        } else {
            crate::audio_toolkit::text::count_words(&text_content) as i64
        };

        // Get date string for today
        let today_str = if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
            dt.with_timezone(&Local).format("%Y-%m-%d").to_string()
        } else {
            // Fallback (unlikely)
            "1970-01-01".to_string()
        };

        // Fetch current stats to update dates array
        let current_dates_json: String = tx.query_row(
            "SELECT transcription_dates FROM user_stats WHERE id = 1",
            [],
            |row| row.get(0),
        ).unwrap_or_else(|_| "[]".to_string());

        let mut dates: Vec<String> = serde_json::from_str(&current_dates_json).unwrap_or_default();
        let mut date_added_to_streak_list = false;
        if !dates.contains(&today_str) {
            dates.push(today_str);
            date_added_to_streak_list = true;
        }
        let new_dates_json = serde_json::to_string(&dates).unwrap_or_else(|_| "[]".to_string());

        // Upsert stats (assuming row 1 exists due to backfill/init)
        tx.execute(
            "UPDATE user_stats SET 
                total_words = total_words + ?1,
                total_duration_ms = total_duration_ms + ?2,
                total_speech_duration_ms = total_speech_duration_ms + ?3,
                total_transcriptions = total_transcriptions + 1,
                last_transcription_date = ?4,
                transcription_dates = ?5,
                first_transcription_date = COALESCE(first_transcription_date, ?4),
                total_filler_words_removed = total_filler_words_removed + ?6
             WHERE id = 1",
            params![
                word_count,
                normalized_recording_duration_ms,
                normalized_speech_duration_ms,
                timestamp,
                new_dates_json,
                filler_words_removed
            ],
        )?;

        tx.commit()?;

        debug!("Saved transcription to database and updated stats (filler_words_removed: {})", filler_words_removed);
        Ok(SavedTranscription {
            entry_id,
            contribution: StatsContribution {
                word_count,
                recording_duration_ms: normalized_recording_duration_ms,
                speech_duration_ms: normalized_speech_duration_ms,
                filler_words_removed,
                date_added_to_streak_list,
                date_key: if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                    dt.with_timezone(&Local).format("%Y-%m-%d").to_string()
                } else {
                    "1970-01-01".to_string()
                },
            },
        })
    }

    pub fn rollback_stats_contribution(&self, contribution: &StatsContribution) -> Result<()> {
        self.with_write_permit(|| {
            let mut conn = self.get_connection()?;
            Self::rollback_stats_contribution_with_conn(&mut conn, contribution)?;

            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!("Failed to emit history-updated event after rollback: {}", e);
            }

            Ok(())
        })
    }

    fn rollback_stats_contribution_with_conn(
        conn: &mut Connection,
        contribution: &StatsContribution,
    ) -> Result<()> {
        let tx = conn.transaction()?;

        let (
            total_words,
            total_duration_ms,
            total_speech_duration_ms,
            total_transcriptions,
            transcription_dates_json,
            total_filler_words_removed
        ): (i64, i64, i64, i64, String, i64) = tx.query_row(
            "SELECT total_words, total_duration_ms, COALESCE(total_speech_duration_ms, 0), total_transcriptions, transcription_dates, COALESCE(total_filler_words_removed, 0) FROM user_stats WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get(0).unwrap_or(0),
                    row.get(1).unwrap_or(0),
                    row.get(2).unwrap_or(0),
                    row.get(3).unwrap_or(0),
                    row.get(4).unwrap_or_else(|_| "[]".to_string()),
                    row.get(5).unwrap_or(0),
                ))
            },
        )?;

        let mut dates: Vec<String> = serde_json::from_str(&transcription_dates_json).unwrap_or_default();
        if contribution.date_added_to_streak_list {
            dates.retain(|date| date != &contribution.date_key);
        }
        let new_dates_json = serde_json::to_string(&dates).unwrap_or_else(|_| "[]".to_string());

        let next_total_words = total_words.saturating_sub(contribution.word_count).max(0);
        let next_total_duration_ms = total_duration_ms
            .saturating_sub(contribution.recording_duration_ms)
            .max(0);
        let next_total_speech_duration_ms = total_speech_duration_ms
            .saturating_sub(contribution.speech_duration_ms)
            .max(0);
        let next_total_transcriptions = total_transcriptions.saturating_sub(1).max(0);
        let next_total_filler_words_removed = total_filler_words_removed
            .saturating_sub(contribution.filler_words_removed)
            .max(0);

        tx.execute(
            "UPDATE user_stats SET
                total_words = ?1,
                total_duration_ms = ?2,
                total_speech_duration_ms = ?3,
                total_transcriptions = ?4,
                transcription_dates = ?5,
                total_filler_words_removed = ?6
             WHERE id = 1",
            params![
                next_total_words,
                next_total_duration_ms,
                next_total_speech_duration_ms,
                next_total_transcriptions,
                new_dates_json,
                next_total_filler_words_removed
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        self.with_write_permit(|| self.cleanup_old_entries_impl())
    }

    fn cleanup_old_entries_impl(&self) -> Result<()> {
        let retention_period = crate::settings::get_recording_retention_period(&self.app_handle);

        match retention_period {
            crate::settings::RecordingRetentionPeriod::Never => {
                // Don't delete anything
                return Ok(());
            }
            crate::settings::RecordingRetentionPeriod::PreserveLimit => {
                // Use the old count-based logic with history_limit
                let limit = crate::settings::get_history_limit(&self.app_handle);
                return self.cleanup_by_count(limit);
            }
            _ => {
                // Use time-based logic
                return self.cleanup_by_time(retention_period);
            }
        }
    }

    fn delete_entries_and_files(&self, entries: &[(i64, String)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let conn = self.get_connection()?;
        let mut deleted_count = 0;

        for (id, file_name) in entries {
            // Delete database entry
            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            // Delete WAV file
            let file_path = self.recordings_dir.join(file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete WAV file {}: {}", file_name, e);
                } else {
                    debug!("Deleted old WAV file: {}", file_name);
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    fn cleanup_by_count(&self, limit: usize) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];
            let deleted_count = self.delete_entries_and_files(entries_to_delete)?;

            if deleted_count > 0 {
                debug!("Cleaned up {} old history entries by count", deleted_count);
            }
        }

        Ok(())
    }

    fn cleanup_by_time(
        &self,
        retention_period: crate::settings::RecordingRetentionPeriod,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        // Calculate cutoff timestamp (current time minus retention period)
        let now = Utc::now().timestamp();
        let cutoff_timestamp = match retention_period {
            crate::settings::RecordingRetentionPeriod::Days3 => now - (3 * 24 * 60 * 60), // 3 days in seconds
            crate::settings::RecordingRetentionPeriod::Weeks2 => now - (2 * 7 * 24 * 60 * 60), // 2 weeks in seconds
            crate::settings::RecordingRetentionPeriod::Months3 => now - (3 * 30 * 24 * 60 * 60), // 3 months in seconds (approximate)
            _ => unreachable!("Should not reach here"),
        };

        // Get all unsaved entries older than the cutoff timestamp
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 AND timestamp < ?1",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries_to_delete: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        let deleted_count = self.delete_entries_and_files(&entries_to_delete)?;

        if deleted_count > 0 {
            debug!(
                "Cleaned up {} old history entries based on retention period",
                deleted_count
            );
        }

        Ok(())
    }

    pub fn get_storage_usage(&self) -> Result<HistoryStats> {
        let conn = self.get_connection()?;
        
        let total_entries: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transcription_history",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Calculate size of all WAV files in the recordings directory
        let mut total_size_bytes: u64 = 0;
        if self.recordings_dir.exists() {
            match fs::read_dir(&self.recordings_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                total_size_bytes += metadata.len();
                            }
                        }
                    }
                }
                Err(e) => error!("Failed to read recordings directory: {}", e),
            }
        }
        
        Ok(HistoryStats {
            total_size_bytes,
            total_entries,
        })
    }

    pub fn prune_older_than(&self, days: u64) -> Result<usize> {
        self.with_write_permit(|| self.prune_older_than_impl(days))
    }

    fn prune_older_than_impl(&self, days: u64) -> Result<usize> {
        let now = Utc::now().timestamp();
        let cutoff_timestamp = now - (days as i64 * 24 * 60 * 60);

        debug!("Pruning history older than {} days (cutoff: {}, now: {})", days, cutoff_timestamp, now);

        let conn = self.get_connection()?;
        
        // Get entries older than cutoff
        let mut stmt = conn.prepare(
            "SELECT id, file_name, timestamp, saved FROM transcription_history WHERE timestamp < ?1 AND saved = 0",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            let id: i64 = row.get("id")?;
            let ts: i64 = row.get("timestamp")?;
            let saved: bool = row.get("saved")?;
            debug!("Found candidate for pruning: ID={}, TS={}, Saved={}", id, ts, saved);
            Ok((id, row.get::<_, String>("file_name")?))
        })?;

        let mut entries_to_delete: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        info!("Found {} unsaved entries older than {} days to delete", entries_to_delete.len(), days);

        let count = self.delete_entries_and_files(&entries_to_delete)?;
        debug!("Pruned {} entries older than {} days", count, days);
        
        // Emit history updated event if we deleted anything
        if count > 0 {
            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!("Failed to emit history-updated event: {}", e);
            }
        }

        Ok(count)
    }

    pub async fn get_history_entries(
        &self,
        limit: usize,
        offset: usize,
        search_query: Option<String>,
        starred_only: bool,
        time_period_start: Option<i64>,
    ) -> Result<Vec<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_history_entries_with_conn(
            &conn,
            &self.recordings_dir,
            limit,
            offset,
            search_query,
            starred_only,
            time_period_start,
        )
    }

    fn get_history_entries_with_conn(
        conn: &Connection,
        recordings_dir: &PathBuf,
        limit: usize,
        offset: usize,
        search_query: Option<String>,
        starred_only: bool,
        time_period_start: Option<i64>,
    ) -> Result<Vec<HistoryEntry>> {
        let mut query = String::from(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, inserted_text, post_process_prompt, duration_ms 
             FROM transcription_history"
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_index = 1;
        let mut has_where = false;

        if let Some(query_str) = search_query {
            if !query_str.trim().is_empty() {
                // Escape LIKE wildcards so literal '%' and '_' in user input
                // don't act as SQL pattern characters.
                let escaped = query_str.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
                let like_query = format!("%{}%", escaped);
                query.push_str(
                    " WHERE (COALESCE(inserted_text, post_processed_text, transcription_text) LIKE ?",
                );
                query.push_str(&param_index.to_string());
                query.push_str(" ESCAPE '\\'");
                params.push(Box::new(like_query.clone()));
                param_index += 1;
                query.push_str(" OR transcription_text LIKE ?");
                query.push_str(&param_index.to_string());
                query.push_str(" ESCAPE '\\'");
                params.push(Box::new(like_query));
                param_index += 1;
                query.push(')');
                has_where = true;
            }
        }

        if starred_only {
            if has_where {
                query.push_str(" AND");
            } else {
                query.push_str(" WHERE");
                has_where = true;
            }
            query.push_str(" saved = 1");
        }

        if let Some(start_ts) = time_period_start {
            if has_where {
                query.push_str(" AND");
            } else {
                query.push_str(" WHERE");
                #[allow(unused_assignments)]
                { has_where = true; }
            }
            query.push_str(&format!(" timestamp >= ?{}", param_index));
            params.push(Box::new(start_ts));
            param_index += 1;
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ?");
        query.push_str(&param_index.to_string());
        params.push(Box::new(limit as i64));
        param_index += 1;

        query.push_str(" OFFSET ?");
        query.push_str(&param_index.to_string());
        params.push(Box::new(offset as i64));

        let mut stmt = conn.prepare(&query)?;

        // rusqlite's params_from_iter expects a reference to a slice of dyn ToSql
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(rusqlite::params_from_iter(params_refs), |row| {
            map_history_entry(row, recordings_dir)
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    pub fn get_latest_entry(&self) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_latest_entry_with_conn(&conn, &self.recordings_dir)
    }

    fn get_latest_entry_with_conn(conn: &Connection, recordings_dir: &PathBuf) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, inserted_text, post_process_prompt, duration_ms
             FROM transcription_history
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt
            .query_row([], |row| {
                map_history_entry(row, recordings_dir)
            })
            .optional()?;

        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        self.with_write_permit(|| {
            let conn = self.get_connection()?;

            let current_saved: bool = conn.query_row(
                "SELECT saved FROM transcription_history WHERE id = ?1",
                params![id],
                |row| row.get("saved"),
            )?;

            let new_saved = !current_saved;

            conn.execute(
                "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
                params![new_saved, id],
            )?;

            debug!("Toggled saved status for entry {}: {}", id, new_saved);

            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!("Failed to emit history-updated event: {}", e);
            }

            Ok(())
        })
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        self.with_write_permit(|| {
            let conn = self.get_connection()?;

            if let Some(entry) = Self::get_entry_by_id_with_conn(&conn, &self.recordings_dir, id)? {
                let file_path = self.get_audio_file_path(&entry.file_name);
                if file_path.exists() {
                    if let Err(e) = fs::remove_file(&file_path) {
                        error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    }
                }
            }

            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            debug!("Deleted history entry with id: {}", id);

            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!("Failed to emit history-updated event: {}", e);
            }

            Ok(())
        })
    }

    /// Clear all history entries, deleting all audio files and database records
    pub async fn clear_all_entries(&self) -> Result<()> {
        self.with_write_permit(|| {
            let conn = self.get_connection()?;
            let entries = Self::get_history_entries_with_conn(
                &conn,
                &self.recordings_dir,
                usize::MAX,
                0,
                None,
                false,
                None,
            )?;
            let total = entries.len();

            info!("Clearing all {} history entries", total);

            for entry in entries {
                let file_path = self.get_audio_file_path(&entry.file_name);
                if file_path.exists() {
                    if let Err(e) = fs::remove_file(&file_path) {
                        error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    }
                }
            }

            conn.execute("DELETE FROM transcription_history", [])?;

            info!("Cleared all {} history entries", total);

            if let Err(e) = self.app_handle.emit("history-updated", ()) {
                error!("Failed to emit history-updated event: {}", e);
            }

            Ok(())
        })
    }

    fn get_entry_by_id_with_conn(
        conn: &Connection,
        recordings_dir: &PathBuf,
        id: i64,
    ) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, inserted_text, post_process_prompt, duration_ms
             FROM transcription_history WHERE id = ?1",
        )?;

        let entry = stmt
            .query_row([id], |row| map_history_entry(row, recordings_dir))
            .optional()?;

        Ok(entry)
    }

    fn format_timestamp_title(&self, timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }

    pub fn get_home_stats(&self) -> Result<HomeStats> {
        let conn = self.get_connection()?;
        
        let (
            total_words,
            total_duration_ms,
            total_speech_duration_ms,
            duration_stats_semantics_version,
            transcription_dates_json,
            total_filler_words_removed
        ): (i64, i64, i64, i64, String, i64) = conn.query_row(
            "SELECT
                total_words,
                total_duration_ms,
                COALESCE(total_speech_duration_ms, 0),
                COALESCE(duration_stats_semantics_version, 0),
                transcription_dates,
                COALESCE(total_filler_words_removed, 0)
             FROM user_stats WHERE id = 1",
            [],
            |row| Ok((
                row.get(0).unwrap_or(0),
                row.get(1).unwrap_or(0),
                row.get(2).unwrap_or(0),
                row.get(3).unwrap_or(0),
                row.get(4).unwrap_or_else(|_| "[]".to_string()),
                row.get(5).unwrap_or(0),
            )),
        ).unwrap_or((0, 0, 0, 0, "[]".to_string(), 0));

        // Read filler filter setting
        let filler_filter_active = crate::settings::get_settings(&self.app_handle).enable_filler_word_filter;
        let (total_duration_minutes, _speech_duration_minutes, wpm, time_saved_minutes) =
            compute_duration_metrics(
                total_words,
                total_duration_ms,
                total_speech_duration_ms,
                duration_stats_semantics_version,
            );

        info!(
            "Stats from DB: Words={}, RecordingDuration={}ms, SpeechDuration={}ms, SemanticsVersion={}, WPM={:.2}, FillerWordsRemoved={}, FillerFilterActive={}",
            total_words,
            total_duration_ms,
            total_speech_duration_ms,
            duration_stats_semantics_version,
            wpm,
            total_filler_words_removed,
            filler_filter_active
        );

        // Streak calculation
        let dates: Vec<String> = serde_json::from_str(&transcription_dates_json).unwrap_or_default();
        
        // Parse dates to NaiveDate
        let mut naive_dates: Vec<chrono::NaiveDate> = dates
            .iter()
            .filter_map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .collect();
            
        naive_dates.sort();
        naive_dates.dedup();
        
        let today = Local::now().date_naive();
        let mut streak_days = 0;
        
        if !naive_dates.is_empty() {
            let last_date = *naive_dates.last().unwrap();
            let diff = today.signed_duration_since(last_date).num_days();
            
            if diff <= 1 {
                streak_days = 1;
                let mut current_expected = last_date.pred_opt().unwrap();
                
                for i in (0..naive_dates.len() - 1).rev() {
                    if naive_dates[i] == current_expected {
                        streak_days += 1;
                        if let Some(pred) = current_expected.pred_opt() {
                            current_expected = pred;
                        } else {
                            break;
                        }
                    } else if naive_dates[i] < current_expected {
                        break;
                    } 
                }
            }
        }

        let mut faster_than_typing_percentage = 0.0;
        if wpm > 40.0 {
            faster_than_typing_percentage = ((wpm - 40.0) / 40.0) * 100.0;
        }

        Ok(HomeStats {
            total_words,
            total_duration_minutes,
            wpm,
            time_saved_minutes,
            streak_days,
            faster_than_typing_percentage,
            total_filler_words_removed,
            filler_filter_active,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use rusqlite_migration::Migrations;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{fs, path::PathBuf};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                inserted_text TEXT,
                post_process_prompt TEXT,
                duration_ms INTEGER DEFAULT 0,
                speech_duration_ms INTEGER DEFAULT 0
            );
            CREATE TABLE user_stats (
                id INTEGER PRIMARY KEY DEFAULT 1,
                total_words INTEGER DEFAULT 0,
                total_duration_ms INTEGER DEFAULT 0,
                total_speech_duration_ms INTEGER DEFAULT 0,
                total_transcriptions INTEGER DEFAULT 0,
                first_transcription_date INTEGER,
                last_transcription_date INTEGER,
                transcription_dates TEXT DEFAULT '[]',
                total_filler_words_removed INTEGER DEFAULT 0,
                duration_stats_semantics_version INTEGER DEFAULT 0
            );
            CREATE TABLE user_stats_migration_backup (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at INTEGER NOT NULL,
                backup_path TEXT,
                payload_json TEXT NOT NULL
            );
            INSERT INTO user_stats (
                id,
                total_words,
                total_duration_ms,
                total_speech_duration_ms,
                total_transcriptions,
                transcription_dates,
                total_filler_words_removed,
                duration_stats_semantics_version
            ) VALUES (1, 0, 0, 0, 0, '[]', 0, 0);",
        )
        .expect("create transcription_history table");
        conn
    }

    fn insert_entry(
        conn: &Connection,
        timestamp: i64,
        text: &str,
        post_processed: Option<&str>,
        inserted: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO transcription_history (file_name, timestamp, saved, title, transcription_text, post_processed_text, inserted_text, post_process_prompt, duration_ms, speech_duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                format!("codictate-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed,
                inserted,
                Option::<String>::None,
                0, // duration_ms
                0 // speech_duration_ms
            ],
        )
        .expect("insert history entry");
    }

    fn insert_duration_entry(
        conn: &Connection,
        timestamp: i64,
        recording_duration_ms: i64,
        speech_duration_ms: i64,
    ) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                inserted_text,
                post_process_prompt,
                duration_ms,
                speech_duration_ms
            ) VALUES (?1, ?2, 0, ?3, 'test', NULL, NULL, NULL, ?4, ?5)",
            params![
                format!("codictate-{}.wav", timestamp),
                timestamp,
                format!("Recording {}", timestamp),
                recording_duration_ms,
                speech_duration_ms
            ],
        )
        .expect("insert duration entry");
    }

    fn make_temp_backup_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "handy-{}-{}-{}",
            prefix,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("create temporary backup directory");
        path
    }

    #[test]
    fn normalize_runtime_durations_enforces_invariants() {
        let (recording, speech) = normalize_runtime_durations(1_000, 9_000);
        assert_eq!(recording, 1_000);
        assert_eq!(speech, 1_000);

        let (recording_from_speech, speech_from_speech) = normalize_runtime_durations(0, 700);
        assert_eq!(recording_from_speech, 700);
        assert_eq!(speech_from_speech, 700);

        let (recording_clamped, speech_clamped) = normalize_runtime_durations(-500, -10);
        assert_eq!(recording_clamped, 0);
        assert_eq!(speech_clamped, 0);
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let dummy_path = std::path::PathBuf::from("/tmp");
        let entry = HistoryManager::get_latest_entry_with_conn(&conn, &dummy_path).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None, None);
        insert_entry(&conn, 200, "second", Some("processed"), Some("inserted"));

        let dummy_path = std::path::PathBuf::from("/tmp");
        let entry = HistoryManager::get_latest_entry_with_conn(&conn, &dummy_path)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
        assert_eq!(entry.inserted_text.as_deref(), Some("inserted"));
        assert_eq!(entry.effective_text, "inserted");
        assert_eq!(entry.raw_text, "second");
    }

    #[test]
    fn get_history_entries_marks_audio_file_existence() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "missing", None, None);
        insert_entry(&conn, 200, "present", None, None);

        let recordings_dir = make_temp_backup_dir("history-audio-existence");
        fs::write(recordings_dir.join("codictate-200.wav"), b"RIFF").expect("seed audio file");

        let entries = HistoryManager::get_history_entries_with_conn(
            &conn,
            &recordings_dir,
            10,
            0,
            None,
            false,
            None,
        )
        .expect("fetch history entries");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file_name, "codictate-200.wav");
        assert!(entries[0].audio_file_exists);
        assert_eq!(entries[1].file_name, "codictate-100.wav");
        assert!(!entries[1].audio_file_exists);

        fs::remove_dir_all(recordings_dir).expect("cleanup recordings dir");
    }

    #[test]
    fn effective_text_uses_inserted_text_first() {
        assert_eq!(
            compute_effective_text(Some("inserted"), Some("processed"), "raw"),
            "inserted"
        );
    }

    #[test]
    fn effective_text_falls_back_to_post_processed() {
        assert_eq!(
            compute_effective_text(None, Some("processed"), "raw"),
            "processed"
        );
    }

    #[test]
    fn effective_text_falls_back_to_raw() {
        assert_eq!(compute_effective_text(None, None, "raw"), "raw");
    }

    #[test]
    fn update_inserted_text_updates_exact_row_id() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "one", Some("processed one"), None);
        insert_entry(&conn, 200, "two", Some("processed two"), None);

        let id_to_update: i64 = conn
            .query_row(
                "SELECT id FROM transcription_history WHERE timestamp = 100",
                [],
                |row| row.get(0),
            )
            .expect("row id to update");

        HistoryManager::update_inserted_text_by_id_with_conn(
            &conn,
            id_to_update,
            "inserted one".to_string(),
        )
        .expect("update inserted text");

        let inserted_first: Option<String> = conn
            .query_row(
                "SELECT inserted_text FROM transcription_history WHERE timestamp = 100",
                [],
                |row| row.get(0),
            )
            .expect("first inserted");
        let inserted_second: Option<String> = conn
            .query_row(
                "SELECT inserted_text FROM transcription_history WHERE timestamp = 200",
                [],
                |row| row.get(0),
            )
            .expect("second inserted");

        assert_eq!(inserted_first.as_deref(), Some("inserted one"));
        assert_eq!(inserted_second, None);
    }

    #[test]
    fn update_refine_output_updates_exact_row_id() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "one", None, None);
        insert_entry(&conn, 200, "two", Some("processed two"), None);

        let id_to_update: i64 = conn
            .query_row(
                "SELECT id FROM transcription_history WHERE timestamp = 100",
                [],
                |row| row.get(0),
            )
            .expect("row id to update");

        HistoryManager::update_refine_output_by_id_with_conn(
            &conn,
            id_to_update,
            "processed one".to_string(),
            Some("prompt".to_string()),
        )
        .expect("update refine output");

        let (post_processed_first, prompt_first): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT post_processed_text, post_process_prompt FROM transcription_history WHERE timestamp = 100",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("updated row");
        let post_processed_second: Option<String> = conn
            .query_row(
                "SELECT post_processed_text FROM transcription_history WHERE timestamp = 200",
                [],
                |row| row.get(0),
            )
            .expect("untouched row");

        assert_eq!(post_processed_first.as_deref(), Some("processed one"));
        assert_eq!(prompt_first.as_deref(), Some("prompt"));
        assert_eq!(post_processed_second.as_deref(), Some("processed two"));
    }

    #[test]
    fn update_refine_output_preserves_existing_inserted_text() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "raw asr", Some("old processed"), Some("previously pasted"));

        let id: i64 = conn
            .query_row(
                "SELECT id FROM transcription_history WHERE timestamp = 100",
                [],
                |row| row.get(0),
            )
            .expect("row id");

        // Simulate a refine that updates post_processed_text but paste fails,
        // so update_inserted_text_by_id is never called.
        HistoryManager::update_refine_output_by_id_with_conn(
            &conn,
            id,
            "new refined output".to_string(),
            Some("new prompt".to_string()),
        )
        .expect("update refine output");

        let (post_processed, inserted): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT post_processed_text, inserted_text FROM transcription_history WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read row after refine");

        assert_eq!(post_processed.as_deref(), Some("new refined output"));
        // inserted_text must be preserved — spec requires it is only updated on paste success
        assert_eq!(inserted.as_deref(), Some("previously pasted"));
    }

    #[test]
    fn migration_adds_inserted_text_without_mutating_existing_rows() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                post_process_prompt TEXT,
                duration_ms INTEGER DEFAULT 0
            );
            CREATE TABLE user_stats (
                id INTEGER PRIMARY KEY DEFAULT 1,
                total_words INTEGER DEFAULT 0,
                total_duration_ms INTEGER DEFAULT 0,
                total_transcriptions INTEGER DEFAULT 0,
                first_transcription_date INTEGER,
                last_transcription_date INTEGER,
                transcription_dates TEXT DEFAULT '[]'
            );
            INSERT INTO user_stats (id, total_words, total_duration_ms, total_transcriptions, transcription_dates)
            VALUES (1, 0, 0, 0, '[]');
            INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                duration_ms
            ) VALUES (
                'codictate-1.wav',
                42,
                0,
                'Legacy row',
                'legacy raw',
                'legacy processed',
                'legacy prompt',
                123
            );",
        )
        .expect("create legacy schema");

        conn.pragma_update(None, "user_version", 4_i32)
            .expect("set legacy version");

        let migrations = Migrations::new(MIGRATIONS.to_vec());
        migrations.to_latest(&mut conn).expect("run migrations");

        let inserted: Option<String> = conn
            .query_row(
                "SELECT inserted_text FROM transcription_history WHERE timestamp = 42",
                [],
                |row| row.get(0),
            )
            .expect("read inserted_text after migration");
        let (raw, processed, prompt, duration): (String, Option<String>, Option<String>, i64) =
            conn.query_row(
                "SELECT transcription_text, post_processed_text, post_process_prompt, duration_ms
                 FROM transcription_history WHERE timestamp = 42",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read existing row data");

        assert_eq!(inserted, None);
        assert_eq!(raw, "legacy raw");
        assert_eq!(processed.as_deref(), Some("legacy processed"));
        assert_eq!(prompt.as_deref(), Some("legacy prompt"));
        assert_eq!(duration, 123);
    }

    #[test]
    fn reconcile_legacy_schema_handles_duplicate_filler_column_state_safely() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                inserted_text TEXT,
                post_process_prompt TEXT,
                duration_ms INTEGER DEFAULT 0
            );
            CREATE TABLE user_stats (
                id INTEGER PRIMARY KEY DEFAULT 1,
                total_words INTEGER DEFAULT 0,
                total_duration_ms INTEGER DEFAULT 0,
                total_transcriptions INTEGER DEFAULT 0,
                first_transcription_date INTEGER,
                last_transcription_date INTEGER,
                transcription_dates TEXT DEFAULT '[]',
                total_filler_words_removed INTEGER DEFAULT 0
            );
            INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                inserted_text,
                post_process_prompt,
                duration_ms
            ) VALUES (
                'codictate-legacy.wav',
                100,
                0,
                'Legacy row',
                'legacy raw',
                'legacy processed',
                'legacy inserted',
                'legacy prompt',
                321
            );
            INSERT INTO user_stats (
                id,
                total_words,
                total_duration_ms,
                total_transcriptions,
                transcription_dates,
                total_filler_words_removed
            ) VALUES (
                1,
                12,
                3456,
                2,
                '[\"2026-02-19\"]',
                7
            );",
        )
        .expect("seed fully-migrated schema");

        // Simulate stale migration tracking: schema already has the column but version is behind.
        conn.pragma_update(None, "user_version", 6_i32)
            .expect("set stale version");

        HistoryManager::reconcile_legacy_schema_state(&conn)
            .expect("reconcile legacy schema state");

        let version_after_reconcile: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read version after reconcile");
        assert_eq!(version_after_reconcile, MIGRATIONS.len() as i32);

        // Verify migrations no longer fail with duplicate-column error.
        let migrations = Migrations::new(MIGRATIONS.to_vec());
        migrations.to_latest(&mut conn).expect("to_latest after reconcile");

        let (raw, inserted, filler_removed): (String, Option<String>, i64) = conn
            .query_row(
                "SELECT transcription_text, inserted_text, (SELECT total_filler_words_removed FROM user_stats WHERE id = 1)
                 FROM transcription_history WHERE timestamp = 100",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("verify preserved data");

        assert_eq!(raw, "legacy raw");
        assert_eq!(inserted.as_deref(), Some("legacy inserted"));
        assert_eq!(filler_removed, 7);
    }

    #[test]
    fn reconcile_legacy_schema_adds_missing_filler_column_without_data_loss() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                post_process_prompt TEXT,
                duration_ms INTEGER DEFAULT 0
            );
            CREATE TABLE user_stats (
                id INTEGER PRIMARY KEY DEFAULT 1,
                total_words INTEGER DEFAULT 0,
                total_duration_ms INTEGER DEFAULT 0,
                total_transcriptions INTEGER DEFAULT 0,
                first_transcription_date INTEGER,
                last_transcription_date INTEGER,
                transcription_dates TEXT DEFAULT '[]'
            );
            INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                duration_ms
            ) VALUES (
                'codictate-old.wav',
                42,
                0,
                'Old row',
                'old raw',
                'old processed',
                'old prompt',
                111
            );
            INSERT INTO user_stats (
                id,
                total_words,
                total_duration_ms,
                total_transcriptions,
                transcription_dates
            ) VALUES (
                1,
                100,
                9999,
                4,
                '[\"2026-02-18\"]'
            );",
        )
        .expect("seed pre-filler schema");

        conn.pragma_update(None, "user_version", 6_i32)
            .expect("set old version");

        HistoryManager::reconcile_legacy_schema_state(&conn)
            .expect("reconcile pre-filler schema");

        let has_filler = HistoryManager::column_exists(&conn, "user_stats", "total_filler_words_removed")
            .expect("check filler column exists");
        assert!(has_filler);

        let (raw, processed, filler_removed): (String, Option<String>, i64) = conn
            .query_row(
                "SELECT
                    transcription_text,
                    post_processed_text,
                    (SELECT total_filler_words_removed FROM user_stats WHERE id = 1)
                 FROM transcription_history WHERE timestamp = 42",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("verify row preserved");

        assert_eq!(raw, "old raw");
        assert_eq!(processed.as_deref(), Some("old processed"));
        assert_eq!(filler_removed, 0);
    }

    fn search_matching_entry_ids(conn: &Connection, query: &str) -> Vec<i64> {
        let like_query = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT id
                 FROM transcription_history
                 WHERE (
                   COALESCE(inserted_text, post_processed_text, transcription_text) LIKE ?1
                   OR transcription_text LIKE ?1
                 )
                 ORDER BY timestamp DESC",
            )
            .expect("prepare search query");

        let rows = stmt
            .query_map(params![like_query], |row| row.get::<_, i64>(0))
            .expect("run search query");

        rows.map(|row| row.expect("row"))
            .collect::<Vec<i64>>()
    }

    #[test]
    fn search_matches_effective_text_when_inserted_text_present() {
        let conn = setup_conn();
        insert_entry(
            &conn,
            100,
            "raw text that should not match",
            Some("processed"),
            Some("final inserted output"),
        );

        let ids = search_matching_entry_ids(&conn, "inserted output");
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn search_matches_raw_text_even_when_effective_text_differs() {
        let conn = setup_conn();
        insert_entry(
            &conn,
            100,
            "raw-only-token",
            Some("processed content"),
            Some("final inserted output"),
        );

        let ids = search_matching_entry_ids(&conn, "raw-only-token");
        assert_eq!(ids.len(), 1);
    }

    fn read_stats(conn: &Connection) -> (i64, i64, i64, i64, String, i64) {
        conn.query_row(
            "SELECT total_words, total_duration_ms, total_speech_duration_ms, total_transcriptions, transcription_dates, total_filler_words_removed FROM user_stats WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get(0).unwrap_or(0),
                    row.get(1).unwrap_or(0),
                    row.get(2).unwrap_or(0),
                    row.get(3).unwrap_or(0),
                    row.get(4).unwrap_or_else(|_| "[]".to_string()),
                    row.get(5).unwrap_or(0),
                ))
            },
        )
        .expect("read stats row")
    }

    #[test]
    fn rollback_stats_contribution_reverses_stats_and_removes_date_when_added() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats SET total_words = 120, total_duration_ms = 6000, total_speech_duration_ms = 5400, total_transcriptions = 3, transcription_dates = '[\"2026-02-15\"]', total_filler_words_removed = 9 WHERE id = 1",
            [],
        )
        .expect("seed stats");

        let contribution = StatsContribution {
            word_count: 40,
            recording_duration_ms: 2000,
            speech_duration_ms: 1500,
            filler_words_removed: 3,
            date_added_to_streak_list: true,
            date_key: "2026-02-15".to_string(),
        };

        HistoryManager::rollback_stats_contribution_with_conn(&mut conn, &contribution)
            .expect("rollback stats");

        let (words, recording_duration, speech_duration, total_transcriptions, dates_json, filler_removed) = read_stats(&conn);
        assert_eq!(words, 80);
        assert_eq!(recording_duration, 4000);
        assert_eq!(speech_duration, 3900);
        assert_eq!(total_transcriptions, 2);
        assert_eq!(dates_json, "[]");
        assert_eq!(filler_removed, 6);
    }

    #[test]
    fn rollback_stats_contribution_keeps_date_when_not_added_by_contribution() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats SET total_words = 80, total_duration_ms = 3200, total_speech_duration_ms = 2800, total_transcriptions = 2, transcription_dates = '[\"2026-02-15\"]', total_filler_words_removed = 4 WHERE id = 1",
            [],
        )
        .expect("seed stats");

        let contribution = StatsContribution {
            word_count: 20,
            recording_duration_ms: 1200,
            speech_duration_ms: 900,
            filler_words_removed: 1,
            date_added_to_streak_list: false,
            date_key: "2026-02-15".to_string(),
        };

        HistoryManager::rollback_stats_contribution_with_conn(&mut conn, &contribution)
            .expect("rollback stats");

        let (words, recording_duration, speech_duration, total_transcriptions, dates_json, filler_removed) = read_stats(&conn);
        assert_eq!(words, 60);
        assert_eq!(recording_duration, 2000);
        assert_eq!(speech_duration, 1900);
        assert_eq!(total_transcriptions, 1);
        assert_eq!(dates_json, "[\"2026-02-15\"]");
        assert_eq!(filler_removed, 3);
    }

    #[test]
    fn rollback_stats_contribution_clamps_values_to_zero() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats SET total_words = 5, total_duration_ms = 100, total_speech_duration_ms = 50, total_transcriptions = 0, transcription_dates = '[\"2026-02-15\"]', total_filler_words_removed = 1 WHERE id = 1",
            [],
        )
        .expect("seed stats");

        let contribution = StatsContribution {
            word_count: 50,
            recording_duration_ms: 5000,
            speech_duration_ms: 4000,
            filler_words_removed: 9,
            date_added_to_streak_list: true,
            date_key: "2026-02-15".to_string(),
        };

        HistoryManager::rollback_stats_contribution_with_conn(&mut conn, &contribution)
            .expect("rollback stats");

        let (words, recording_duration, speech_duration, total_transcriptions, dates_json, filler_removed) = read_stats(&conn);
        assert_eq!(words, 0);
        assert_eq!(recording_duration, 0);
        assert_eq!(speech_duration, 0);
        assert_eq!(total_transcriptions, 0);
        assert_eq!(dates_json, "[]");
        assert_eq!(filler_removed, 0);
    }

    #[test]
    fn compute_duration_metrics_uses_hybrid_semantics_when_marker_active() {
        let (recording_minutes, speech_minutes, wpm, time_saved) =
            compute_duration_metrics(300, 120_000, 60_000, DURATION_SEMANTICS_VERSION_V1);

        assert!((recording_minutes - 2.0).abs() < 0.000_1);
        assert!((speech_minutes - 1.0).abs() < 0.000_1);
        assert!((wpm - 300.0).abs() < 0.000_1);
        assert!((time_saved - 5.5).abs() < 0.000_1);
    }

    #[test]
    fn compute_duration_metrics_falls_back_to_legacy_when_marker_inactive() {
        let (recording_minutes, speech_minutes, wpm, _) =
            compute_duration_metrics(300, 120_000, 60_000, 0);

        assert!((recording_minutes - 2.0).abs() < 0.000_1);
        assert!((speech_minutes - 2.0).abs() < 0.000_1);
        assert!((wpm - 150.0).abs() < 0.000_1);
    }

    #[test]
    fn duration_semantics_backfill_is_idempotent_and_creates_single_backup() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats
             SET total_duration_ms = 6000,
                 total_speech_duration_ms = 0,
                 duration_stats_semantics_version = 0
             WHERE id = 1",
            [],
        )
        .expect("seed legacy totals");

        insert_duration_entry(&conn, 1, 2000, 1500);
        // Legacy compatibility: speech_duration_ms missing in old rows falls back to recording duration.
        insert_duration_entry(&conn, 2, 1000, 0);

        let backup_dir = make_temp_backup_dir("duration-idempotent");
        HistoryManager::ensure_duration_stats_semantics_with_backup_root(&mut conn, backup_dir.as_path())
            .expect("first backfill pass");
        HistoryManager::ensure_duration_stats_semantics_with_backup_root(&mut conn, backup_dir.as_path())
            .expect("second backfill pass");

        let (recording_total, speech_total, version): (i64, i64, i64) = conn
            .query_row(
                "SELECT total_duration_ms, total_speech_duration_ms, duration_stats_semantics_version
                 FROM user_stats WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read backfilled totals");
        assert_eq!(recording_total, 7800);
        assert_eq!(speech_total, 7300);
        assert_eq!(version, DURATION_SEMANTICS_VERSION_V1);

        let backup_row_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM user_stats_migration_backup", [], |row| {
                row.get(0)
            })
            .expect("read backup row count");
        assert_eq!(backup_row_count, 1);

        let backup_path: String = conn
            .query_row(
                "SELECT backup_path FROM user_stats_migration_backup ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("read backup path");
        assert!(PathBuf::from(backup_path).exists());

        let backup_file_count = fs::read_dir(&backup_dir)
            .expect("read backup dir")
            .count();
        assert_eq!(backup_file_count, 1);

        let _ = fs::remove_dir_all(backup_dir);
    }

    #[test]
    fn duration_semantics_backfill_preserves_residual_when_history_rows_missing() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats
             SET total_duration_ms = 9100,
                 total_speech_duration_ms = 0,
                 duration_stats_semantics_version = 0
             WHERE id = 1",
            [],
        )
        .expect("seed legacy totals");

        // Simulates pruned/deleted history rows: no live rows to contribute to sums.
        let backup_dir = make_temp_backup_dir("duration-residual");
        HistoryManager::ensure_duration_stats_semantics_with_backup_root(&mut conn, backup_dir.as_path())
            .expect("run backfill");

        let (recording_total, speech_total, version): (i64, i64, i64) = conn
            .query_row(
                "SELECT total_duration_ms, total_speech_duration_ms, duration_stats_semantics_version
                 FROM user_stats WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read backfilled totals");
        assert_eq!(recording_total, 9100);
        assert_eq!(speech_total, 9100);
        assert_eq!(version, DURATION_SEMANTICS_VERSION_V1);

        let _ = fs::remove_dir_all(backup_dir);
    }

    #[test]
    fn duration_semantics_backfill_falls_back_to_recording_for_legacy_rows() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats
             SET total_duration_ms = 100,
                 total_speech_duration_ms = 0,
                 duration_stats_semantics_version = 0
             WHERE id = 1",
            [],
        )
        .expect("seed legacy totals");
        insert_duration_entry(&conn, 1, 1000, 0);

        let backup_dir = make_temp_backup_dir("duration-legacy-speech-fallback");
        HistoryManager::ensure_duration_stats_semantics_with_backup_root(&mut conn, backup_dir.as_path())
            .expect("run backfill");

        let (recording_total, speech_total): (i64, i64) = conn
            .query_row(
                "SELECT total_duration_ms, total_speech_duration_ms FROM user_stats WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read totals");
        // old_total_duration_ms == legacy_effective_duration_ms(1000), so residual is zero.
        assert_eq!(recording_total, 1000);
        assert_eq!(speech_total, 1000);

        let _ = fs::remove_dir_all(backup_dir);
    }

    #[test]
    fn duration_semantics_backfill_clamps_malformed_speech_duration_rows() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats
             SET total_duration_ms = 100,
                 total_speech_duration_ms = 0,
                 duration_stats_semantics_version = 0
             WHERE id = 1",
            [],
        )
        .expect("seed legacy totals");
        insert_duration_entry(&conn, 1, 1000, 9000);

        let backup_dir = make_temp_backup_dir("duration-malformed-clamp");
        HistoryManager::ensure_duration_stats_semantics_with_backup_root(
            &mut conn,
            backup_dir.as_path(),
        )
        .expect("run backfill");

        let (recording_total, speech_total): (i64, i64) = conn
            .query_row(
                "SELECT total_duration_ms, total_speech_duration_ms FROM user_stats WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read totals");
        assert_eq!(recording_total, 1000);
        assert_eq!(speech_total, 1000);

        let _ = fs::remove_dir_all(backup_dir);
    }

    #[test]
    fn duration_semantics_backfill_reuses_existing_backup_row_before_marker_flip() {
        let mut conn = setup_conn();
        conn.execute(
            "UPDATE user_stats
             SET total_duration_ms = 100,
                 total_speech_duration_ms = 0,
                 duration_stats_semantics_version = 0
             WHERE id = 1",
            [],
        )
        .expect("seed legacy totals");
        insert_duration_entry(&conn, 1, 1000, 0);

        let snapshot = HistoryManager::snapshot_user_stats(&conn, 0).expect("snapshot stats");
        let payload_json = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");
        conn.execute(
            "INSERT INTO user_stats_migration_backup (created_at, backup_path, payload_json)
             VALUES (?1, NULL, ?2)",
            params![Utc::now().timestamp(), payload_json],
        )
        .expect("insert existing backup row");

        let backup_dir = make_temp_backup_dir("duration-existing-backup");
        HistoryManager::ensure_duration_stats_semantics_with_backup_root(
            &mut conn,
            backup_dir.as_path(),
        )
        .expect("run backfill");

        let backup_row_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM user_stats_migration_backup", [], |row| {
                row.get(0)
            })
            .expect("count backup rows");
        assert_eq!(backup_row_count, 1);

        let stored_backup_path: Option<String> = conn
            .query_row(
                "SELECT backup_path FROM user_stats_migration_backup WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("read backup path");
        let stored_backup_path = stored_backup_path.expect("backup path must be set");
        assert!(PathBuf::from(stored_backup_path).exists());

        let version: i64 = conn
            .query_row(
                "SELECT duration_stats_semantics_version FROM user_stats WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("read semantics marker");
        assert_eq!(version, DURATION_SEMANTICS_VERSION_V1);

        let _ = fs::remove_dir_all(backup_dir);
    }
}
