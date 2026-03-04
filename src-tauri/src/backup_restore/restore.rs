//! Restore apply and startup reconciliation flows.
//!
//! Handles staged import/swap/rollback behavior, marker-based crash recovery,
//! and restore runtime artifact cleanup.

use super::*;

pub(super) const MANUAL_STATS_REPAIR_ENV_VAR: &str = "HANDY_MANUAL_STATS_REPAIR_20260303";
pub(super) const MANUAL_STATS_REPAIR_BAD_WORDS: i64 = 46_208;
pub(super) const MANUAL_STATS_REPAIR_BAD_DURATION_MS: i64 = 22_697_871;
pub(super) const MANUAL_STATS_REPAIR_BAD_SPEECH_MS: i64 = 3_202_691;
pub(super) const MANUAL_STATS_REPAIR_TARGET_WORDS: i64 = 43_604;
pub(super) const MANUAL_STATS_REPAIR_TARGET_DURATION_MS: i64 = 22_720_677;
pub(super) const MANUAL_STATS_REPAIR_TARGET_SPEECH_MS: i64 = 22_618_064;

fn should_surface_recoverable_warning_during_apply(code: &str) -> bool {
    // Extension mismatch is surfaced during preflight context and intentionally
    // hidden in UI detail lists; keep apply-success warnings aligned with that.
    code != "archive_extension_unexpected"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RestoreStatsSource {
    FallbackRecompute,
    CanonicalPayload,
}

impl RestoreStatsSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::FallbackRecompute => "fallback_recompute",
            Self::CanonicalPayload => "canonical_payload",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct HistoryImportSummary {
    pub row_count: u64,
    pub zero_speech_duration_rows: u64,
    pub recomputed_stats: UserStatsPayloadV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManualStatsRepairReport {
    pub applied: bool,
    pub reason: &'static str,
}

fn env_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn user_stats_table_exists(conn: &Connection) -> Result<bool, String> {
    conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='user_stats'",
        [],
        |row| row.get::<_, bool>(0),
    )
    .map_err(|error| format!("Failed to inspect user_stats table availability: {error}"))
}

pub(super) fn maybe_run_manual_stats_repair(
    app_data_dir: &Path,
) -> Result<Option<ManualStatsRepairReport>, String> {
    let should_repair = std::env::var(MANUAL_STATS_REPAIR_ENV_VAR)
        .ok()
        .map(|value| env_truthy(&value))
        .unwrap_or(false);
    if !should_repair {
        return Ok(None);
    }

    let history_db_path = app_data_dir.join(HISTORY_DB_FILE);
    if !history_db_path.exists() {
        info!(
            event_code = "restore_stats_manual_repair",
            outcome = "skipped",
            reason = "history_db_missing",
            "Skipped manual restore-stats repair because history DB was not found"
        );
        return Ok(Some(ManualStatsRepairReport {
            applied: false,
            reason: "history_db_missing",
        }));
    }

    let conn = Connection::open(&history_db_path)
        .map_err(|error| format!("Failed to open history DB for manual stats repair: {error}"))?;
    if !user_stats_table_exists(&conn)? {
        info!(
            event_code = "restore_stats_manual_repair",
            outcome = "skipped",
            reason = "user_stats_table_missing",
            "Skipped manual restore-stats repair because user_stats table was not found"
        );
        return Ok(Some(ManualStatsRepairReport {
            applied: false,
            reason: "user_stats_table_missing",
        }));
    }

    let (total_words, total_duration_ms, total_speech_duration_ms, semantics_version): (
        i64,
        i64,
        i64,
        i64,
    ) = match conn.query_row(
        "SELECT
            COALESCE(total_words, 0),
            COALESCE(total_duration_ms, 0),
            COALESCE(total_speech_duration_ms, 0),
            COALESCE(duration_stats_semantics_version, 0)
         FROM user_stats
         WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ) {
        Ok(values) => values,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            info!(
                event_code = "restore_stats_manual_repair",
                outcome = "skipped",
                reason = "user_stats_row_missing",
                "Skipped manual restore-stats repair because user_stats row id=1 was not found"
            );
            return Ok(Some(ManualStatsRepairReport {
                applied: false,
                reason: "user_stats_row_missing",
            }));
        }
        Err(error) => {
            return Err(format!(
                "Failed to read current stats for manual repair: {error}"
            ));
        }
    };

    let guard_matches = total_words == MANUAL_STATS_REPAIR_BAD_WORDS
        && total_duration_ms == MANUAL_STATS_REPAIR_BAD_DURATION_MS
        && total_speech_duration_ms == MANUAL_STATS_REPAIR_BAD_SPEECH_MS
        && semantics_version == 1;

    if !guard_matches {
        info!(
            event_code = "restore_stats_manual_repair",
            outcome = "skipped",
            reason = "guard_mismatch",
            total_words,
            total_duration_ms,
            total_speech_duration_ms,
            semantics_version,
            "Skipped manual restore-stats repair because guard values did not match"
        );
        return Ok(Some(ManualStatsRepairReport {
            applied: false,
            reason: "guard_mismatch",
        }));
    }

    conn.execute(
        "UPDATE user_stats
         SET total_words = ?1,
             total_duration_ms = ?2,
             total_speech_duration_ms = ?3,
             duration_stats_semantics_version = 1
         WHERE id = 1",
        params![
            MANUAL_STATS_REPAIR_TARGET_WORDS,
            MANUAL_STATS_REPAIR_TARGET_DURATION_MS,
            MANUAL_STATS_REPAIR_TARGET_SPEECH_MS,
        ],
    )
    .map_err(|error| format!("Failed to apply manual restore-stats repair update: {error}"))?;

    info!(
        event_code = "restore_stats_manual_repair",
        outcome = "applied",
        old_total_words = MANUAL_STATS_REPAIR_BAD_WORDS,
        old_total_duration_ms = MANUAL_STATS_REPAIR_BAD_DURATION_MS,
        old_total_speech_duration_ms = MANUAL_STATS_REPAIR_BAD_SPEECH_MS,
        new_total_words = MANUAL_STATS_REPAIR_TARGET_WORDS,
        new_total_duration_ms = MANUAL_STATS_REPAIR_TARGET_DURATION_MS,
        new_total_speech_duration_ms = MANUAL_STATS_REPAIR_TARGET_SPEECH_MS,
        "Applied manual restore-stats repair update"
    );

    Ok(Some(ManualStatsRepairReport {
        applied: true,
        reason: "guard_match",
    }))
}

