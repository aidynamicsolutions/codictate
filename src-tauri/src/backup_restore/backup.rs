//! Backup export pipeline.
//!
//! Builds an isolated export workspace, writes payload files, generates
//! manifest/checksums, and packages the final archive atomically.

use super::*;

const ESTIMATED_ARCHIVE_METADATA_OVERHEAD_BYTES: u64 = 8 * 1024;
const BACKUP_PROGRESS_TOTAL_UNITS: u64 = 10_000;
const BACKUP_PROGRESS_PREPARE_UNITS: u64 = 100;
const BACKUP_PROGRESS_EXPORT_HISTORY_START_UNITS: u64 = BACKUP_PROGRESS_PREPARE_UNITS;
const BACKUP_PROGRESS_EXPORT_HISTORY_END_UNITS: u64 = 1_200;
const BACKUP_PROGRESS_EXPORT_DICTIONARY_START_UNITS: u64 = BACKUP_PROGRESS_EXPORT_HISTORY_END_UNITS;
const BACKUP_PROGRESS_EXPORT_DICTIONARY_END_UNITS: u64 = 1_700;
const BACKUP_PROGRESS_EXPORT_USER_STORE_START_UNITS: u64 =
    BACKUP_PROGRESS_EXPORT_DICTIONARY_END_UNITS;
const BACKUP_PROGRESS_EXPORT_USER_STORE_END_UNITS: u64 = 2_200;
const BACKUP_PROGRESS_EXPORT_RECORDINGS_START_UNITS: u64 =
    BACKUP_PROGRESS_EXPORT_USER_STORE_END_UNITS;
const BACKUP_PROGRESS_EXPORT_RECORDINGS_END_UNITS: u64 = 3_500;
const BACKUP_PROGRESS_CHECKSUMS_START_UNITS: u64 = BACKUP_PROGRESS_EXPORT_RECORDINGS_END_UNITS;
const BACKUP_PROGRESS_CHECKSUMS_END_UNITS: u64 = 4_500;
const BACKUP_PROGRESS_PACKAGE_START_UNITS: u64 = BACKUP_PROGRESS_CHECKSUMS_END_UNITS;
const BACKUP_PROGRESS_PACKAGE_END_UNITS: u64 = 9_900;
const PACKAGE_PROGRESS_MIN_EMIT_BYTES: u64 = 8 * 1024 * 1024;
const PACKAGE_PROGRESS_MAX_EMIT_INTERVAL: StdDuration = StdDuration::from_millis(250);
const HISTORY_PROGRESS_ROW_INTERVAL: u64 = 200;
const RECORDINGS_PROGRESS_FILE_INTERVAL: u64 = 5;
const CHECKSUM_PROGRESS_FILE_INTERVAL: u64 = 20;

fn emit_backup_progress<R: tauri::Runtime>(app: &AppHandle<R>, phase: &str, current: u64) {
    emit_progress(
        app,
        "backup",
        phase,
        current.min(BACKUP_PROGRESS_TOTAL_UNITS),
        BACKUP_PROGRESS_TOTAL_UNITS,
    );
}

pub(super) fn map_stage_progress_units(
    start_units: u64,
    end_units: u64,
    processed: u64,
    total: u64,
) -> u64 {
    if end_units <= start_units {
        return end_units;
    }
    if total == 0 {
        return end_units;
    }

    let span = end_units - start_units;
    let bounded_processed = processed.min(total);
    start_units + bounded_processed.saturating_mul(span) / total
}

fn should_emit_step_progress(
    processed: u64,
    last_emitted: u64,
    total: u64,
    interval: u64,
) -> bool {
    if total == 0 {
        return true;
    }
    processed == total || processed.saturating_sub(last_emitted) >= interval
}

pub(super) fn package_progress_units(bytes_written: u64, total_bytes: u64) -> u64 {
    if BACKUP_PROGRESS_PACKAGE_END_UNITS <= BACKUP_PROGRESS_PACKAGE_START_UNITS {
        return BACKUP_PROGRESS_PACKAGE_START_UNITS;
    }

    if total_bytes == 0 {
        return BACKUP_PROGRESS_PACKAGE_END_UNITS;
    }

    let span = BACKUP_PROGRESS_PACKAGE_END_UNITS - BACKUP_PROGRESS_PACKAGE_START_UNITS;
    let bounded_written = bytes_written.min(total_bytes);
    BACKUP_PROGRESS_PACKAGE_START_UNITS + bounded_written.saturating_mul(span) / total_bytes
}

