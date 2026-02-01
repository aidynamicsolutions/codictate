use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use tracing::{debug, error, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
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
    pub post_process_prompt: Option<String>,
    pub duration_ms: i64,
    pub file_path: String,
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
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
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

    /// Save a transcription to history (both database and WAV file)
    pub async fn save_transcription(
        &self,
        audio_samples: Vec<f32>,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
        duration_ms: i64,
    ) -> Result<()> {
        let timestamp = Utc::now().timestamp();
        let file_name = format!("codictate-{}.wav", timestamp);
        let title = self.format_timestamp_title(timestamp);

        // Save WAV file
        let file_path = self.recordings_dir.join(&file_name);
        save_wav_file(file_path, &audio_samples).await?;

        // Save to database
        self.save_to_database(
            file_name,
            timestamp,
            title,
            transcription_text,
            post_processed_text,
            post_process_prompt,
            duration_ms,
        )?;

        // Clean up old entries
        self.cleanup_old_entries()?;

        // Emit history updated event
        info!("Emitting history-updated event");
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        } else {
            debug!("Successfully emitted history-updated event");
        }

        Ok(())
    }

    fn save_to_database(
        &self,
        file_name: String,
        timestamp: i64,
        title: String,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
        duration_ms: i64,
    ) -> Result<()> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        // 1. Insert into transcription_history
        tx.execute(
            "INSERT INTO transcription_history (file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![file_name, timestamp, false, title, transcription_text, post_processed_text, post_process_prompt, duration_ms],
        )?;

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
        if !dates.contains(&today_str) {
            dates.push(today_str);
        }
        let new_dates_json = serde_json::to_string(&dates).unwrap_or_else(|_| "[]".to_string());

        // Upsert stats (assuming row 1 exists due to backfill/init)
        tx.execute(
            "UPDATE user_stats SET 
                total_words = total_words + ?1,
                total_duration_ms = total_duration_ms + ?2,
                total_transcriptions = total_transcriptions + 1,
                last_transcription_date = ?3,
                transcription_dates = ?4,
                first_transcription_date = COALESCE(first_transcription_date, ?3)
             WHERE id = 1",
            params![word_count, duration_ms, timestamp, new_dates_json],
        )?;

        tx.commit()?;

        debug!("Saved transcription to database and updated stats");
        Ok(())
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
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
    ) -> Result<Vec<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut query = String::from(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, duration_ms 
             FROM transcription_history"
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_index = 1;

        if let Some(query_str) = search_query {
            if !query_str.trim().is_empty() {
                query.push_str(" WHERE transcription_text LIKE ?");
                query.push_str(&param_index.to_string());
                params.push(Box::new(format!("%{}%", query_str)));
                param_index += 1;
            }
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
            Ok(HistoryEntry {
                id: row.get("id")?,
                file_name: row.get("file_name")?,
                timestamp: row.get("timestamp")?,
                saved: row.get("saved")?,
                title: row.get("title")?,
                transcription_text: row.get("transcription_text")?,
                post_processed_text: row.get("post_processed_text")?,
                post_process_prompt: row.get("post_process_prompt")?,
                duration_ms: row.get("duration_ms")?,
                file_path: self.recordings_dir.join(row.get::<_, String>("file_name")?).to_string_lossy().to_string(),
            })
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
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, duration_ms
             FROM transcription_history
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt
            .query_row([], |row| {
                Ok(HistoryEntry {
                    id: row.get("id")?,
                    file_name: row.get("file_name")?,
                    timestamp: row.get("timestamp")?,
                    saved: row.get("saved")?,
                    title: row.get("title")?,
                    transcription_text: row.get("transcription_text")?,
                    post_processed_text: row.get("post_processed_text")?,
                    post_process_prompt: row.get("post_process_prompt")?,
                    duration_ms: row.get("duration_ms")?,
                    file_path: recordings_dir.join(row.get::<_, String>("file_name")?).to_string_lossy().to_string(),
                })
            })
            .optional()?;

        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
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

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, duration_ms
             FROM transcription_history WHERE id = ?1",
        )?;

        let entry = stmt
            .query_row([id], |row| {
                Ok(HistoryEntry {
                    id: row.get("id")?,
                    file_name: row.get("file_name")?,
                    timestamp: row.get("timestamp")?,
                    saved: row.get("saved")?,
                    title: row.get("title")?,
                    transcription_text: row.get("transcription_text")?,
                    post_processed_text: row.get("post_processed_text")?,
                    post_process_prompt: row.get("post_process_prompt")?,
                    duration_ms: row.get("duration_ms")?,
                    file_path: self.recordings_dir.join(row.get::<_, String>("file_name")?).to_string_lossy().to_string(),
                })
            })
            .optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get the entry to find the file name
        if let Some(entry) = self.get_entry_by_id(id).await? {
            // Delete the audio file first
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    /// Clear all history entries, deleting all audio files and database records
    pub async fn clear_all_entries(&self) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries to delete their audio files
        // Use a large limit to get all entries. Since we are clearing everything, pagination isn't strictly needed here 
        // but the method signature changed.
        let entries = self.get_history_entries(usize::MAX, 0, None).await?;
        let total = entries.len();
        
        info!("Clearing all {} history entries", total);

        // Delete all audio files
        for entry in entries {
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with other deletions
                }
            }
        }

        // Delete all records from database
        conn.execute("DELETE FROM transcription_history", [])?;

        info!("Cleared all {} history entries", total);

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
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
        
        let (total_words, total_duration_ms, transcription_dates_json): (i64, i64, String) = conn.query_row(
            "SELECT total_words, total_duration_ms, transcription_dates FROM user_stats WHERE id = 1",
            [],
            |row| Ok((
                row.get(0).unwrap_or(0),
                row.get(1).unwrap_or(0),
                row.get(2).unwrap_or_else(|_| "[]".to_string())
            )),
        ).unwrap_or((0, 0, "[]".to_string()));

        let total_duration_minutes = total_duration_ms as f64 / 60000.0;
        
        // WPM Calculation
        // Note: Ideally we should track "wpm_total_duration" separately to exclude
        // legacy 0-duration entries if we want perfection, but for lifetime stats 
        // across many entries, the impact shrinks. 
        // For now, we utilize the global counters directly.
        
        let wpm = if total_duration_minutes > 0.001 {
            total_words as f64 / total_duration_minutes
        } else {
            0.0
        };

        // Time Saved
        let time_to_type_minutes = total_words as f64 / 40.0;
        let time_saved_minutes = time_to_type_minutes - total_duration_minutes;

        info!("Stats from DB: Words={}, Duration={}ms, WPM={:.2}", total_words, total_duration_ms, wpm);

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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

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
                post_process_prompt TEXT,
                duration_ms INTEGER DEFAULT 0
            );",
        )
        .expect("create transcription_history table");
        conn
    }

    fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
        conn.execute(
            "INSERT INTO transcription_history (file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("handy-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed,
                Option::<String>::None,
                0 // duration_ms
            ],
        )
        .expect("insert history entry");
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None);
        insert_entry(&conn, 200, "second", Some("processed"));

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
    }
}