pub fn reconcile_startup<R: tauri::Runtime>(app: &AppHandle<R>) {
    if ensure_supported_platform().is_err() {
        return;
    }

    let Ok(app_data_dir) = app_data_dir(app) else {
        return;
    };

    prune_expired_checkpoint(&app_data_dir);
    prune_stale_runtime_artifacts(&app_data_dir);
    if let Err(error) = maybe_run_manual_stats_repair(&app_data_dir) {
        warn!(
            error = %error,
            "Failed to evaluate one-time manual restore-stats repair"
        );
    }

    let marker_path = marker_path(&app_data_dir);
    if !marker_path.exists() {
        return;
    }

    let marker = match read_json_file::<RestoreMarker>(&marker_path) {
        Ok(marker) => marker,
        Err(error) => {
            warn!(
                error = %error,
                "Failed to read restore marker at startup; removing marker"
            );
            let _ = remove_file_with_parent_sync(&marker_path);
            return;
        }
    };

    if marker.state == "in_progress" {
        let snapshot = match validated_runtime_snapshot_path(&app_data_dir, &marker.snapshot_path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                warn!(
                    error = %error,
                    snapshot_path = %marker.snapshot_path,
                    "Restore marker snapshot path is invalid; removing stale marker"
                );
                let _ = remove_file_with_parent_sync(&marker_path);
                return;
            }
        };

        let Some(snapshot_layout) = marker.snapshot_layout.as_ref() else {
            error!(
                snapshot_path = %marker.snapshot_path,
                "Restore marker is missing snapshot layout metadata; leaving marker in place and active data untouched"
            );
            return;
        };

        if let Err(error) = validate_restore_marker_snapshot(&snapshot, snapshot_layout) {
            error!(
                error = %error,
                snapshot_path = %snapshot.display(),
                "Restore marker snapshot failed integrity validation; leaving marker in place and active data untouched"
            );
            return;
        }

        match replace_active_data_from_source(&snapshot, snapshot_layout, &app_data_dir) {
            Ok(()) => {
                info!("Rolled back uncommitted restore during startup reconciliation");

                if let Err(error) = user_dictionary::reload_dictionary_state(app) {
                    warn!(
                        error = %error,
                        "Failed to refresh in-memory dictionary during startup reconciliation"
                    );
                }
                reload_user_store_state(app);
                let _ = app.emit("history-updated", ());
                let _ = app.emit("dictionary-updated", ());
                let _ = app.emit("user-profile-updated", ());

                let _ = remove_file_with_parent_sync(&marker_path);
                if let Err(error) = remove_snapshot_dir(&snapshot) {
                    warn!(
                        error = %error,
                        path = %snapshot.display(),
                        "Startup rollback succeeded, but snapshot cleanup failed"
                    );
                }
            }
            Err(error) => {
                error!(
                    error = %error,
                    "Failed to rollback uncommitted restore from startup marker; keeping marker for retry"
                );
            }
        }
    } else if marker.state == "committed" {
        info!("Found committed restore marker during startup; keeping restored data");
        let _ = remove_file_with_parent_sync(&marker_path);
    } else {
        warn!(state = %marker.state, "Unknown restore marker state; removing marker");
        let _ = remove_file_with_parent_sync(&marker_path);
    }
}
pub fn apply_restore<R: tauri::Runtime>(
    app: &AppHandle<R>,
    request: ApplyRestoreRequest,
) -> Result<ApplyRestoreReport, String> {
    ensure_supported_platform()?;
    let _op_guard = start_operation(app)?;
    emit_progress(app, "restore", "preflight", 1, 8);

    ensure_not_cancelled(app)?;
    let archive_path = PathBuf::from(request.archive_path);
    // Use a single opened archive handle for preflight and extraction to avoid
    // path-based TOCTOU replacement between validation and use.
    let mut archive_file = open_archive_for_restore(&archive_path)?;
    let preflight = build_preflight_context_with_open_archive(app, &archive_path, &mut archive_file)?;

    if !preflight.report.can_apply {
        let reasons = preflight
            .report
            .blocking_findings
            .iter()
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "Restore cannot continue because preflight found blocking issues: {reasons}"
        ));
    }

    let app_data_dir = app_data_dir(app)?;
    let runtime_dir = runtime_dir(&app_data_dir);
    fs::create_dir_all(&runtime_dir)
        .map_err(|error| format!("Failed to create restore runtime directory: {error}"))?;

    let work_root = runtime_dir.join(format!("restore-work-{}", timestamp_millis()));
    let extract_dir = work_root.join("extracted");
    let new_data_dir = work_root.join("new-data");

    fs::create_dir_all(&extract_dir)
        .map_err(|error| format!("Failed to create restore extraction directory: {error}"))?;
    fs::create_dir_all(&new_data_dir)
        .map_err(|error| format!("Failed to create restore staging directory: {error}"))?;

    let restore_result = (|| -> Result<ApplyRestoreReport, String> {
        ensure_not_cancelled(app)?;

        emit_progress(app, "restore", "extract", 2, 8);
        extract_archive(app, &mut archive_file, &extract_dir)?;

        ensure_not_cancelled(app)?;

        emit_progress(app, "restore", "import-history", 3, 8);
        prepare_staged_history_db(&app_data_dir, &new_data_dir)?;
        let history_import_summary = import_history_jsonl(
            &extract_dir.join(HISTORY_FILE),
            &new_data_dir.join(HISTORY_DB_FILE),
            app,
        )?;

        ensure_not_cancelled(app)?;

        emit_progress(app, "restore", "import-dictionary", 4, 8);
        let dictionary_entries = import_dictionary_payload(
            &extract_dir.join(DICTIONARY_FILE),
            &new_data_dir.join(USER_DICTIONARY_FILE),
        )?;

        emit_progress(app, "restore", "import-user-store", 5, 8);
        let mut warnings = preflight
            .report
            .recoverable_findings
            .iter()
            .filter(|finding| should_surface_recoverable_warning_during_apply(&finding.code))
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        import_user_store_payload(
            &extract_dir.join(USER_STORE_FILE),
            &app_data_dir.join(USER_STORE_DB_FILE),
            &new_data_dir.join(USER_STORE_DB_FILE),
            &mut warnings,
        )?;

        let stats_source = import_user_stats_payload(
            &extract_dir.join(HISTORY_USER_STATS_FILE),
            &new_data_dir.join(HISTORY_DB_FILE),
            &mut warnings,
        )?;
        let imported_stats = read_user_stats_payload_from_history_db(&new_data_dir.join(HISTORY_DB_FILE))?;
        info!(
            event_code = "restore_stats_import_summary",
            stats_source = stats_source.as_str(),
            history_rows = history_import_summary.row_count,
            zero_speech_duration_rows = history_import_summary.zero_speech_duration_rows,
            recomputed_total_words = history_import_summary.recomputed_stats.total_words,
            recomputed_total_duration_ms = history_import_summary.recomputed_stats.total_duration_ms,
            recomputed_total_speech_duration_ms = history_import_summary
                .recomputed_stats
                .total_speech_duration_ms,
            total_words = imported_stats.total_words,
            total_duration_ms = imported_stats.total_duration_ms,
            total_speech_duration_ms = imported_stats.total_speech_duration_ms,
            total_transcriptions = imported_stats.total_transcriptions,
            total_filler_words_removed = imported_stats.total_filler_words_removed,
            duration_stats_semantics_version = imported_stats.duration_stats_semantics_version,
            "Restore history stats import summary"
        );

        emit_progress(app, "restore", "import-recordings", 6, 8);
        import_recordings_payload(app, &extract_dir, &new_data_dir)?;

        ensure_not_cancelled(app)?;

        validate_staged_history_db(&new_data_dir.join(HISTORY_DB_FILE))?;

        emit_progress(app, "restore", "swap", 7, 8);
        let snapshot_path = create_snapshot(&app_data_dir, &runtime_dir)?;
        let snapshot_layout = build_restore_marker_snapshot_layout(&snapshot_path)?;
        let marker_path = marker_path(&app_data_dir);

        durable_write_json(
            &marker_path,
            &RestoreMarker {
                state: "in_progress".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: Some(snapshot_layout.clone()),
            },
        )?;

        if let Err(error) = perform_staged_swap(&app_data_dir, &new_data_dir, &runtime_dir) {
            return rollback_after_swap_failure(
                &app_data_dir,
                &marker_path,
                &snapshot_path,
                &snapshot_layout,
                "Restore failed during swap",
                &error.to_string(),
            );
        }

        #[cfg(test)]
        if should_failpoint("restore_after_swap_before_commit") {
            return rollback_after_swap_failure(
                &app_data_dir,
                &marker_path,
                &snapshot_path,
                &snapshot_layout,
                "Restore failed after swap while finalizing commit",
                "Injected failpoint: restore_after_swap_before_commit",
            );
        }

        if let Err(error) = durable_write_json(
            &marker_path,
            &RestoreMarker {
                state: "committed".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: Some(snapshot_layout.clone()),
            },
        ) {
            return rollback_after_swap_failure(
                &app_data_dir,
                &marker_path,
                &snapshot_path,
                &snapshot_layout,
                "Restore failed after swap while persisting commit marker",
                &error,
            );
        }

        if let Err(error) = publish_undo_checkpoint(&app_data_dir, &snapshot_path) {
            warn!(
                error = %error,
                "Restore committed but undo checkpoint could not be published"
            );
            warnings.push(format!(
                "Restore completed, but Undo Last Restore checkpoint could not be created: {error}"
            ));
        }

        if let Err(error) = remove_file_with_parent_sync(&marker_path) {
            warn!(
                error = %error,
                path = %marker_path.display(),
                "Failed to remove committed restore marker; startup will reconcile"
            );
        }

        // Keep dictionary in-memory state aligned with the restored file.
        if let Err(error) = user_dictionary::reload_dictionary_state(app) {
            warn!(error = %error, "Failed to refresh in-memory dictionary after restore");
        }
        reload_user_store_state(app);

        let _ = app.emit("history-updated", ());
        let _ = app.emit("dictionary-updated", ());
        let _ = app.emit("user-profile-updated", ());

        emit_progress(app, "restore", "finalize", 8, 8);

        Ok(ApplyRestoreReport {
            warnings,
            counts: BackupCounts {
                history_entries: history_import_summary.row_count,
                recording_files: count_files(&app_data_dir.join(RECORDINGS_DIR)).unwrap_or(0),
                dictionary_entries: dictionary_entries as u64,
            },
        })
    })();

    if let Err(error) = fs::remove_dir_all(&work_root) {
        warn!(
            error = %error,
            path = %work_root.display(),
            "Failed to clean restore work directory"
        );
    }
    prune_orphan_snapshot_dirs_keep_latest(&app_data_dir);

    restore_result
}
pub(super) fn rollback_after_swap_failure<T>(
    app_data_dir: &Path,
    marker_path: &Path,
    snapshot_path: &Path,
    snapshot_layout: &UndoCheckpointSnapshotLayout,
    context: &str,
    cause: &str,
) -> Result<T, String> {
    error!(context = context, cause = cause, "Rolling back from restore snapshot");

    match replace_active_data_from_source(snapshot_path, snapshot_layout, app_data_dir) {
        Ok(()) => {
            if let Err(error) = remove_file_with_parent_sync(marker_path) {
                warn!(
                    error = %error,
                    path = %marker_path.display(),
                    "Rollback succeeded but marker cleanup failed; startup will reconcile"
                );
            }
            if let Err(error) = remove_snapshot_dir(snapshot_path) {
                warn!(
                    error = %error,
                    path = %snapshot_path.display(),
                    "Rollback succeeded but snapshot cleanup failed; stale data will be pruned later"
                );
            }
            Err(format!("{context} and was rolled back: {cause}"))
        }
        Err(rollback_error) => Err(format!(
            "{context}, rollback failed, and recovery marker was kept for startup reconciliation: {cause}; rollback error: {rollback_error}"
        )),
    }
}