pub fn create_backup<R: tauri::Runtime>(
    app: &AppHandle<R>,
    request: CreateBackupRequest,
) -> Result<CreateBackupReport, String> {
    ensure_supported_platform()?;
    let _op_guard = start_operation(app)?;
    let backup_started_at = std::time::Instant::now();
    info!(
        scope = ?request.scope,
        output_path = %request.output_path,
        "Backup operation started"
    );
    emit_backup_progress(app, "prepare", BACKUP_PROGRESS_PREPARE_UNITS);

    let output_path = normalize_output_archive_path(PathBuf::from(request.output_path));
    let app_data_dir = app_data_dir(app)?;
    let runtime_dir = runtime_dir(&app_data_dir);
    fs::create_dir_all(&runtime_dir)
        .map_err(|error| format!("Failed to create backup runtime directory: {error}"))?;

    let workspace = runtime_dir.join(format!("export-workspace-{}", timestamp_millis()));
    fs::create_dir_all(&workspace)
        .map_err(|error| format!("Failed to create backup workspace: {error}"))?;

    let backup_result = (|| -> Result<CreateBackupReport, String> {
        ensure_not_cancelled(app)?;

        let mut warnings = Vec::new();
        let mut referenced_recordings = BTreeSet::new();

        fs::create_dir_all(workspace.join("history"))
            .map_err(|error| format!("Failed to create history payload directory: {error}"))?;
        fs::create_dir_all(workspace.join("dictionary"))
            .map_err(|error| format!("Failed to create dictionary payload directory: {error}"))?;
        fs::create_dir_all(workspace.join("user"))
            .map_err(|error| format!("Failed to create user payload directory: {error}"))?;

        info!("Backup stage started: export-history");
        let export_history_started_at = std::time::Instant::now();
        let history_count = export_history_jsonl(
            &app_data_dir.join(HISTORY_DB_FILE),
            &workspace.join(HISTORY_FILE),
            matches!(request.scope, BackupScope::Complete),
            &mut referenced_recordings,
            app,
            |processed, total| {
                let current = map_stage_progress_units(
                    BACKUP_PROGRESS_EXPORT_HISTORY_START_UNITS,
                    BACKUP_PROGRESS_EXPORT_HISTORY_END_UNITS,
                    processed,
                    total,
                );
                emit_backup_progress(app, "export-history", current);
            },
        )?;
        info!(
            elapsed_ms = export_history_started_at.elapsed().as_millis() as u64,
            history_entries = history_count,
            referenced_recordings = referenced_recordings.len() as u64,
            "Backup stage completed: export-history"
        );
        validate_export_payload_file_size(&workspace.join(HISTORY_FILE), HISTORY_FILE)?;
        let user_stats_payload_exported = export_user_stats_payload(
            &app_data_dir.join(HISTORY_DB_FILE),
            &workspace.join(HISTORY_USER_STATS_FILE),
        )?;
        if user_stats_payload_exported {
            validate_export_payload_file_size(
                &workspace.join(HISTORY_USER_STATS_FILE),
                HISTORY_USER_STATS_FILE,
            )?;
        }

        ensure_not_cancelled(app)?;

        info!("Backup stage started: export-dictionary");
        let export_dictionary_started_at = std::time::Instant::now();
        emit_backup_progress(
            app,
            "export-dictionary",
            BACKUP_PROGRESS_EXPORT_DICTIONARY_START_UNITS,
        );
        let dictionary_count = export_dictionary_payload(app, &workspace.join(DICTIONARY_FILE))?;
        emit_backup_progress(
            app,
            "export-dictionary",
            BACKUP_PROGRESS_EXPORT_DICTIONARY_END_UNITS,
        );
        info!(
            elapsed_ms = export_dictionary_started_at.elapsed().as_millis() as u64,
            dictionary_entries = dictionary_count,
            "Backup stage completed: export-dictionary"
        );
        validate_export_payload_file_size(&workspace.join(DICTIONARY_FILE), DICTIONARY_FILE)?;

        ensure_not_cancelled(app)?;

        info!("Backup stage started: export-user-store");
        let export_user_store_started_at = std::time::Instant::now();
        emit_backup_progress(
            app,
            "export-user-store",
            BACKUP_PROGRESS_EXPORT_USER_STORE_START_UNITS,
        );
        export_user_store_payload(&app_data_dir, &workspace.join(USER_STORE_FILE))?;
        emit_backup_progress(
            app,
            "export-user-store",
            BACKUP_PROGRESS_EXPORT_USER_STORE_END_UNITS,
        );
        info!(
            elapsed_ms = export_user_store_started_at.elapsed().as_millis() as u64,
            "Backup stage completed: export-user-store"
        );
        validate_export_payload_file_size(&workspace.join(USER_STORE_FILE), USER_STORE_FILE)?;

        let recording_count = if matches!(request.scope, BackupScope::Complete) {
            info!("Backup stage started: export-recordings");
            let export_recordings_started_at = std::time::Instant::now();
            emit_backup_progress(
                app,
                "export-recordings",
                BACKUP_PROGRESS_EXPORT_RECORDINGS_START_UNITS,
            );
            let exported_recording_count = export_recordings_payload(
                app,
                &app_data_dir,
                &workspace,
                &referenced_recordings,
                &mut warnings,
                |processed, total| {
                    let current = map_stage_progress_units(
                        BACKUP_PROGRESS_EXPORT_RECORDINGS_START_UNITS,
                        BACKUP_PROGRESS_EXPORT_RECORDINGS_END_UNITS,
                        processed,
                        total,
                    );
                    emit_backup_progress(app, "export-recordings", current);
                },
            )?;
            info!(
                elapsed_ms = export_recordings_started_at.elapsed().as_millis() as u64,
                exported_recordings = exported_recording_count,
                warnings = warnings.len() as u64,
                "Backup stage completed: export-recordings"
            );
            exported_recording_count
        } else {
            info!("Backup stage skipped: export-recordings (smaller scope)");
            0
        };

        ensure_not_cancelled(app)?;

        let mut manifest = BackupManifest {
            backup_format_version: BACKUP_FORMAT_VERSION.to_string(),
            created_at: now_rfc3339(),
            created_with_app_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: current_platform().to_string(),
            includes_recordings: matches!(request.scope, BackupScope::Complete),
            estimated_size_bytes: 0,
            counts: ManifestCounts {
                history_entries: history_count,
                recording_files: recording_count,
                dictionary_entries: dictionary_count,
            },
            components: ManifestComponents {
                history_payload_version: HISTORY_PAYLOAD_VERSION,
                user_stats_payload_version: user_stats_payload_exported
                    .then_some(USER_STATS_PAYLOAD_VERSION),
                dictionary_payload_version: DICTIONARY_PAYLOAD_VERSION,
                user_store_payload_version: USER_STORE_PAYLOAD_VERSION,
            },
            warnings: ManifestWarnings {
                missing_recordings: warnings.clone(),
            },
        };

        write_json_file_atomically(&workspace.join(MANIFEST_FILE), &manifest)?;

        let mut payload_files = collect_payload_files(&workspace)?;
        payload_files.sort();
        validate_workspace_payload_size_limits(&workspace, &payload_files)?;

        manifest.estimated_size_bytes = payload_files
            .iter()
            .map(|relative| workspace.join(relative))
            .filter_map(|path| path.metadata().ok())
            .map(|meta| meta.len())
            .sum();

        write_json_file_atomically(&workspace.join(MANIFEST_FILE), &manifest)?;
        validate_workspace_payload_size_limits(&workspace, &payload_files)?;

        info!("Backup stage started: checksums");
        let checksums_started_at = std::time::Instant::now();
        emit_backup_progress(app, "checksums", BACKUP_PROGRESS_CHECKSUMS_START_UNITS);
        write_checksums_file(
            &workspace,
            &payload_files,
            || ensure_not_cancelled(app),
            |processed, total| {
                let current = map_stage_progress_units(
                    BACKUP_PROGRESS_CHECKSUMS_START_UNITS,
                    BACKUP_PROGRESS_CHECKSUMS_END_UNITS,
                    processed,
                    total,
                );
                emit_backup_progress(app, "checksums", current);
            },
        )?;
        info!(
            elapsed_ms = checksums_started_at.elapsed().as_millis() as u64,
            payload_files = payload_files.len() as u64,
            "Backup stage completed: checksums"
        );

        ensure_not_cancelled(app)?;

        info!("Backup stage started: package");
        let package_started_at = std::time::Instant::now();
        let mut last_package_checkpoint_percent = 0_u64;
        emit_backup_progress(app, "package", BACKUP_PROGRESS_PACKAGE_START_UNITS);
        package_workspace_to_archive_with_cancel(
            &workspace,
            &output_path,
            || ensure_not_cancelled(app),
            |bytes_written, total_bytes| {
                emit_backup_progress(app, "package", package_progress_units(bytes_written, total_bytes));
                if total_bytes == 0 {
                    return;
                }

                let checkpoint_percent =
                    bytes_written.saturating_mul(100) / total_bytes;
                let rounded_checkpoint = (checkpoint_percent / 10) * 10;
                let reached_next_checkpoint =
                    rounded_checkpoint >= last_package_checkpoint_percent.saturating_add(10);
                let is_final_checkpoint = bytes_written == total_bytes;
                if reached_next_checkpoint || is_final_checkpoint {
                    info!(
                        checkpoint_percent = checkpoint_percent.min(100),
                        bytes_written,
                        total_bytes,
                        elapsed_ms = package_started_at.elapsed().as_millis() as u64,
                        "Backup package progress checkpoint"
                    );
                    last_package_checkpoint_percent = rounded_checkpoint.min(100);
                }
            },
        )?;
        emit_backup_progress(app, "package", BACKUP_PROGRESS_TOTAL_UNITS);
        info!(
            elapsed_ms = package_started_at.elapsed().as_millis() as u64,
            "Backup stage completed: package"
        );

        Ok(CreateBackupReport {
            output_path: output_path.to_string_lossy().to_string(),
            counts: BackupCounts {
                history_entries: history_count,
                recording_files: recording_count,
                dictionary_entries: dictionary_count,
            },
            warnings,
        })
    })();

    let cleanup_error = fs::remove_dir_all(&workspace).err();
    if let Some(error) = cleanup_error {
        warn!(
            error = %error,
            path = %workspace.display(),
            "Failed to remove backup workspace"
        );
    }

    match &backup_result {
        Ok(report) => {
            info!(
                elapsed_ms = backup_started_at.elapsed().as_millis() as u64,
                history_entries = report.counts.history_entries,
                recording_files = report.counts.recording_files,
                dictionary_entries = report.counts.dictionary_entries,
                warnings = report.warnings.len() as u64,
                "Backup operation completed"
            );
        }
        Err(error) => {
            warn!(
                elapsed_ms = backup_started_at.elapsed().as_millis() as u64,
                error = %error,
                "Backup operation failed"
            );
        }
    }

    backup_result
}

