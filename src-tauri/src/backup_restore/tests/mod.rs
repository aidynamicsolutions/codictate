//! Backup/restore integration-style and unit test coverage.

use super::*;
use super::backup::{
    export_history_jsonl, export_recordings_payload, map_stage_progress_units,
    package_progress_units, package_workspace_to_archive_with_cancel,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{json, Value};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex, MutexGuard};
use std::time::Duration as StdDuration;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::App;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

    static BR_TEST_GUARD: Mutex<()> = Mutex::new(());

    struct TestEnvGuard {
        root: TempDir,
        home_original: Option<String>,
        xdg_original: Option<String>,
        failpoint_original: Option<String>,
        manual_stats_repair_original: Option<String>,
        payload_limit_original: Option<String>,
        extract_payload_limit_original: Option<String>,
        extract_total_limit_original: Option<String>,
        archive_entries_limit_original: Option<String>,
    }

    impl TestEnvGuard {
        fn new() -> Self {
            let root = tempfile::tempdir().expect("create test env temp dir");
            let home = root.path().join("home");
            let xdg_data_home = root.path().join("xdg-data");
            fs::create_dir_all(&home).expect("create test HOME");
            fs::create_dir_all(&xdg_data_home).expect("create test XDG_DATA_HOME");

            let home_original = std::env::var("HOME").ok();
            let xdg_original = std::env::var("XDG_DATA_HOME").ok();
            let failpoint_original = std::env::var("HANDY_TEST_BR_FAILPOINT").ok();
            let manual_stats_repair_original =
                std::env::var("HANDY_MANUAL_STATS_REPAIR_20260303").ok();
            let payload_limit_original = std::env::var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES").ok();
            let extract_payload_limit_original =
                std::env::var("HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES").ok();
            let extract_total_limit_original =
                std::env::var("HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES").ok();
            let archive_entries_limit_original =
                std::env::var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES").ok();

            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_DATA_HOME", &xdg_data_home);
            std::env::remove_var("HANDY_TEST_BR_FAILPOINT");
            std::env::remove_var("HANDY_MANUAL_STATS_REPAIR_20260303");
            std::env::remove_var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES");
            std::env::remove_var("HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES");
            std::env::remove_var("HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES");
            std::env::remove_var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES");

            Self {
                root,
                home_original,
                xdg_original,
                failpoint_original,
                manual_stats_repair_original,
                payload_limit_original,
                extract_payload_limit_original,
                extract_total_limit_original,
                archive_entries_limit_original,
            }
        }

        fn set_failpoint(&self, value: &str) {
            std::env::set_var("HANDY_TEST_BR_FAILPOINT", value);
        }

        fn clear_failpoint(&self) {
            std::env::remove_var("HANDY_TEST_BR_FAILPOINT");
        }

        fn enable_manual_stats_repair(&self) {
            std::env::set_var("HANDY_MANUAL_STATS_REPAIR_20260303", "1");
        }

        fn disable_manual_stats_repair(&self) {
            std::env::remove_var("HANDY_MANUAL_STATS_REPAIR_20260303");
        }

        fn set_payload_limit_bytes(&self, bytes: u64) {
            std::env::set_var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES", bytes.to_string());
        }

        fn clear_payload_limit_bytes(&self) {
            std::env::remove_var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES");
        }

        fn set_extract_payload_limit_bytes(&self, bytes: u64) {
            std::env::set_var(
                "HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES",
                bytes.to_string(),
            );
        }

        fn clear_extract_payload_limit_bytes(&self) {
            std::env::remove_var("HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES");
        }

        fn set_extract_total_limit_bytes(&self, bytes: u64) {
            std::env::set_var(
                "HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES",
                bytes.to_string(),
            );
        }

        fn clear_extract_total_limit_bytes(&self) {
            std::env::remove_var("HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES");
        }

        fn set_archive_entries_limit(&self, entries: u64) {
            std::env::set_var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES", entries.to_string());
        }

        fn clear_archive_entries_limit(&self) {
            std::env::remove_var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES");
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            let _ = self.root.path();
            match &self.home_original {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }

            match &self.xdg_original {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }

            match &self.failpoint_original {
                Some(value) => std::env::set_var("HANDY_TEST_BR_FAILPOINT", value),
                None => std::env::remove_var("HANDY_TEST_BR_FAILPOINT"),
            }

            match &self.manual_stats_repair_original {
                Some(value) => std::env::set_var("HANDY_MANUAL_STATS_REPAIR_20260303", value),
                None => std::env::remove_var("HANDY_MANUAL_STATS_REPAIR_20260303"),
            }

            match &self.payload_limit_original {
                Some(value) => std::env::set_var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES", value),
                None => std::env::remove_var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES"),
            }

            match &self.extract_payload_limit_original {
                Some(value) => {
                    std::env::set_var("HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES", value)
                }
                None => std::env::remove_var("HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES"),
            }

            match &self.extract_total_limit_original {
                Some(value) => std::env::set_var("HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES", value),
                None => std::env::remove_var("HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES"),
            }

            match &self.archive_entries_limit_original {
                Some(value) => std::env::set_var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES", value),
                None => std::env::remove_var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES"),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestHistoryRow {
        id: i64,
        file_name: String,
        title: String,
        transcription_text: String,
        timestamp: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DictionaryEntrySnapshot {
        input: String,
        aliases: Vec<String>,
        replacement: String,
        is_replacement: bool,
        fuzzy_enabled: Option<bool>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct StateSnapshot {
        history_rows: Vec<TestHistoryRow>,
        user_stats: Option<TestUserStatsSnapshot>,
        dictionary_entries: Vec<DictionaryEntrySnapshot>,
        user_store: Value,
        recordings: Vec<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestUserStatsSnapshot {
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

    fn build_test_app() -> App<MockRuntime> {
        let app = mock_builder()
            .manage(BackupRestoreRuntime::default())
            .build(mock_context(noop_assets()))
            .expect("build mock app");

        let dictionary_state = crate::user_dictionary::initialize_dictionary_state(app.handle());
        let managed = app.manage(dictionary_state);
        assert!(managed, "dictionary state should be managed exactly once");

        app
    }

    fn setup_test_app() -> (MutexGuard<'static, ()>, TestEnvGuard, App<MockRuntime>, PathBuf) {
        let guard = BR_TEST_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let env = TestEnvGuard::new();
        let app = build_test_app();
        let app_data = app_data_dir(app.handle()).expect("resolve app data dir");
        fs::create_dir_all(&app_data).expect("create app data dir");
        assert!(
            app_data.starts_with(env.root.path()),
            "test app data path should stay inside isolated temp root"
        );
        (guard, env, app, app_data)
    }

    fn custom_word(input: &str, replacement: &str) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            aliases: Vec::new(),
            replacement: replacement.to_string(),
            is_replacement: true,
            fuzzy_enabled: Some(false),
        }
    }

    fn history_rows(prefix: &str) -> Vec<TestHistoryRow> {
        vec![
            TestHistoryRow {
                id: 1,
                file_name: format!("{prefix}-1.wav"),
                title: format!("{prefix} title 1"),
                transcription_text: format!("{prefix} transcription 1"),
                timestamp: 1_700_000_001,
            },
            TestHistoryRow {
                id: 2,
                file_name: format!("{prefix}-2.wav"),
                title: format!("{prefix} title 2"),
                transcription_text: format!("{prefix} transcription 2"),
                timestamp: 1_700_000_002,
            },
        ]
    }

    fn seed_history_db(app_data_dir: &Path, rows: &[TestHistoryRow]) {
        let db_path = app_data_dir.join(HISTORY_DB_FILE);
        initialize_history_db(&db_path).expect("initialize history db");
        let conn = Connection::open(&db_path).expect("open history db");

        conn.execute("DELETE FROM transcription_history", [])
            .expect("clear history rows");
        conn.execute("DELETE FROM user_stats", [])
            .expect("clear user stats");

        for row in rows {
            conn.execute(
                "INSERT INTO transcription_history (
                    id,
                    file_name,
                    timestamp,
                    saved,
                    title,
                    transcription_text,
                    post_processed_text,
                    post_process_prompt,
                    duration_ms,
                    inserted_text,
                    speech_duration_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, ?7, NULL, ?8)",
                params![
                    row.id,
                    row.file_name,
                    row.timestamp,
                    false,
                    row.title,
                    row.transcription_text,
                    1_000_i64,
                    900_i64,
                ],
            )
            .expect("insert history row");
        }

        let total_words: i64 = rows
            .iter()
            .map(|row| crate::audio_toolkit::text::count_words(&row.transcription_text) as i64)
            .sum();
        let total_duration_ms = (rows.len() as i64).saturating_mul(1_000);
        let total_speech_duration_ms = (rows.len() as i64).saturating_mul(900);
        let first_timestamp = rows.iter().map(|row| row.timestamp).min();
        let last_timestamp = rows.iter().map(|row| row.timestamp).max();
        let transcription_dates: Vec<String> = rows
            .iter()
            .filter_map(|row| DateTime::<Utc>::from_timestamp(row.timestamp, 0))
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d").to_string())
            .collect();
        let transcription_dates_json =
            serde_json::to_string(&transcription_dates).expect("serialize seed date keys");

        conn.execute(
            "INSERT INTO user_stats (
                id,
                total_words,
                total_duration_ms,
                total_transcriptions,
                first_transcription_date,
                last_transcription_date,
                transcription_dates,
                total_filler_words_removed,
                total_speech_duration_ms,
                duration_stats_semantics_version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                1_i64,
                total_words,
                total_duration_ms,
                rows.len() as i64,
                first_timestamp,
                last_timestamp,
                transcription_dates_json,
                0_i64,
                total_speech_duration_ms,
                1_i64,
            ],
        )
        .expect("insert user stats seed row");
    }

    fn seed_user_stats(app_data_dir: &Path, stats: &TestUserStatsSnapshot) {
        let db_path = app_data_dir.join(HISTORY_DB_FILE);
        initialize_history_db(&db_path).expect("initialize history db for user_stats seed");
        let conn = Connection::open(&db_path).expect("open history db for user_stats seed");

        conn.execute("DELETE FROM user_stats", [])
            .expect("clear existing user_stats row");
        conn.execute(
            "INSERT INTO user_stats (
                id,
                total_words,
                total_duration_ms,
                total_transcriptions,
                first_transcription_date,
                last_transcription_date,
                transcription_dates,
                total_filler_words_removed,
                total_speech_duration_ms,
                duration_stats_semantics_version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                1_i64,
                stats.total_words,
                stats.total_duration_ms,
                stats.total_transcriptions,
                stats.first_transcription_date,
                stats.last_transcription_date,
                serde_json::to_string(&stats.transcription_dates).expect("serialize seeded dates"),
                stats.total_filler_words_removed,
                stats.total_speech_duration_ms,
                stats.duration_stats_semantics_version,
            ],
        )
        .expect("insert user_stats seed row");
    }

    fn seed_dictionary<R: tauri::Runtime>(app: &AppHandle<R>, entries: Vec<CustomWordEntry>) {
        crate::user_dictionary::set_dictionary_entries(app, entries).expect("seed dictionary");
    }

    fn seed_user_store(app_data_dir: &Path, value: &Value) {
        write_json_file_atomically(&app_data_dir.join(USER_STORE_DB_FILE), value)
            .expect("seed user store");
    }

    fn seed_recordings(app_data_dir: &Path, files: &[(&str, &[u8])]) {
        let recordings = app_data_dir.join(RECORDINGS_DIR);
        if recordings.exists() {
            fs::remove_dir_all(&recordings).expect("reset recordings dir");
        }
        fs::create_dir_all(&recordings).expect("create recordings dir");
        for (name, content) in files {
            fs::write(recordings.join(name), content).expect("write recording fixture");
        }
    }

    fn write_large_file(path: &Path, total_bytes: usize) {
        let mut file = File::create(path).expect("create large fixture file");
        let chunk = vec![b'X'; 1024 * 1024];
        let mut remaining = total_bytes;
        while remaining > 0 {
            let take = remaining.min(chunk.len());
            file.write_all(&chunk[..take])
                .expect("write large fixture chunk");
            remaining -= take;
        }
        file.flush().expect("flush large fixture file");
        file.sync_all().expect("sync large fixture file");
    }

    fn set_cancel_requested<R: tauri::Runtime>(app: &AppHandle<R>, requested: bool) {
        let runtime = app.state::<BackupRestoreRuntime>();
        runtime.cancel_requested.store(requested, Ordering::SeqCst);
    }

    fn read_history_rows(app_data_dir: &Path) -> Vec<TestHistoryRow> {
        let db_path = app_data_dir.join(HISTORY_DB_FILE);
        if !db_path.exists() {
            return Vec::new();
        }

        let conn = Connection::open(db_path).expect("open history db for assertions");
        let mut stmt = conn
            .prepare(
                "SELECT id, file_name, title, transcription_text, timestamp
                 FROM transcription_history
                 ORDER BY id ASC",
            )
            .expect("prepare history query");

        stmt.query_map([], |row| {
            Ok(TestHistoryRow {
                id: row.get(0)?,
                file_name: row.get(1)?,
                title: row.get(2)?,
                transcription_text: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })
        .expect("query history rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect history rows")
    }

    fn read_dictionary_entries(app_data_dir: &Path) -> Vec<DictionaryEntrySnapshot> {
        let dictionary_path = app_data_dir.join(USER_DICTIONARY_FILE);
        if !dictionary_path.exists() {
            return Vec::new();
        }

        let payload = read_json_file::<DictionaryPayload>(&dictionary_path)
            .expect("read dictionary payload for assertions");
        let mut entries = payload
            .entries
            .into_iter()
            .map(|entry| DictionaryEntrySnapshot {
                input: entry.input,
                aliases: entry.aliases,
                replacement: entry.replacement,
                is_replacement: entry.is_replacement,
                fuzzy_enabled: entry.fuzzy_enabled,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.input.cmp(&b.input));
        entries
    }

    fn read_user_store(app_data_dir: &Path) -> Value {
        let path = app_data_dir.join(USER_STORE_DB_FILE);
        if !path.exists() {
            return json!({});
        }
        read_json_file(&path).expect("read user store")
    }

    fn read_user_stats(app_data_dir: &Path) -> Option<TestUserStatsSnapshot> {
        let db_path = app_data_dir.join(HISTORY_DB_FILE);
        if !db_path.exists() {
            return None;
        }
        let conn = Connection::open(db_path).expect("open history db for user_stats assertions");
        let user_stats_table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='user_stats'",
                [],
                |row| row.get(0),
            )
            .expect("check user_stats table existence");
        if !user_stats_table_exists {
            return None;
        }

        conn.query_row(
            "SELECT
                COALESCE(total_words, 0),
                COALESCE(total_duration_ms, 0),
                COALESCE(total_transcriptions, 0),
                first_transcription_date,
                last_transcription_date,
                COALESCE(transcription_dates, '[]'),
                COALESCE(total_filler_words_removed, 0),
                COALESCE(total_speech_duration_ms, 0),
                COALESCE(duration_stats_semantics_version, 0)
             FROM user_stats
             WHERE id = 1",
            [],
            |row| {
                let transcription_dates_json: String = row.get(5)?;
                Ok(TestUserStatsSnapshot {
                    total_words: row.get(0)?,
                    total_duration_ms: row.get(1)?,
                    total_transcriptions: row.get(2)?,
                    first_transcription_date: row.get(3)?,
                    last_transcription_date: row.get(4)?,
                    transcription_dates: serde_json::from_str::<Vec<String>>(&transcription_dates_json)
                        .unwrap_or_default(),
                    total_filler_words_removed: row.get(6)?,
                    total_speech_duration_ms: row.get(7)?,
                    duration_stats_semantics_version: row.get(8)?,
                })
            },
        )
        .optional()
        .expect("read user_stats row")
    }

    fn collect_recordings_recursive(root: &Path, current: &Path, out: &mut Vec<String>) {
        if !current.exists() {
            return;
        }
        let entries = fs::read_dir(current).expect("read recordings directory");
        for entry in entries {
            let entry = entry.expect("read recordings entry");
            let path = entry.path();
            let file_type = entry.file_type().expect("inspect recordings entry type");
            if file_type.is_dir() {
                collect_recordings_recursive(root, &path, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("recording path should be under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(relative);
            }
        }
    }

    fn list_recordings(app_data_dir: &Path) -> Vec<String> {
        let recordings = app_data_dir.join(RECORDINGS_DIR);
        let mut files = Vec::new();
        collect_recordings_recursive(&recordings, &recordings, &mut files);
        files.sort();
        files
    }

    fn list_runtime_dirs_with_prefix(app_data_dir: &Path, prefix: &str) -> Vec<PathBuf> {
        let runtime = runtime_dir(app_data_dir);
        if !runtime.exists() {
            return Vec::new();
        }

        let mut dirs = Vec::new();
        for entry in fs::read_dir(&runtime).expect("read runtime directory") {
            let entry = entry.expect("read runtime directory entry");
            if !entry
                .file_type()
                .expect("inspect runtime directory entry type")
                .is_dir()
            {
                continue;
            }
            if entry.file_name().to_string_lossy().starts_with(prefix) {
                dirs.push(entry.path());
            }
        }
        dirs.sort();
        dirs
    }

    fn snapshot_state(app_data_dir: &Path) -> StateSnapshot {
        StateSnapshot {
            history_rows: read_history_rows(app_data_dir),
            user_stats: read_user_stats(app_data_dir),
            dictionary_entries: read_dictionary_entries(app_data_dir),
            user_store: read_user_store(app_data_dir),
            recordings: list_recordings(app_data_dir),
        }
    }

    fn make_backup<R: tauri::Runtime>(app: &AppHandle<R>, scope: BackupScope, name: &str) -> PathBuf {
        let app_data = app_data_dir(app).expect("resolve app data for backup path");
        let output = app_data.join(format!("{name}.{BACKUP_FILE_EXTENSION}"));
        if output.exists() {
            fs::remove_file(&output).expect("remove stale backup output");
        }
        let report = create_backup(
            app,
            CreateBackupRequest {
                scope,
                output_path: output.to_string_lossy().to_string(),
            },
        )
        .expect("create backup");
        PathBuf::from(report.output_path)
    }

    fn rewrite_archive<F>(archive_path: &Path, mut mutator: F)
    where
        F: FnMut(&str, &[u8]) -> Option<Vec<u8>>,
    {
        let source = File::open(archive_path).expect("open archive for rewrite");
        let mut archive = ZipArchive::new(source).expect("parse archive for rewrite");

        let mut entries = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).expect("read archive entry for rewrite");
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .expect("read archive entry bytes for rewrite");
            entries.push((name, bytes));
        }
        drop(archive);

        let temp_path = archive_path.with_extension(format!("{}.tmp", BACKUP_FILE_EXTENSION));
        let output = File::create(&temp_path).expect("create temporary rewritten archive");
        let mut writer = ZipWriter::new(output);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for (name, bytes) in entries {
            if let Some(next_bytes) = mutator(&name, &bytes) {
                writer
                    .start_file(name, options)
                    .expect("start archive entry during rewrite");
                writer
                    .write_all(&next_bytes)
                    .expect("write archive entry during rewrite");
            }
        }

        let mut finished = writer.finish().expect("finish rewritten archive");
        finished.flush().expect("flush rewritten archive");
        finished.sync_all().expect("sync rewritten archive");
        drop(finished);

        fs::rename(&temp_path, archive_path).expect("replace archive with rewritten output");
    }

    fn tamper_archive_file<F>(archive_path: &Path, relative: &str, mutator: F)
    where
        F: FnOnce(Vec<u8>) -> Vec<u8>,
    {
        let mut mutator = Some(mutator);
        rewrite_archive(archive_path, |name, bytes| {
            if name == relative {
                let f = mutator
                    .take()
                    .expect("archive mutator should only run once");
                Some(f(bytes.to_vec()))
            } else {
                Some(bytes.to_vec())
            }
        });
    }

    fn rename_archive_entry(archive_path: &Path, from: &str, to: &str) {
        let source = File::open(archive_path).expect("open archive for entry rename");
        let mut archive = ZipArchive::new(source).expect("parse archive for entry rename");

        let mut entries = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).expect("read archive entry for rename");
            if entry.is_dir() {
                continue;
            }
            let name = if entry.name() == from {
                to.to_string()
            } else {
                entry.name().to_string()
            };
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .expect("read archive entry bytes for rename");
            entries.push((name, bytes));
        }
        drop(archive);

        let temp_path = archive_path.with_extension(format!("{}.tmp", BACKUP_FILE_EXTENSION));
        let output = File::create(&temp_path).expect("create temporary renamed archive");
        let mut writer = ZipWriter::new(output);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for (name, bytes) in entries {
            writer
                .start_file(name, options)
                .expect("start archive entry during rename");
            writer
                .write_all(&bytes)
                .expect("write archive entry during rename");
        }

        let mut finished = writer.finish().expect("finish renamed archive");
        finished.flush().expect("flush renamed archive");
        finished.sync_all().expect("sync renamed archive");
        drop(finished);

        fs::rename(&temp_path, archive_path).expect("replace archive with renamed output");
    }

    fn archive_entry_checksum(archive_path: &Path, relative: &str) -> String {
        let source = File::open(archive_path).expect("open archive for checksum");
        let mut archive = ZipArchive::new(source).expect("parse archive for checksum");
        let mut index = None;
        for i in 0..archive.len() {
            let entry = archive.by_index(i).expect("read archive entry for checksum index");
            if entry.name() == relative {
                index = Some(i);
                break;
            }
        }
        let entry_index = index.expect("expected archive entry to exist for checksum");
        checksum_zip_entry(&mut archive, entry_index).expect("calculate archive entry checksum")
    }

    fn replace_checksum_line(text: &str, relative: &str, checksum: &str) -> String {
        let mut found = false;
        let mut lines = Vec::new();

        for line in text.lines() {
            if line.trim_end().ends_with(relative) {
                lines.push(format!("{checksum}  {relative}"));
                found = true;
            } else {
                lines.push(line.to_string());
            }
        }

        assert!(found, "expected checksum line for {relative}");
        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        }
    }

    fn replace_checksum_path_line(
        text: &str,
        old_relative: &str,
        new_relative: &str,
        checksum: &str,
    ) -> String {
        let mut found = false;
        let mut lines = Vec::new();

        for line in text.lines() {
            if line.trim_end().ends_with(old_relative) {
                lines.push(format!("{checksum}  {new_relative}"));
                found = true;
            } else {
                lines.push(line.to_string());
            }
        }

        assert!(
            found,
            "expected checksum line for {old_relative} to replace with {new_relative}"
        );
        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        }
    }


include!("backup_export_tests.rs");
include!("preflight_restore_tests.rs");
include!("restore_startup_tests.rs");
include!("undo_checkpoint_tests.rs");
include!("path_validation_tests.rs");
include!("runtime_gating_tests.rs");