pub(super) fn extract_archive<R: tauri::Runtime>(
    app: &AppHandle<R>,
    archive_file: &mut File,
    destination: &Path,
) -> Result<(), String> {
    archive_file
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("Failed to rewind backup archive before extraction: {error}"))?;

    let mut archive = ZipArchive::new(&mut *archive_file)
        .map_err(|error| format!("Failed to parse backup archive for extraction: {error}"))?;
    let mut total_extracted_bytes = 0_u64;

    for index in 0..archive.len() {
        ensure_not_cancelled(app)?;
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read archive entry #{index}: {error}"))?;

        let normalized = normalize_archive_path(entry.name())
            .map_err(|error| format!("Unsafe archive path '{}': {error}", entry.name()))?;

        if entry.is_dir() {
            fs::create_dir_all(destination.join(&normalized)).map_err(|error| {
                format!("Failed to create extracted directory '{normalized}': {error}")
            })?;
            continue;
        }

        if is_link_entry(&entry) {
            return Err(format!(
                "Archive entry '{}' is a symlink/hardlink and cannot be extracted",
                entry.name()
            ));
        }

        let output_path = destination.join(&normalized);
        if output_path.strip_prefix(destination).is_err() {
            return Err(format!(
                "Archive entry '{}' resolved outside extraction destination",
                entry.name()
            ));
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create extracted file parent directory '{}': {error}",
                    parent.display()
                )
            })?;
        }

        extract_entry_bounded(
            app,
            &mut entry,
            &output_path,
            &normalized,
            &mut total_extracted_bytes,
        )?;
    }

    Ok(())
}