pub fn get_backup_estimate<R: tauri::Runtime>(
    app: &AppHandle<R>,
) -> Result<BackupEstimateReport, String> {
    ensure_supported_platform()?;

    let app_data_dir = app_data_dir(app)?;
    let (
        history_entries,
        history_payload_estimated_bytes,
        recording_files,
        recording_bytes,
    ) = estimate_history_payload_and_recordings(&app_data_dir)?;

    let dictionary_entries = user_dictionary::get_dictionary_snapshot(app);
    let dictionary_entries_count = dictionary_entries.len() as u64;
    let dictionary_payload_estimated_bytes = serde_json::to_vec(&DictionaryPayload {
        version: DICTIONARY_PAYLOAD_VERSION,
        entries: dictionary_entries.as_ref().clone(),
    })
    .map_err(|error| format!("Failed to estimate dictionary payload size: {error}"))?
    .len() as u64;

    let user_store_payload_estimated_bytes = estimate_user_store_payload_size(&app_data_dir)?;

    let smaller_payload_bytes = history_payload_estimated_bytes
        .saturating_add(dictionary_payload_estimated_bytes)
        .saturating_add(user_store_payload_estimated_bytes);
    let complete_payload_bytes = smaller_payload_bytes.saturating_add(recording_bytes);

    let smaller_estimated_size_bytes = estimate_total_backup_bytes(
        smaller_payload_bytes,
        false,
        history_entries,
        dictionary_entries_count,
        0,
    );
    let complete_estimated_size_bytes = estimate_total_backup_bytes(
        complete_payload_bytes,
        true,
        history_entries,
        dictionary_entries_count,
        recording_files,
    );

    Ok(BackupEstimateReport {
        complete_estimated_size_bytes,
        smaller_estimated_size_bytes,
        difference_bytes: complete_estimated_size_bytes.saturating_sub(smaller_estimated_size_bytes),
        recording_files,
        history_entries,
        dictionary_entries: dictionary_entries_count,
    })
}

fn estimate_total_backup_bytes(
    payload_bytes: u64,
    includes_recordings: bool,
    history_entries: u64,
    dictionary_entries: u64,
    recording_files: u64,
) -> u64 {
    let mut manifest = BackupManifest {
        backup_format_version: BACKUP_FORMAT_VERSION.to_string(),
        created_at: now_rfc3339(),
        created_with_app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: current_platform().to_string(),
        includes_recordings,
        estimated_size_bytes: 0,
        counts: ManifestCounts {
            history_entries,
            recording_files,
            dictionary_entries,
        },
        components: ManifestComponents {
            history_payload_version: HISTORY_PAYLOAD_VERSION,
            user_stats_payload_version: Some(USER_STATS_PAYLOAD_VERSION),
            dictionary_payload_version: DICTIONARY_PAYLOAD_VERSION,
            user_store_payload_version: USER_STORE_PAYLOAD_VERSION,
        },
        warnings: ManifestWarnings::default(),
    };

    // Apply one refinement pass so manifest size reflects the estimated value width.
    let mut estimated_total = payload_bytes.saturating_add(ESTIMATED_ARCHIVE_METADATA_OVERHEAD_BYTES);
    for _ in 0..2 {
        manifest.estimated_size_bytes = estimated_total;
        let manifest_size = serde_json::to_vec(&manifest)
            .map(|bytes| bytes.len() as u64)
            .unwrap_or_default();
        estimated_total = payload_bytes
            .saturating_add(manifest_size)
            .saturating_add(ESTIMATED_ARCHIVE_METADATA_OVERHEAD_BYTES);
    }

    estimated_total
}