#[cfg(not(test))]
fn extract_payload_size_limit_bytes() -> u64 {
    MAX_PAYLOAD_FILE_SIZE_BYTES
}

#[cfg(test)]
fn extract_payload_size_limit_bytes() -> u64 {
    std::env::var("HANDY_TEST_BR_MAX_EXTRACT_PAYLOAD_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(MAX_PAYLOAD_FILE_SIZE_BYTES)
}

#[cfg(not(test))]
fn extract_total_uncompressed_limit_bytes() -> u64 {
    MAX_TOTAL_UNCOMPRESSED_BYTES
}

#[cfg(test)]
fn extract_total_uncompressed_limit_bytes() -> u64 {
    std::env::var("HANDY_TEST_BR_MAX_EXTRACT_TOTAL_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(MAX_TOTAL_UNCOMPRESSED_BYTES)
}

pub(super) fn extract_entry_bounded<Rt: tauri::Runtime, Re: Read>(
    app: &AppHandle<Rt>,
    entry: &mut Re,
    output_path: &Path,
    archive_relative_path: &str,
    total_extracted_bytes: &mut u64,
) -> Result<(), String> {
    extract_entry_bounded_with_cancel(
        entry,
        output_path,
        archive_relative_path,
        total_extracted_bytes,
        || ensure_not_cancelled(app),
    )
}

pub(super) fn extract_entry_bounded_with_cancel<Re: Read, F>(
    entry: &mut Re,
    output_path: &Path,
    archive_relative_path: &str,
    total_extracted_bytes: &mut u64,
    mut cancel_check: F,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .map_err(|error| {
            format!(
                "Failed to create extracted payload file '{}': {error}",
                output_path.display()
            )
        })?;

    let mut entry_extracted_bytes = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    let payload_limit = extract_payload_size_limit_bytes();
    let total_limit = extract_total_uncompressed_limit_bytes();

    cancel_check()?;
    loop {
        cancel_check()?;
        let read = entry.read(&mut buffer).map_err(|error| {
            format!(
                "Failed to read archive payload entry '{archive_relative_path}': {error}"
            )
        })?;
        if read == 0 {
            break;
        }

        let chunk_len = read as u64;
        entry_extracted_bytes = entry_extracted_bytes.saturating_add(chunk_len);
        if entry_extracted_bytes > payload_limit {
            drop(output);
            cleanup_partial_extracted_file(output_path);
            return Err(format!(
                "archive_payload_size_limit_extracted:{archive_relative_path}:{entry_extracted_bytes}>{payload_limit}"
            ));
        }

        *total_extracted_bytes = total_extracted_bytes.saturating_add(chunk_len);
        if *total_extracted_bytes > total_limit {
            drop(output);
            cleanup_partial_extracted_file(output_path);
            return Err(format!(
                "archive_uncompressed_limit_extracted:{}>{}",
                *total_extracted_bytes, total_limit
            ));
        }

        output.write_all(&buffer[..read]).map_err(|error| {
            format!(
                "Failed to extract payload file '{}': {error}",
                output_path.display()
            )
        })?;
    }

    output
        .flush()
        .map_err(|error| format!("Failed to flush extracted file: {error}"))?;

    Ok(())
}

fn cleanup_partial_extracted_file(output_path: &Path) {
    if let Err(error) = fs::remove_file(output_path) {
        warn!(
            error = %error,
            path = %output_path.display(),
            "Failed to remove partially extracted file after limit breach"
        );
    }
}

pub(super) fn prepare_staged_history_db(app_data_dir: &Path, new_data_dir: &Path) -> Result<(), String> {
    let active_history_db = app_data_dir.join(HISTORY_DB_FILE);
    let staged_history_db = new_data_dir.join(HISTORY_DB_FILE);

    if active_history_db.exists() {
        copy_file_chunked(&active_history_db, &staged_history_db)?;
    } else {
        initialize_history_db(&staged_history_db)?;
    }

    Ok(())
}

pub(super) fn initialize_history_db(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create history DB parent directory: {error}"))?;
    }

    let conn = Connection::open(path)
        .map_err(|error| format!("Failed to create staged history database: {error}"))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL,
            post_processed_text TEXT,
            post_process_prompt TEXT,
            duration_ms INTEGER DEFAULT 0,
            inserted_text TEXT,
            speech_duration_ms INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS user_stats (
            id INTEGER PRIMARY KEY DEFAULT 1,
            total_words INTEGER DEFAULT 0,
            total_duration_ms INTEGER DEFAULT 0,
            total_transcriptions INTEGER DEFAULT 0,
            first_transcription_date INTEGER,
            last_transcription_date INTEGER,
            transcription_dates TEXT DEFAULT '[]',
            total_filler_words_removed INTEGER DEFAULT 0,
            total_speech_duration_ms INTEGER DEFAULT 0,
            duration_stats_semantics_version INTEGER DEFAULT 1
        );
        ",
    )
    .map_err(|error| format!("Failed to initialize staged history schema: {error}"))?;

    Ok(())
}