fn estimate_history_payload_and_recordings(
    app_data_dir: &Path,
) -> Result<(u64, u64, u64, u64), String> {
    let history_db_path = app_data_dir.join(HISTORY_DB_FILE);
    if !history_db_path.exists() {
        return Ok((0, 0, 0, 0));
    }

    let history_payload_estimated_bytes = history_db_path
        .metadata()
        .map_err(|error| format!("Failed to read history DB metadata for backup estimate: {error}"))?
        .len();

    let conn = Connection::open_with_flags(&history_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Failed to open history DB for backup estimate: {error}"))?;

    if !sqlite_table_exists(&conn, "transcription_history")? {
        return Ok((0, history_payload_estimated_bytes, 0, 0));
    }

    let history_entries = conn
        .query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            let count: i64 = row.get(0)?;
            Ok(count.max(0) as u64)
        })
        .map_err(|error| format!("Failed to count history rows for backup estimate: {error}"))?;

    let recordings_root = app_data_dir.join(RECORDINGS_DIR);
    ensure_safe_recordings_root_for_backup(&recordings_root)?;
    let mut recording_files = 0_u64;
    let mut recording_bytes = 0_u64;

    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT file_name
             FROM transcription_history
             WHERE file_name IS NOT NULL
               AND TRIM(file_name) != ''",
        )
        .map_err(|error| format!("Failed to query history recordings for backup estimate: {error}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Failed to iterate recording references for backup estimate: {error}"))?;

    for row in rows {
        let file_name = row
            .map_err(|error| format!("Failed to read recording reference for backup estimate: {error}"))?;
        let safe_name = match sanitize_relative_file_name(&file_name) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let source = recordings_root.join(safe_name);
        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }

        recording_files = recording_files.saturating_add(1);
        recording_bytes = recording_bytes.saturating_add(metadata.len());
    }

    Ok((
        history_entries,
        history_payload_estimated_bytes,
        recording_files,
        recording_bytes,
    ))
}

fn ensure_safe_recordings_root_for_backup(recordings_root: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(recordings_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to inspect recordings root for backup '{}': {error}",
                recordings_root.display()
            ));
        }
    };

    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to read recordings root '{}' because it is a symbolic link.",
            recordings_root.display()
        ));
    }

    Ok(())
}

fn sqlite_table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM sqlite_master
                 WHERE type = 'table' AND name = ?1
             )",
            [table_name],
            |row| row.get(0),
        )
        .map_err(|error| format!("Failed to inspect SQLite schema for backup estimate: {error}"))?;

    Ok(exists != 0)
}

fn estimate_user_store_payload_size(app_data_dir: &Path) -> Result<u64, String> {
    let user_store_path = app_data_dir.join(USER_STORE_DB_FILE);
    if !user_store_path.exists() {
        return Ok(2);
    }

    user_store_path
        .metadata()
        .map(|metadata| metadata.len())
        .map_err(|error| format!("Failed to read user-store metadata for backup estimate: {error}"))
}

pub(super) fn normalize_output_archive_path(mut output_path: PathBuf) -> PathBuf {
    if output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(BACKUP_FILE_EXTENSION))
        .unwrap_or(false)
    {
        return output_path;
    }

    output_path.set_extension(BACKUP_FILE_EXTENSION);
    output_path
}

pub(super) fn export_history_jsonl<R: tauri::Runtime>(
    history_db_path: &Path,
    output_path: &Path,
    collect_recordings: bool,
    referenced_recordings: &mut BTreeSet<String>,
    app: &AppHandle<R>,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<u64, String> {
    let conn = Connection::open_with_flags(history_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Failed to open history database for backup: {error}"))?;
    let total_rows = conn
        .query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            let count: i64 = row.get(0)?;
            Ok(count.max(0) as u64)
        })
        .map_err(|error| format!("Failed to count history rows for backup export: {error}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, inserted_text, post_process_prompt, duration_ms, COALESCE(speech_duration_ms, 0)
             FROM transcription_history
             ORDER BY id ASC",
        )
        .map_err(|error| format!("Failed to query history entries for backup: {error}"))?;

    let parent = output_path
        .parent()
        .ok_or_else(|| "Invalid history payload output path".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create history payload directory: {error}"))?;

    let mut writer = BufWriter::new(
        File::create(output_path)
            .map_err(|error| format!("Failed to create history payload file: {error}"))?,
    );

    let mut count = 0_u64;
    let mut last_emitted_count = 0_u64;
    on_progress(0, total_rows);
    let rows = stmt
        .query_map([], |row| {
            Ok(HistoryRowV1 {
                id: row.get(0)?,
                file_name: row.get(1)?,
                timestamp: row.get(2)?,
                saved: row.get(3)?,
                title: row.get(4)?,
                transcription_text: row.get(5)?,
                post_processed_text: row.get(6)?,
                inserted_text: row.get(7)?,
                post_process_prompt: row.get(8)?,
                duration_ms: row.get(9)?,
                speech_duration_ms: row.get(10)?,
            })
        })
        .map_err(|error| format!("Failed to iterate history entries for backup: {error}"))?;

    for row in rows {
        ensure_not_cancelled(app)?;
        let mut row =
            row.map_err(|error| format!("Failed to read history row for backup: {error}"))?;
        let safe_file_name = sanitize_relative_file_name(&row.file_name).map_err(|error| {
            format!(
                "Invalid history row file_name '{}' during backup export: {error}",
                row.file_name
            )
        })?;
        row.file_name = safe_file_name.clone();

        if collect_recordings {
            referenced_recordings.insert(safe_file_name);
        }

        let row_bytes = serde_json::to_vec(&row)
            .map_err(|error| format!("Failed to serialize history row for backup: {error}"))?;
        if row_bytes.len() > MAX_HISTORY_JSONL_LINE_BYTES {
            return Err(format!(
                "history_jsonl_line_too_large:{}>{}",
                row_bytes.len(),
                MAX_HISTORY_JSONL_LINE_BYTES
            ));
        }
        writer
            .write_all(&row_bytes)
            .map_err(|error| format!("Failed to write history row for backup: {error}"))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("Failed to write history row for backup: {error}"))?;

        count = count.saturating_add(1);
        if count > MAX_HISTORY_ROWS {
            return Err(format!(
                "History row count exceeds backup bound of {MAX_HISTORY_ROWS}"
            ));
        }

        if should_emit_step_progress(
            count,
            last_emitted_count,
            total_rows,
            HISTORY_PROGRESS_ROW_INTERVAL,
        ) {
            on_progress(count, total_rows);
            last_emitted_count = count;
        }
    }

    writer
        .flush()
        .map_err(|error| format!("Failed to flush history payload: {error}"))?;
    on_progress(count, total_rows);

    Ok(count)
}

pub(super) fn export_dictionary_payload<R: tauri::Runtime>(
    app: &AppHandle<R>,
    output_path: &Path,
) -> Result<u64, String> {
    let entries = user_dictionary::get_dictionary_snapshot(app);
    let payload = DictionaryPayload {
        version: DICTIONARY_PAYLOAD_VERSION,
        entries: entries.as_ref().clone(),
    };

    write_json_file_atomically(output_path, &payload)?;
    Ok(payload.entries.len() as u64)
}

pub(super) fn export_user_stats_payload(
    history_db_path: &Path,
    output_path: &Path,
) -> Result<bool, String> {
    if !history_db_path.exists() {
        return Ok(false);
    }

    let conn = Connection::open_with_flags(history_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Failed to open history database for user stats export: {error}"))?;

    if !sqlite_table_exists(&conn, "user_stats")? {
        return Ok(false);
    }

    let row = match conn.query_row(
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
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
            ))
        },
    ) {
        Ok(row) => row,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Failed to read canonical user stats for backup export: {error}"
            ));
        }
    };

    let transcription_dates: Vec<String> = serde_json::from_str(&row.5).unwrap_or_default();
    let payload = normalize_user_stats_payload(&UserStatsPayloadV1 {
        version: USER_STATS_PAYLOAD_VERSION,
        total_words: row.0,
        total_duration_ms: row.1,
        total_transcriptions: row.2,
        first_transcription_date: row.3,
        last_transcription_date: row.4,
        transcription_dates,
        total_filler_words_removed: row.6,
        total_speech_duration_ms: row.7,
        duration_stats_semantics_version: row.8,
    });

    validate_user_stats_payload(&payload).map_err(|error| {
        format!("Canonical user stats payload failed validation during backup export: {error}")
    })?;
    write_json_file_atomically(output_path, &payload)?;
    Ok(true)
}