pub(super) fn import_history_jsonl<R: tauri::Runtime>(
    jsonl_path: &Path,
    staged_history_db: &Path,
    app: &AppHandle<R>,
) -> Result<HistoryImportSummary, String> {
    let mut conn = Connection::open(staged_history_db)
        .map_err(|error| format!("Failed to open staged history DB for import: {error}"))?;

    let tx = conn
        .transaction()
        .map_err(|error| format!("Failed to open staged history transaction: {error}"))?;

    tx.execute("DELETE FROM transcription_history", [])
        .map_err(|error| format!("Failed to clear staged history entries: {error}"))?;
    tx.execute("DELETE FROM user_stats", [])
        .map_err(|error| format!("Failed to clear staged user_stats: {error}"))?;

    let input = BufReader::new(
        File::open(jsonl_path)
            .map_err(|error| format!("Failed to open history payload JSONL: {error}"))?,
    );

    let mut row_count = 0_u64;
    let mut total_words = 0_i64;
    let mut total_duration_ms = 0_i64;
    let mut total_speech_duration_ms = 0_i64;
    let mut zero_speech_duration_rows = 0_u64;
    let mut first_timestamp: Option<i64> = None;
    let mut last_timestamp: Option<i64> = None;
    let mut date_keys = BTreeSet::new();

    let mut input = input;
    while let Some(line) = read_history_jsonl_line_bounded(&mut input)? {
        ensure_not_cancelled(app)?;
        if line.trim().is_empty() {
            continue;
        }

        let row: HistoryRowV1 = serde_json::from_str(&line)
            .map_err(|error| format!("Invalid history JSONL row: {error}"))?;
        let safe_file_name = sanitize_relative_file_name(&row.file_name).map_err(|error| {
            format!(
                "Invalid history JSONL row file_name '{}': {error}",
                row.file_name
            )
        })?;

        tx.execute(
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
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                row.id,
                safe_file_name,
                row.timestamp,
                row.saved,
                row.title,
                row.transcription_text,
                row.post_processed_text,
                row.post_process_prompt,
                row.duration_ms,
                row.inserted_text,
                row.speech_duration_ms,
            ],
        )
        .map_err(|error| format!("Failed to insert staged history row: {error}"))?;

        let stats_text = row
            .post_processed_text
            .as_deref()
            .unwrap_or(row.transcription_text.as_str());
        total_words = total_words
            .saturating_add(crate::audio_toolkit::text::count_words(stats_text) as i64);

        let (normalized_duration, normalized_speech, speech_was_missing) =
            normalize_restore_duration_pair(row.duration_ms, row.speech_duration_ms);
        if speech_was_missing {
            zero_speech_duration_rows = zero_speech_duration_rows.saturating_add(1);
        }

        total_duration_ms = total_duration_ms.saturating_add(normalized_duration);
        total_speech_duration_ms = total_speech_duration_ms.saturating_add(normalized_speech);

        first_timestamp = Some(first_timestamp.map_or(row.timestamp, |current| current.min(row.timestamp)));
        last_timestamp = Some(last_timestamp.map_or(row.timestamp, |current| current.max(row.timestamp)));

        if let Some(dt_utc) = DateTime::<Utc>::from_timestamp(row.timestamp, 0) {
            let date_key = dt_utc.with_timezone(&Local).format("%Y-%m-%d").to_string();
            date_keys.insert(date_key);
        }

        row_count = row_count.saturating_add(1);
        if row_count > MAX_HISTORY_ROWS {
            return Err(format!(
                "History row count exceeds supported bound ({} > {})",
                row_count, MAX_HISTORY_ROWS
            ));
        }
    }

    let transcription_dates = date_keys.iter().cloned().collect::<Vec<_>>();

    let recomputed_stats = normalize_user_stats_payload(&UserStatsPayloadV1 {
        version: USER_STATS_PAYLOAD_VERSION,
        total_words,
        total_duration_ms,
        total_transcriptions: row_count as i64,
        first_transcription_date: first_timestamp,
        last_transcription_date: last_timestamp,
        transcription_dates,
        total_filler_words_removed: 0,
        total_speech_duration_ms,
        duration_stats_semantics_version: 1,
    });
    validate_user_stats_payload(&recomputed_stats).map_err(|error| {
        format!("Recomputed user stats failed validation during restore import: {error}")
    })?;

    tx.execute(
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
            recomputed_stats.total_words,
            recomputed_stats.total_duration_ms,
            recomputed_stats.total_transcriptions,
            recomputed_stats.first_transcription_date,
            recomputed_stats.last_transcription_date,
            serde_json::to_string(&recomputed_stats.transcription_dates)
                .map_err(|error| format!("Failed to serialize recomputed transcription dates: {error}"))?,
            recomputed_stats.total_filler_words_removed,
            recomputed_stats.total_speech_duration_ms,
            recomputed_stats.duration_stats_semantics_version,
        ],
    )
    .map_err(|error| format!("Failed to persist recomputed user_stats: {error}"))?;

    tx.commit()
        .map_err(|error| format!("Failed to commit staged history import transaction: {error}"))?;

    Ok(HistoryImportSummary {
        row_count,
        zero_speech_duration_rows,
        recomputed_stats,
    })
}