pub(super) fn export_user_store_payload(app_data_dir: &Path, output_path: &Path) -> Result<(), String> {
    let source = app_data_dir.join(USER_STORE_DB_FILE);
    if source.exists() {
        copy_file_chunked(&source, output_path)?;
        return Ok(());
    }

    write_json_file_atomically(output_path, &serde_json::json!({}))
}

pub(super) fn export_recordings_payload<R: tauri::Runtime>(
    app: &AppHandle<R>,
    app_data_dir: &Path,
    workspace: &Path,
    referenced_recordings: &BTreeSet<String>,
    warnings: &mut Vec<String>,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<u64, String> {
    let recordings_root = app_data_dir.join(RECORDINGS_DIR);
    ensure_safe_recordings_root_for_backup(&recordings_root)?;
    let target_root = workspace.join(RECORDINGS_DIR);
    fs::create_dir_all(&target_root)
        .map_err(|error| format!("Failed to create recordings payload directory: {error}"))?;

    let total_recordings = referenced_recordings.len() as u64;
    let mut count = 0_u64;
    let mut processed = 0_u64;
    let mut last_emitted_processed = 0_u64;
    on_progress(0, total_recordings);

    for file_name in referenced_recordings {
        ensure_not_cancelled(app)?;
        let source = recordings_root.join(file_name);
        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) => {
                warnings.push(format!(
                    "Recording file '{file_name}' could not be read during backup: {error}"
                ));
                processed = processed.saturating_add(1);
                if should_emit_step_progress(
                    processed,
                    last_emitted_processed,
                    total_recordings,
                    RECORDINGS_PROGRESS_FILE_INTERVAL,
                ) {
                    on_progress(processed, total_recordings);
                    last_emitted_processed = processed;
                }
                continue;
            }
        };

        if metadata.file_type().is_symlink() {
            warnings.push(format!(
                "Recording file '{file_name}' is a symbolic link and was skipped during backup."
            ));
            processed = processed.saturating_add(1);
            if should_emit_step_progress(
                processed,
                last_emitted_processed,
                total_recordings,
                RECORDINGS_PROGRESS_FILE_INTERVAL,
            ) {
                on_progress(processed, total_recordings);
                last_emitted_processed = processed;
            }
            continue;
        }

        if !metadata.is_file() {
            warnings.push(format!(
                "Recording path '{file_name}' is not a regular file and was skipped during backup."
            ));
            processed = processed.saturating_add(1);
            if should_emit_step_progress(
                processed,
                last_emitted_processed,
                total_recordings,
                RECORDINGS_PROGRESS_FILE_INTERVAL,
            ) {
                on_progress(processed, total_recordings);
                last_emitted_processed = processed;
            }
            continue;
        }

        let safe_name = sanitize_relative_file_name(file_name)?;
        let relative_archive_path = format!("{RECORDINGS_DIR}/{safe_name}");
        validate_export_payload_size_from_len(&relative_archive_path, metadata.len())?;
        let destination = target_root.join(safe_name);
        copy_file_chunked_with_cancel(&source, &destination, || ensure_not_cancelled(app))?;
        count = count.saturating_add(1);
        processed = processed.saturating_add(1);
        if should_emit_step_progress(
            processed,
            last_emitted_processed,
            total_recordings,
            RECORDINGS_PROGRESS_FILE_INTERVAL,
        ) {
            on_progress(processed, total_recordings);
            last_emitted_processed = processed;
        }
    }
    on_progress(processed, total_recordings);

    Ok(count)
}

pub(super) fn validate_workspace_payload_size_limits(
    workspace: &Path,
    payload_files: &[String],
) -> Result<(), String> {
    for relative in payload_files {
        validate_export_payload_file_size(&workspace.join(relative), relative)?;
    }

    Ok(())
}

pub(super) fn write_checksums_file<F, P>(
    workspace: &Path,
    payload_files: &[String],
    mut cancel_check: F,
    mut on_progress: P,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
    P: FnMut(u64, u64),
{
    let output_path = workspace.join(CHECKSUM_FILE);
    let mut writer = BufWriter::new(
        File::create(&output_path)
            .map_err(|error| format!("Failed to create checksum file: {error}"))?,
    );

    let total_files = payload_files.len() as u64;
    let mut processed = 0_u64;
    let mut last_emitted_processed = 0_u64;
    on_progress(0, total_files);

    for relative in payload_files {
        cancel_check()?;
        let absolute = workspace.join(relative);
        let checksum = checksum_path(&absolute)?;
        writer
            .write_all(format!("{checksum}  {relative}\n").as_bytes())
            .map_err(|error| format!("Failed to write checksum line: {error}"))?;

        processed = processed.saturating_add(1);
        if should_emit_step_progress(
            processed,
            last_emitted_processed,
            total_files,
            CHECKSUM_PROGRESS_FILE_INTERVAL,
        ) {
            on_progress(processed, total_files);
            last_emitted_processed = processed;
        }
    }

    writer
        .flush()
        .map_err(|error| format!("Failed to flush checksum file: {error}"))?;
    on_progress(processed, total_files);

    Ok(())
}

pub(super) fn package_workspace_to_archive_with_cancel<F, P>(
    workspace: &Path,
    output_path: &Path,
    mut cancel_check: F,
    mut on_progress: P,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
    P: FnMut(u64, u64),
{
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create output directory: {error}"))?;
    }

    let temp_output = output_path.with_extension(format!("{}.tmp", BACKUP_FILE_EXTENSION));
    if temp_output.exists() {
        fs::remove_file(&temp_output)
            .map_err(|error| format!("Failed to remove stale temporary backup output: {error}"))?;
    }

    let package_result = (|| -> Result<(), String> {
        cancel_check()?;

        let output_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_output)
            .map_err(|error| format!("Failed to create temporary backup archive: {error}"))?;

        let mut zip = ZipWriter::new(output_file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        let mut files_to_zip = collect_payload_files(workspace)?;
        files_to_zip.push(CHECKSUM_FILE.to_string());
        files_to_zip.sort();

        let total_bytes = files_to_zip.iter().try_fold(0_u64, |acc, relative| {
            let metadata = fs::metadata(workspace.join(relative)).map_err(|error| {
                format!("Failed to inspect payload file '{relative}' for ZIP sizing: {error}")
            })?;
            Ok::<u64, String>(acc.saturating_add(metadata.len()))
        })?;
        let mut bytes_written = 0_u64;
        let mut last_emitted_bytes = 0_u64;
        let mut last_emitted_at = std::time::Instant::now();
        on_progress(0, total_bytes);

        let mut buffer = vec![0_u8; 1024 * 1024];

        for relative in files_to_zip {
            cancel_check()?;

            let absolute = workspace.join(&relative);
            let mut source = File::open(&absolute).map_err(|error| {
                format!("Failed to open payload file '{relative}' for ZIP: {error}")
            })?;

            zip.start_file(relative.clone(), options)
                .map_err(|error| format!("Failed to start ZIP entry '{relative}': {error}"))?;

            loop {
                cancel_check()?;

                let read = source.read(&mut buffer).map_err(|error| {
                    format!("Failed to read payload file '{relative}': {error}")
                })?;
                if read == 0 {
                    break;
                }
                zip.write_all(&buffer[..read])
                    .map_err(|error| format!("Failed to write ZIP entry '{relative}': {error}"))?;

                bytes_written = bytes_written.saturating_add(read as u64);
                let now = std::time::Instant::now();
                let should_emit = bytes_written == total_bytes
                    || bytes_written.saturating_sub(last_emitted_bytes)
                        >= PACKAGE_PROGRESS_MIN_EMIT_BYTES
                    || now.saturating_duration_since(last_emitted_at)
                        >= PACKAGE_PROGRESS_MAX_EMIT_INTERVAL;
                if should_emit {
                    on_progress(bytes_written, total_bytes);
                    last_emitted_bytes = bytes_written;
                    last_emitted_at = now;
                }
            }
        }

        let mut finished = zip
            .finish()
            .map_err(|error| format!("Failed to finalize backup archive: {error}"))?;
        finished
            .flush()
            .map_err(|error| format!("Failed to flush backup archive: {error}"))?;
        finished
            .sync_all()
            .map_err(|error| format!("Failed to sync backup archive: {error}"))?;

        drop(finished);
        cancel_check()?;

        fs::rename(&temp_output, output_path)
            .map_err(|error| format!("Failed to move backup archive into place: {error}"))?;

        fsync_parent(output_path)
            .map_err(|error| format!("Failed to sync backup archive parent directory: {error}"))?;

        on_progress(total_bytes, total_bytes);

        Ok(())
    })();

    if package_result.is_err() && temp_output.exists() {
        if let Err(error) = fs::remove_file(&temp_output) {
            warn!(
                error = %error,
                path = %temp_output.display(),
                "Failed to remove temporary backup archive after package failure"
            );
        }
    }

    package_result
}