fn normalize_restore_duration_pair(
    recording_duration_ms: i64,
    speech_duration_ms: i64,
) -> (i64, i64, bool) {
    let mut normalized_recording = recording_duration_ms.max(0);
    let mut normalized_speech = speech_duration_ms.max(0);
    let speech_was_missing = normalized_speech == 0 && normalized_recording > 0;

    // Preserve runtime behavior when only speech duration exists.
    if normalized_recording == 0 && normalized_speech > 0 {
        normalized_recording = normalized_speech;
    }

    if normalized_speech > 0 {
        normalized_speech = normalized_speech.min(normalized_recording);
    } else {
        // Legacy fallback: historical rows often had speech=0.
        normalized_speech = normalized_recording;
    }

    (normalized_recording, normalized_speech, speech_was_missing)
}

fn push_warning_once(warnings: &mut Vec<String>, warning: String) {
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

pub(super) fn import_user_stats_payload(
    payload_path: &Path,
    staged_history_db: &Path,
    warnings: &mut Vec<String>,
) -> Result<RestoreStatsSource, String> {
    if !payload_path.exists() {
        return Ok(RestoreStatsSource::FallbackRecompute);
    }

    let payload = match read_json_file::<UserStatsPayloadV1>(payload_path) {
        Ok(payload) => payload,
        Err(error) => {
            push_warning_once(
                warnings,
                format!(
                    "{HISTORY_USER_STATS_FILE} was malformed and productivity stats were recomputed: {error}"
                ),
            );
            return Ok(RestoreStatsSource::FallbackRecompute);
        }
    };

    if let Err(error) = validate_user_stats_payload(&payload) {
        push_warning_once(
            warnings,
            format!(
                "{HISTORY_USER_STATS_FILE} had invalid structure and productivity stats were recomputed: {error}"
            ),
        );
        return Ok(RestoreStatsSource::FallbackRecompute);
    }
    let payload = normalize_user_stats_payload(&payload);
    let transcription_dates_json = serde_json::to_string(&payload.transcription_dates)
        .map_err(|error| format!("Failed to serialize canonical transcription dates: {error}"))?;

    let mut conn = Connection::open(staged_history_db)
        .map_err(|error| format!("Failed to open staged history DB for user stats import: {error}"))?;
    let tx = conn
        .transaction()
        .map_err(|error| format!("Failed to start staged user stats import transaction: {error}"))?;
    tx.execute("DELETE FROM user_stats", [])
        .map_err(|error| format!("Failed to clear staged user_stats for canonical import: {error}"))?;
    tx.execute(
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
            payload.total_words,
            payload.total_duration_ms,
            payload.total_transcriptions,
            payload.first_transcription_date,
            payload.last_transcription_date,
            transcription_dates_json,
            payload.total_filler_words_removed,
            payload.total_speech_duration_ms,
            payload.duration_stats_semantics_version,
        ],
    )
    .map_err(|error| format!("Failed to import canonical user_stats payload: {error}"))?;
    tx.commit()
        .map_err(|error| format!("Failed to commit canonical user_stats import: {error}"))?;

    Ok(RestoreStatsSource::CanonicalPayload)
}

pub(super) fn read_user_stats_payload_from_history_db(
    history_db_path: &Path,
) -> Result<UserStatsPayloadV1, String> {
    let conn = Connection::open(history_db_path)
        .map_err(|error| format!("Failed to open history DB for stats snapshot read: {error}"))?;
    let payload = conn
        .query_row(
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
                Ok(UserStatsPayloadV1 {
                    version: USER_STATS_PAYLOAD_VERSION,
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
        .map_err(|error| format!("Failed to read staged user stats snapshot: {error}"))?;

    let payload = normalize_user_stats_payload(&payload);
    validate_user_stats_payload(&payload)
        .map_err(|error| format!("Staged user stats snapshot failed validation: {error}"))?;
    Ok(payload)
}

pub(super) fn validate_staged_history_db(path: &Path) -> Result<(), String> {
    let conn = Connection::open(path)
        .map_err(|error| format!("Failed to open staged history DB for validation: {error}"))?;

    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| format!("Failed to run history integrity check: {error}"))?;

    if quick_check.trim().to_lowercase() != "ok" {
        return Err(format!(
            "Staged history DB integrity check failed: {quick_check}"
        ));
    }

    Ok(())
}

pub(super) fn import_dictionary_payload(payload_path: &Path, destination_path: &Path) -> Result<usize, String> {
    let payload: DictionaryPayload = serde_json::from_reader(
        File::open(payload_path)
            .map_err(|error| format!("Failed to open dictionary payload for restore: {error}"))?,
    )
    .map_err(|error| format!("Failed to parse dictionary payload for restore: {error}"))?;
    let entry_count = payload.entries.len();

    write_json_file_atomically(
        destination_path,
        &DictionaryPayload {
            version: DICTIONARY_PAYLOAD_VERSION,
            entries: payload.entries,
        },
    )?;

    Ok(entry_count)
}

pub(super) fn import_user_store_payload(
    payload_path: &Path,
    active_user_store_path: &Path,
    destination_path: &Path,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if payload_path.exists() {
        match read_json_file::<serde_json::Value>(payload_path) {
            Ok(value) => {
                match validate_user_store_payload_shape(&value) {
                    Ok(()) => {
                        write_json_file_atomically(destination_path, &value)?;
                        return Ok(());
                    }
                    Err(error) => {
                        warnings.push(format!(
                            "user/user_store.json had invalid structure in backup and local fallback/defaults were used: {error}"
                        ));
                    }
                }
            }
            Err(error) => {
                warnings.push(format!(
                    "user/user_store.json was malformed in backup and local fallback/defaults were used: {error}"
                ));
            }
        }
    } else {
        warnings.push(
            "user/user_store.json was missing in backup and local fallback/defaults were used."
                .to_string(),
        );
    }

    if active_user_store_path.exists() {
        match read_json_file::<serde_json::Value>(active_user_store_path) {
            Ok(value) => match validate_user_store_payload_shape(&value) {
                Ok(()) => {
                    write_json_file_atomically(destination_path, &value)?;
                    return Ok(());
                }
                Err(error) => {
                    warnings.push(format!(
                        "Local user_store.json fallback had invalid structure and defaults were used: {error}"
                    ));
                }
            },
            Err(error) => {
                warnings.push(format!(
                    "Local user_store.json fallback could not be read and defaults were used: {error}"
                ));
            }
        }
    }

    write_json_file_atomically(destination_path, &serde_json::json!({}))
}

pub(super) fn import_recordings_payload<R: tauri::Runtime>(
    app: &AppHandle<R>,
    extract_dir: &Path,
    new_data_dir: &Path,
) -> Result<(), String> {
    ensure_not_cancelled(app)?;

    let source_recordings = extract_dir.join(RECORDINGS_DIR);
    let destination_recordings = new_data_dir.join(RECORDINGS_DIR);

    if destination_recordings.exists() {
        ensure_not_cancelled(app)?;
        fs::remove_dir_all(&destination_recordings)
            .map_err(|error| format!("Failed to reset staged recordings directory: {error}"))?;
    }

    if source_recordings.exists() {
        let mut cancel_check = || ensure_not_cancelled(app);
        copy_dir_recursive_chunked_with_cancel(
            &source_recordings,
            &destination_recordings,
            &mut cancel_check,
        )?;
    } else {
        ensure_not_cancelled(app)?;
        fs::create_dir_all(&destination_recordings)
            .map_err(|error| format!("Failed to create empty staged recordings directory: {error}"))?;
    }

    Ok(())
}

pub(super) fn create_snapshot(app_data_dir: &Path, runtime_dir: &Path) -> Result<PathBuf, String> {
    let snapshot_path = runtime_dir.join(format!("snapshot-{}", timestamp_millis()));
    fs::create_dir_all(&snapshot_path)
        .map_err(|error| format!("Failed to create restore snapshot directory: {error}"))?;

    copy_managed_data(app_data_dir, &snapshot_path)?;
    Ok(snapshot_path)
}

pub(super) fn build_restore_marker_snapshot_layout(
    snapshot_path: &Path,
) -> Result<UndoCheckpointSnapshotLayout, String> {
    Ok(UndoCheckpointSnapshotLayout {
        history_db: snapshot_component_present(snapshot_path, HISTORY_DB_FILE, false)?,
        user_dictionary: snapshot_component_present(snapshot_path, USER_DICTIONARY_FILE, false)?,
        user_store: snapshot_component_present(snapshot_path, USER_STORE_DB_FILE, false)?,
        recordings_dir: snapshot_component_present(snapshot_path, RECORDINGS_DIR, true)?,
    })
}

pub(super) fn validate_restore_marker_snapshot(
    snapshot_path: &Path,
    layout: &UndoCheckpointSnapshotLayout,
) -> Result<(), String> {
    validate_snapshot_component(snapshot_path, HISTORY_DB_FILE, false, layout.history_db)?;
    validate_snapshot_component(
        snapshot_path,
        USER_DICTIONARY_FILE,
        false,
        layout.user_dictionary,
    )?;
    validate_snapshot_component(snapshot_path, USER_STORE_DB_FILE, false, layout.user_store)?;
    validate_snapshot_component(snapshot_path, RECORDINGS_DIR, true, layout.recordings_dir)?;
    Ok(())
}

pub(super) fn copy_managed_data(source_root: &Path, destination_root: &Path) -> Result<(), String> {
    for file_name in [HISTORY_DB_FILE, USER_DICTIONARY_FILE, USER_STORE_DB_FILE] {
        let source = source_root.join(file_name);
        let destination = destination_root.join(file_name);

        if source.exists() {
            copy_file_chunked(&source, &destination)?;
        }
    }

    let source_recordings = source_root.join(RECORDINGS_DIR);
    let destination_recordings = destination_root.join(RECORDINGS_DIR);
    if source_recordings.exists() {
        copy_dir_recursive_chunked(&source_recordings, &destination_recordings)?;
    }

    Ok(())
}

pub(super) fn perform_staged_swap(app_data_dir: &Path, new_data_dir: &Path, runtime_dir: &Path) -> Result<(), String> {
    let displaced_dir = runtime_dir.join(format!("swap-old-{}", timestamp_millis()));
    fs::create_dir_all(&displaced_dir)
        .map_err(|error| format!("Failed to create displaced swap directory: {error}"))?;

    for file_name in [HISTORY_DB_FILE, USER_DICTIONARY_FILE, USER_STORE_DB_FILE] {
        let active = app_data_dir.join(file_name);
        let staged = new_data_dir.join(file_name);
        let displaced = displaced_dir.join(file_name);

        if active.exists() {
            if let Some(parent) = displaced.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "Failed to create displaced file parent '{}': {error}",
                        parent.display()
                    )
                })?;
            }

            fs::rename(&active, &displaced).map_err(|error| {
                format!(
                    "Failed to move active file '{}' into displaced swap dir: {error}",
                    active.display()
                )
            })?;
        }

        if staged.exists() {
            if let Some(parent) = active.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!("Failed to create active file parent '{}': {error}", parent.display())
                })?;
            }

            fs::rename(&staged, &active).map_err(|error| {
                format!(
                    "Failed to move staged file '{}' into active location '{}': {error}",
                    staged.display(),
                    active.display()
                )
            })?;
        }
    }

    let active_recordings = app_data_dir.join(RECORDINGS_DIR);
    let staged_recordings = new_data_dir.join(RECORDINGS_DIR);
    let displaced_recordings = displaced_dir.join(RECORDINGS_DIR);

    if active_recordings.exists() {
        fs::rename(&active_recordings, &displaced_recordings).map_err(|error| {
            format!(
                "Failed to move active recordings directory '{}' to displaced swap dir: {error}",
                active_recordings.display()
            )
        })?;
    }

    #[cfg(test)]
    if should_failpoint("swap_after_displace") {
        return Err("Injected failpoint: swap_after_displace".to_string());
    }

    if staged_recordings.exists() {
        fs::rename(&staged_recordings, &active_recordings).map_err(|error| {
            format!(
                "Failed to move staged recordings directory '{}' into active location '{}': {error}",
                staged_recordings.display(),
                active_recordings.display()
            )
        })?;
    } else {
        fs::create_dir_all(&active_recordings).map_err(|error| {
            format!(
                "Failed to create active recordings directory '{}': {error}",
                active_recordings.display()
            )
        })?;
    }

    fsync_path(app_data_dir)
        .map_err(|error| format!("Failed to sync app data dir after swap: {error}"))?;

    if let Err(error) = fs::remove_dir_all(&displaced_dir) {
        warn!(
            error = %error,
            path = %displaced_dir.display(),
            "Failed to remove displaced swap directory after successful swap"
        );
    }

    Ok(())
}

#[cfg(test)]
pub(super) fn should_failpoint(name: &str) -> bool {
    std::env::var("HANDY_TEST_BR_FAILPOINT")
        .ok()
        .map(|value| value.trim() == name)
        .unwrap_or(false)
}

pub(super) fn replace_active_data_from_source(
    source_root: &Path,
    snapshot_layout: &UndoCheckpointSnapshotLayout,
    app_data_dir: &Path,
) -> Result<(), String> {
    debug_assert!(
        validate_restore_marker_snapshot(source_root, snapshot_layout).is_ok(),
        "replace_active_data_from_source must only be called with a validated snapshot layout"
    );
    validate_restore_marker_snapshot(source_root, snapshot_layout).map_err(|error| {
        format!("Refusing to replace active data from invalid restore snapshot: {error}")
    })?;

    for file_name in [HISTORY_DB_FILE, USER_DICTIONARY_FILE, USER_STORE_DB_FILE] {
        let active = app_data_dir.join(file_name);
        if active.exists() {
            if active.is_file() {
                fs::remove_file(&active).map_err(|error| {
                    format!("Failed to remove active file '{}' before rollback: {error}", active.display())
                })?;
            } else {
                fs::remove_dir_all(&active).map_err(|error| {
                    format!(
                        "Failed to remove active path '{}' before rollback: {error}",
                        active.display()
                    )
                })?;
            }
        }

        let source = source_root.join(file_name);
        if source.exists() {
            copy_file_chunked(&source, &active)?;
        }
    }

    let active_recordings = app_data_dir.join(RECORDINGS_DIR);
    if active_recordings.exists() {
        fs::remove_dir_all(&active_recordings).map_err(|error| {
            format!(
                "Failed to remove active recordings directory '{}' before rollback: {error}",
                active_recordings.display()
            )
        })?;
    }

    let source_recordings = source_root.join(RECORDINGS_DIR);
    if source_recordings.exists() {
        copy_dir_recursive_chunked(&source_recordings, &active_recordings)?;
    } else {
        fs::create_dir_all(&active_recordings).map_err(|error| {
            format!(
                "Failed to create empty active recordings directory during rollback: {error}"
            )
        })?;
    }

    Ok(())
}
