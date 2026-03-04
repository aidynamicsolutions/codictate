//! Restore preflight validation.
//!
//! Inspects archive structure and payload integrity, validates compatibility and
//! resource bounds, and returns structured findings without mutating local data.

use super::*;

const PREFLIGHT_PROGRESS_TOTAL_UNITS: u64 = 1_000;
const PREFLIGHT_PROGRESS_START_UNITS: u64 = 25;
const PREFLIGHT_PROGRESS_FILE_VALIDATION_UNITS: u64 = 120;
const PREFLIGHT_PROGRESS_ARCHIVE_OPENED_UNITS: u64 = 220;
const PREFLIGHT_PROGRESS_ARCHIVE_SCANNED_UNITS: u64 = 430;
const PREFLIGHT_PROGRESS_MANIFEST_VALIDATED_UNITS: u64 = 560;
const PREFLIGHT_PROGRESS_CHECKSUM_VALIDATED_UNITS: u64 = 700;
const PREFLIGHT_PROGRESS_PAYLOAD_VALIDATED_UNITS: u64 = 820;
const PREFLIGHT_PROGRESS_LOCAL_VALIDATED_UNITS: u64 = 910;
const PREFLIGHT_PROGRESS_SPACE_VALIDATED_UNITS: u64 = 975;
const PREFLIGHT_CHECKSUM_PROGRESS_TARGET_UPDATES: u64 = 80;

struct PreflightProgressReporter<'a, R: tauri::Runtime> {
    app: &'a AppHandle<R>,
    enabled: bool,
    last_units: u64,
}

impl<'a, R: tauri::Runtime> PreflightProgressReporter<'a, R> {
    fn new(app: &'a AppHandle<R>, enabled: bool) -> Self {
        Self {
            app,
            enabled,
            last_units: 0,
        }
    }

    fn checkpoint(&mut self, units: u64) {
        if !self.enabled {
            return;
        }

        let bounded = units.min(PREFLIGHT_PROGRESS_TOTAL_UNITS);
        let monotonic = bounded.max(self.last_units);
        if monotonic == self.last_units {
            return;
        }

        self.last_units = monotonic;
        emit_progress(
            self.app,
            "restore",
            "preflight",
            monotonic,
            PREFLIGHT_PROGRESS_TOTAL_UNITS,
        );
    }

    fn complete(&mut self) {
        self.checkpoint(PREFLIGHT_PROGRESS_TOTAL_UNITS);
    }
}

fn should_emit_standalone_preflight_progress<R: tauri::Runtime>(app: &AppHandle<R>) -> bool {
    !is_operation_in_progress(app) && !is_maintenance_mode(app)
}

pub(super) fn map_preflight_stage_progress_units(
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

pub(super) fn checksum_progress_emit_interval(total_steps: u64) -> u64 {
    if total_steps == 0 {
        return 1;
    }

    (total_steps / PREFLIGHT_CHECKSUM_PROGRESS_TARGET_UPDATES).max(1)
}

pub(super) fn should_emit_checksum_progress_update(
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

pub fn preflight_restore<R: tauri::Runtime>(
    app: &AppHandle<R>,
    request: PreflightRestoreRequest,
) -> Result<PreflightRestoreReport, String> {
    ensure_supported_platform()?;
    let archive_path = PathBuf::from(request.archive_path);
    let context = build_preflight_context(app, &archive_path)?;
    Ok(context.report)
}

#[cfg(not(test))]
fn archive_entry_limit() -> u64 {
    MAX_ARCHIVE_ENTRIES
}

#[cfg(test)]
fn archive_entry_limit() -> u64 {
    std::env::var("HANDY_TEST_BR_MAX_ARCHIVE_ENTRIES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(MAX_ARCHIVE_ENTRIES)
}

pub(super) fn open_archive_for_preflight(archive_path: &Path) -> Result<File, String> {
    File::open(archive_path).map_err(|error| {
        format!(
            "Failed to open backup archive for preflight '{}': {error}",
            archive_path.display()
        )
    })
}

pub(super) fn open_archive_for_restore(archive_path: &Path) -> Result<File, String> {
    File::open(archive_path).map_err(|error| {
        format!(
            "Failed to open backup archive for restore '{}': {error}",
            archive_path.display()
        )
    })
}

pub(super) fn empty_preflight_report() -> PreflightRestoreReport {
    PreflightRestoreReport {
        can_apply: false,
        blocking_findings: Vec::new(),
        recoverable_findings: Vec::new(),
        summary: None,
        compatibility_note_code:
            PreflightCompatibilityNoteCode::V1MacosGuaranteedCrossPlatformBestEffort,
        compatibility_note:
            "V1 restore compatibility is guaranteed for macOS backups. Cross-platform restore is best-effort in v1.".to_string(),
    }
}

pub(super) fn build_preflight_context<R: tauri::Runtime>(
    app: &AppHandle<R>,
    archive_path: &Path,
) -> Result<PreflightContext, String> {
    let emit_detailed_progress = should_emit_standalone_preflight_progress(app);
    let mut progress = PreflightProgressReporter::new(app, emit_detailed_progress);
    progress.checkpoint(PREFLIGHT_PROGRESS_START_UNITS);
    let context = build_preflight_context_with_progress(app, archive_path, &mut progress);
    progress.complete();
    context
}

fn build_preflight_context_with_progress<R: tauri::Runtime>(
    app: &AppHandle<R>,
    archive_path: &Path,
    progress: &mut PreflightProgressReporter<'_, R>,
) -> Result<PreflightContext, String> {
    let mut report = empty_preflight_report();
    if !archive_path.exists() {
        report.blocking_findings.push(RestoreFinding {
            code: "archive_missing".to_string(),
            message: format!("Backup archive not found: {}", archive_path.display()),
        });
        progress.checkpoint(PREFLIGHT_PROGRESS_FILE_VALIDATION_UNITS);
        return Ok(PreflightContext { report });
    }

    let mut archive_file = match open_archive_for_preflight(archive_path) {
        Ok(file) => file,
        Err(error) => {
            report.blocking_findings.push(RestoreFinding {
                code: "archive_open_failed".to_string(),
                message: error,
            });
            progress.checkpoint(PREFLIGHT_PROGRESS_FILE_VALIDATION_UNITS);
            return Ok(PreflightContext { report });
        }
    };
    progress.checkpoint(PREFLIGHT_PROGRESS_FILE_VALIDATION_UNITS);

    build_preflight_context_with_open_archive_with_progress(
        app,
        archive_path,
        &mut archive_file,
        progress,
    )
}

pub(super) fn build_preflight_context_with_open_archive<R: tauri::Runtime>(
    app: &AppHandle<R>,
    archive_path: &Path,
    archive_file: &mut File,
) -> Result<PreflightContext, String> {
    let mut progress = PreflightProgressReporter::new(app, false);
    build_preflight_context_with_open_archive_with_progress(
        app,
        archive_path,
        archive_file,
        &mut progress,
    )
}

fn build_preflight_context_with_open_archive_with_progress<R: tauri::Runtime>(
    app: &AppHandle<R>,
    archive_path: &Path,
    archive_file: &mut File,
    progress: &mut PreflightProgressReporter<'_, R>,
) -> Result<PreflightContext, String> {
    let mut report = empty_preflight_report();
    progress.checkpoint(PREFLIGHT_PROGRESS_ARCHIVE_OPENED_UNITS);

    if archive_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| !extension.eq_ignore_ascii_case(BACKUP_FILE_EXTENSION))
        .unwrap_or(true)
    {
        report.recoverable_findings.push(RestoreFinding {
            code: "archive_extension_unexpected".to_string(),
            message: "Backup file extension is unexpected; continuing preflight anyway.".to_string(),
        });
    }

    let metadata = match archive_file.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            report.blocking_findings.push(RestoreFinding {
                code: "archive_metadata_error".to_string(),
                message: format!("Failed to read backup file metadata: {error}"),
            });
            progress.checkpoint(PREFLIGHT_PROGRESS_ARCHIVE_SCANNED_UNITS);
            return Ok(PreflightContext {
                report,
            });
        }
    };

    if metadata.len() > MAX_ARCHIVE_SIZE_BYTES {
        report.blocking_findings.push(RestoreFinding {
            code: "archive_size_limit".to_string(),
            message: format!(
                "Backup archive is too large ({} bytes). Maximum allowed is {} bytes.",
                metadata.len(), MAX_ARCHIVE_SIZE_BYTES
            ),
        });
    }

    archive_file
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("Failed to rewind backup archive before preflight: {error}"))?;

    let mut archive = match ZipArchive::new(&mut *archive_file) {
        Ok(archive) => archive,
        Err(error) => {
            report.blocking_findings.push(RestoreFinding {
                code: "archive_parse_failed".to_string(),
                message: format!("Failed to parse backup archive as ZIP: {error}"),
            });
            report.can_apply = false;
            progress.checkpoint(PREFLIGHT_PROGRESS_ARCHIVE_SCANNED_UNITS);
            return Ok(PreflightContext { report });
        }
    };

    let entry_limit = archive_entry_limit();
    if archive.len() as u64 > entry_limit {
        report.blocking_findings.push(RestoreFinding {
            code: "archive_entry_limit".to_string(),
            message: format!(
                "Backup archive entry count exceeds limit ({} > {}).",
                archive.len(), entry_limit
            ),
        });

        report.can_apply = false;
        drop(archive);
        archive_file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("Failed to rewind backup archive after preflight: {error}"))?;
        progress.checkpoint(PREFLIGHT_PROGRESS_ARCHIVE_SCANNED_UNITS);
        return Ok(PreflightContext { report });
    }

    let inventory = inspect_archive_entries(&mut archive, &mut report);
    progress.checkpoint(PREFLIGHT_PROGRESS_ARCHIVE_SCANNED_UNITS);

    if inventory.total_uncompressed_bytes > MAX_TOTAL_UNCOMPRESSED_BYTES {
        report.blocking_findings.push(RestoreFinding {
            code: "archive_uncompressed_limit".to_string(),
            message: format!(
                "Backup uncompressed size exceeds limit ({} > {}).",
                inventory.total_uncompressed_bytes, MAX_TOTAL_UNCOMPRESSED_BYTES
            ),
        });
    }

    for required in [MANIFEST_FILE, CHECKSUM_FILE, HISTORY_FILE, DICTIONARY_FILE] {
        if inventory.entries.contains_key(required) {
            continue;
        }

        if inventory.oversized_entries.contains(required) {
            report.blocking_findings.push(RestoreFinding {
                code: "required_payload_size_limit".to_string(),
                message: format!(
                    "Required payload '{required}' exceeds payload size limit and cannot be restored."
                ),
            });
        } else {
            report.blocking_findings.push(RestoreFinding {
                code: "missing_required_payload".to_string(),
                message: format!("Required payload '{required}' is missing from backup archive."),
            });
        }
    }

    if let Some(index) = inventory.entries.get(MANIFEST_FILE) {
        match read_json_from_zip::<BackupManifest, _>(&mut archive, *index) {
            Ok(parsed) => {
                validate_manifest_compatibility(&parsed, &mut report);
                report.summary = Some(PreflightSummary {
                    backup_format_version: parsed.backup_format_version.clone(),
                    created_at: parsed.created_at.clone(),
                    created_with_app_version: parsed.created_with_app_version.clone(),
                    platform: parsed.platform.clone(),
                    includes_recordings: parsed.includes_recordings,
                    counts: BackupCounts {
                        history_entries: parsed.counts.history_entries,
                        recording_files: parsed.counts.recording_files,
                        dictionary_entries: parsed.counts.dictionary_entries,
                    },
                    estimated_size_bytes: parsed.estimated_size_bytes,
                });
            }
            Err(error) => report.blocking_findings.push(RestoreFinding {
                code: "manifest_parse_failed".to_string(),
                message: format!("Failed to parse manifest.json: {error}"),
            }),
        }
    }
    progress.checkpoint(PREFLIGHT_PROGRESS_MANIFEST_VALIDATED_UNITS);

    // Validate checksums and checksum syntax before any restore writes.
    if let Some(checksum_index) = inventory.entries.get(CHECKSUM_FILE) {
        match read_text_from_zip(&mut archive, *checksum_index) {
            Ok(contents) => match parse_checksums(&contents) {
                Ok(checksums) => {
                    let required_targets = [MANIFEST_FILE, HISTORY_FILE, DICTIONARY_FILE];
                    let mut payload_paths = inventory
                        .entries
                        .keys()
                        .filter(|path| path.as_str() != CHECKSUM_FILE)
                        .cloned()
                        .collect::<Vec<_>>();
                    payload_paths.sort();

                    let required_checksum_steps = required_targets
                        .iter()
                        .filter(|required| !inventory.oversized_entries.contains(**required))
                        .count() as u64;
                    let payload_checksum_steps = payload_paths.len() as u64;
                    let checksum_verification_steps = checksums.len() as u64;
                    let checksum_progress_total_steps = required_checksum_steps
                        .saturating_add(payload_checksum_steps)
                        .saturating_add(checksum_verification_steps);
                    let checksum_emit_interval =
                        checksum_progress_emit_interval(checksum_progress_total_steps);
                    let mut checksum_progress_processed_steps = 0_u64;
                    let mut checksum_progress_last_emitted_steps = 0_u64;

                    let mut emit_checksum_progress_step =
                        |progress: &mut PreflightProgressReporter<'_, R>| {
                            if checksum_progress_total_steps == 0 {
                                return;
                            }

                            checksum_progress_processed_steps = checksum_progress_processed_steps
                                .saturating_add(1)
                                .min(checksum_progress_total_steps);

                            if should_emit_checksum_progress_update(
                                checksum_progress_processed_steps,
                                checksum_progress_last_emitted_steps,
                                checksum_progress_total_steps,
                                checksum_emit_interval,
                            ) {
                                let current = map_preflight_stage_progress_units(
                                    PREFLIGHT_PROGRESS_MANIFEST_VALIDATED_UNITS,
                                    PREFLIGHT_PROGRESS_CHECKSUM_VALIDATED_UNITS,
                                    checksum_progress_processed_steps,
                                    checksum_progress_total_steps,
                                );
                                progress.checkpoint(current);
                                checksum_progress_last_emitted_steps =
                                    checksum_progress_processed_steps;
                            }
                        };

                    for required in required_targets {
                        if inventory.oversized_entries.contains(required) {
                            continue;
                        }

                        if !checksums.contains_key(required) {
                            report.blocking_findings.push(RestoreFinding {
                                code: "checksum_missing_required".to_string(),
                                message: format!(
                                    "checksums.sha256 is missing required payload checksum for '{required}'."
                                ),
                            });
                        }
                        emit_checksum_progress_step(progress);
                    }

                    for payload_path in payload_paths {
                        if !checksums.contains_key(payload_path.as_str()) {
                            report.blocking_findings.push(RestoreFinding {
                                code: "checksum_missing_payload".to_string(),
                                message: format!(
                                    "checksums.sha256 is missing payload checksum for '{payload_path}'."
                                ),
                            });
                        }
                        emit_checksum_progress_step(progress);
                    }

                    for (relative, expected_checksum) in checksums {
                        if let Some(index) = inventory.entries.get(relative.as_str()) {
                            match checksum_zip_entry(&mut archive, *index) {
                                Ok(actual_checksum) => {
                                    if actual_checksum != expected_checksum {
                                        report.blocking_findings.push(RestoreFinding {
                                            code: "checksum_mismatch".to_string(),
                                            message: format!(
                                                "Checksum mismatch for '{relative}'."
                                            ),
                                        });
                                    }
                                }
                                Err(error) => {
                                    report.blocking_findings.push(RestoreFinding {
                                        code: "checksum_verification_failed".to_string(),
                                        message: format!(
                                            "Failed to verify checksum for '{relative}': {error}"
                                        ),
                                    });
                                }
                            }
                        } else if inventory.oversized_entries.contains(relative.as_str()) {
                            // Size violations are already blocking findings; skip checksum
                            // verification to avoid loading oversized payload entries.
                        } else {
                            report.blocking_findings.push(RestoreFinding {
                                code: "checksum_payload_missing".to_string(),
                                message: format!(
                                    "Checksum references '{relative}' but payload is missing from archive."
                                ),
                            });
                        }
                        emit_checksum_progress_step(progress);
                    }

                    if checksum_progress_total_steps > 0
                        && checksum_progress_processed_steps < checksum_progress_total_steps
                    {
                        progress.checkpoint(map_preflight_stage_progress_units(
                            PREFLIGHT_PROGRESS_MANIFEST_VALIDATED_UNITS,
                            PREFLIGHT_PROGRESS_CHECKSUM_VALIDATED_UNITS,
                            checksum_progress_total_steps,
                            checksum_progress_total_steps,
                        ));
                    }
                }
                Err(error) => report.blocking_findings.push(RestoreFinding {
                    code: "checksum_parse_failed".to_string(),
                    message: format!("Failed to parse checksums.sha256: {error}"),
                }),
            },
            Err(error) => report.blocking_findings.push(RestoreFinding {
                code: "checksum_read_failed".to_string(),
                message: format!("Failed to read checksums.sha256: {error}"),
            }),
        }
    }
    progress.checkpoint(PREFLIGHT_PROGRESS_CHECKSUM_VALIDATED_UNITS);

    // Validate history row stream and enforce row bound.
    if let Some(index) = inventory.entries.get(HISTORY_FILE) {
        match read_history_row_count(&mut archive, *index) {
            Ok(row_count) => {
                if row_count > MAX_HISTORY_ROWS {
                    report.blocking_findings.push(RestoreFinding {
                        code: "history_row_limit".to_string(),
                        message: format!(
                            "History row count exceeds limit ({} > {}).",
                            row_count, MAX_HISTORY_ROWS
                        ),
                    });
                }
                if let Some(summary) = report.summary.as_mut() {
                    if summary.counts.history_entries == 0 {
                        summary.counts.history_entries = row_count;
                    }
                }
            }
            Err(error) => {
                let code = if error.starts_with("history_jsonl_line_too_large:") {
                    "history_line_size_limit"
                } else {
                    "history_payload_invalid"
                };
                report.blocking_findings.push(RestoreFinding {
                    code: code.to_string(),
                    message: format!("Failed to validate history payload: {error}"),
                });
            }
        }
    }

    // Validate dictionary payload shape.
    if let Some(index) = inventory.entries.get(DICTIONARY_FILE) {
        match read_json_from_zip::<DictionaryPayload, _>(&mut archive, *index) {
            Ok(payload) => {
                if let Some(summary) = report.summary.as_mut() {
                    if summary.counts.dictionary_entries == 0 {
                        summary.counts.dictionary_entries = payload.entries.len() as u64;
                    }
                }
            }
            Err(error) => report.blocking_findings.push(RestoreFinding {
                code: "dictionary_payload_invalid".to_string(),
                message: format!("Failed to parse dictionary payload: {error}"),
            }),
        }
    }

    // user_store is recoverable: keep local state on missing, malformed, or invalid-structure file.
    if let Some(index) = inventory.entries.get(USER_STORE_FILE) {
        match read_json_from_zip::<serde_json::Value, _>(&mut archive, *index) {
            Ok(value) => {
                if let Err(error) = validate_user_store_payload_shape(&value) {
                    report.recoverable_findings.push(RestoreFinding {
                        code: "user_store_payload_recoverable".to_string(),
                        message: format!(
                            "user/user_store.json has invalid structure and will be replaced with local fallback/defaults: {error}"
                        ),
                    });
                }
            }
            Err(error) => {
                report.recoverable_findings.push(RestoreFinding {
                    code: "user_store_payload_recoverable".to_string(),
                    message: format!(
                        "user/user_store.json is malformed and will be replaced with local fallback/defaults: {error}"
                    ),
                });
            }
        }
    } else {
        report.recoverable_findings.push(RestoreFinding {
            code: "user_store_missing_recoverable".to_string(),
            message:
            "user/user_store.json is missing; restore will keep local user-store state when available."
                    .to_string(),
        });
    }

    // user_stats payload is optional for legacy backups; fallback recompute is allowed.
    if let Some(index) = inventory.entries.get(HISTORY_USER_STATS_FILE) {
        match read_json_from_zip::<UserStatsPayloadV1, _>(&mut archive, *index) {
            Ok(payload) => {
                if let Err(error) = validate_user_stats_payload(&payload) {
                    report.recoverable_findings.push(RestoreFinding {
                        code: "user_stats_payload_recoverable".to_string(),
                        message: format!(
                            "{HISTORY_USER_STATS_FILE} had invalid structure and productivity stats will be recomputed: {error}"
                        ),
                    });
                }
            }
            Err(error) => {
                report.recoverable_findings.push(RestoreFinding {
                    code: "user_stats_payload_recoverable".to_string(),
                    message: format!(
                        "{HISTORY_USER_STATS_FILE} was malformed and productivity stats will be recomputed: {error}"
                    ),
                });
            }
        }
    } else {
        report.recoverable_findings.push(RestoreFinding {
            code: "user_stats_missing_recoverable".to_string(),
            message: format!(
                "{HISTORY_USER_STATS_FILE} is missing; productivity stats will be recomputed from history entries."
            ),
        });
    }
    progress.checkpoint(PREFLIGHT_PROGRESS_PAYLOAD_VALIDATED_UNITS);

    // Local integrity gate: stop restore if current local history data is corrupt.
    let app_data_dir = match app_data_dir(app) {
        Ok(path) => path,
        Err(error) => {
            report.blocking_findings.push(RestoreFinding {
                code: "local_data_dir_unavailable".to_string(),
                message: format!("Could not access local app data directory: {error}"),
            });
            report.can_apply = false;
            progress.checkpoint(PREFLIGHT_PROGRESS_LOCAL_VALIDATED_UNITS);
            return Ok(PreflightContext { report });
        }
    };

    if let Err(error) = validate_local_history_integrity(&app_data_dir) {
        report.blocking_findings.push(RestoreFinding {
            code: "local_history_data_corrupted".to_string(),
            message: error,
        });
    }
    progress.checkpoint(PREFLIGHT_PROGRESS_LOCAL_VALIDATED_UNITS);

    // Free-space gate.
    let active_managed_bytes = managed_data_size_bytes(&app_data_dir)?;
    let required_free_space = inventory
        .total_uncompressed_bytes
        .saturating_mul(2)
        .saturating_add(active_managed_bytes)
        .saturating_add(MAX_PAYLOAD_FILE_SIZE_BYTES)
        .saturating_add(SAFETY_MARGIN_BYTES);

    let available_space = match fs2::available_space(&app_data_dir) {
        Ok(value) => value,
        Err(error) => {
            report.blocking_findings.push(RestoreFinding {
                code: "available_space_check_failed".to_string(),
                message: format!("Could not determine available disk space for restore: {error}"),
            });
            report.can_apply = false;
            progress.checkpoint(PREFLIGHT_PROGRESS_SPACE_VALIDATED_UNITS);
            return Ok(PreflightContext { report });
        }
    };

    if available_space < required_free_space {
        report.blocking_findings.push(RestoreFinding {
            code: "insufficient_free_space".to_string(),
            message: format!(
                "Not enough free disk space. Required: {} bytes, available: {} bytes.",
                required_free_space, available_space
            ),
        });
    }

    report.can_apply = report.blocking_findings.is_empty();
    drop(archive);

    archive_file
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("Failed to rewind backup archive after preflight: {error}"))?;
    progress.checkpoint(PREFLIGHT_PROGRESS_SPACE_VALIDATED_UNITS);

    Ok(PreflightContext { report })
}

pub(super) fn validate_manifest_compatibility(manifest: &BackupManifest, report: &mut PreflightRestoreReport) {
    let major_version = manifest
        .backup_format_version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .unwrap_or(0);

    if major_version != 1 {
        report.blocking_findings.push(RestoreFinding {
            code: "backup_format_version_unsupported".to_string(),
            message: format!(
                "Backup format '{}' is unsupported by this app version.",
                manifest.backup_format_version
            ),
        });
    }

    let current_platform = current_platform();
    if !manifest.platform.is_empty() && !manifest.platform.eq_ignore_ascii_case(current_platform) {
        report.recoverable_findings.push(RestoreFinding {
            code: "cross_platform_best_effort".to_string(),
            message: format!(
                "Backup was created on '{}' while current platform is '{}'. Cross-platform restore is best-effort in v1.",
                manifest.platform, current_platform
            ),
        });
    }
}

pub(super) fn inspect_archive_entries<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    report: &mut PreflightRestoreReport,
) -> ArchiveEntryInventory {
    let mut entries = HashMap::new();
    let mut oversized_entries = BTreeSet::new();
    let mut normalized_paths = BTreeSet::new();
    let mut file_paths = BTreeSet::new();
    let mut lowercase_paths: HashMap<String, String> = HashMap::new();
    let mut total_uncompressed_bytes = 0_u64;

    'entry_scan: for index in 0..archive.len() {
        let file = match archive.by_index(index) {
            Ok(file) => file,
            Err(error) => {
                report.blocking_findings.push(RestoreFinding {
                    code: "archive_entry_read_failed".to_string(),
                    message: format!("Failed to read backup archive entry #{index}: {error}"),
                });
                continue;
            }
        };

        let raw_name = file.name().to_string();
        let normalized = match normalize_archive_path(&raw_name) {
            Ok(path) => path,
            Err(error) => {
                report.blocking_findings.push(RestoreFinding {
                    code: "archive_unsafe_path".to_string(),
                    message: format!(
                        "Archive contains unsafe path '{raw_name}' and cannot be restored: {error}"
                    ),
                });
                continue;
            }
        };

        if is_link_entry(&file) {
            report.blocking_findings.push(RestoreFinding {
                code: "archive_link_entry".to_string(),
                message: format!(
                    "Archive entry '{raw_name}' is a symlink/hardlink and is not allowed."
                ),
            });
            continue;
        }

        let lowercase = normalized.to_lowercase();
        if let Some(existing) = lowercase_paths.get(&lowercase) {
            if existing == &normalized {
                report.blocking_findings.push(RestoreFinding {
                    code: "archive_duplicate_path".to_string(),
                    message: format!(
                        "Archive has duplicate normalized path '{normalized}'."
                    ),
                });
            } else {
                report.blocking_findings.push(RestoreFinding {
                    code: "archive_case_collision".to_string(),
                    message: format!(
                        "Archive has case-colliding paths '{existing}' and '{normalized}'."
                    ),
                });
            }
            continue;
        }

        if normalized_paths.contains(&normalized) {
            report.blocking_findings.push(RestoreFinding {
                code: "archive_duplicate_path".to_string(),
                message: format!("Archive has duplicate path '{normalized}'."),
            });
            continue;
        }

        let segments = normalized.split('/').collect::<Vec<_>>();
        if segments.len() > 1 {
            let mut parent = String::new();
            for (idx, segment) in segments
                .iter()
                .enumerate()
                .take(segments.len().saturating_sub(1))
            {
                if idx > 0 {
                    parent.push('/');
                }
                parent.push_str(segment);

                if file_paths.contains(parent.as_str()) {
                    report.blocking_findings.push(RestoreFinding {
                        code: "archive_path_conflict".to_string(),
                        message: format!(
                            "Archive path '{normalized}' requires '{parent}' to be a directory, but '{parent}' is a file entry."
                        ),
                    });
                    continue 'entry_scan;
                }
            }
        }

        lowercase_paths.insert(lowercase, normalized.clone());
        normalized_paths.insert(normalized.clone());

        if !file.is_dir() {
            let child_prefix = format!("{normalized}/");
            if let Some(existing_child) = normalized_paths
                .range(child_prefix.clone()..)
                .find(|candidate| candidate.starts_with(&child_prefix))
            {
                report.blocking_findings.push(RestoreFinding {
                    code: "archive_path_conflict".to_string(),
                    message: format!(
                        "Archive file path '{normalized}' conflicts with nested entry '{existing_child}'."
                    ),
                });
                continue;
            }

            file_paths.insert(normalized.clone());
            let uncompressed = file.size();
            total_uncompressed_bytes = total_uncompressed_bytes.saturating_add(uncompressed);

            if uncompressed > MAX_PAYLOAD_FILE_SIZE_BYTES {
                report.blocking_findings.push(RestoreFinding {
                    code: "archive_payload_size_limit".to_string(),
                    message: format!(
                        "Archive entry '{normalized}' exceeds payload size limit ({} > {}).",
                        uncompressed, MAX_PAYLOAD_FILE_SIZE_BYTES
                    ),
                });
                oversized_entries.insert(normalized);
                continue;
            }
            entries.insert(normalized, index);
        }
    }

    ArchiveEntryInventory {
        entries,
        oversized_entries,
        total_uncompressed_bytes,
    }
}

pub(super) fn validate_local_history_integrity(app_data_dir: &Path) -> Result<(), String> {
    let history_path = app_data_dir.join(HISTORY_DB_FILE);
    if !history_path.exists() {
        return Ok(());
    }

    let conn = Connection::open_with_flags(&history_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| {
            format!(
                "Current local history data appears unreadable. Restore was stopped to avoid overwriting potentially recoverable data: {error}"
            )
        })?;

    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| {
            format!(
                "Current local history data could not be validated. Restore was stopped to avoid data loss: {error}"
            )
        })?;

    if quick_check.trim().eq_ignore_ascii_case("ok") {
        return Ok(());
    }

    Err(
        "Current local history data appears corrupted. Restore was stopped to avoid data loss. Repair local data first, then try restore again."
            .to_string(),
    )
}

pub(super) fn is_link_entry(file: &zip::read::ZipFile<'_>) -> bool {
    if let Some(mode) = file.unix_mode() {
        let file_type_bits = mode & 0o170000;
        // 0o120000 is symlink. ZIP does not have a distinct hardlink type,
        // but we keep this guard explicit for safety policy parity.
        return file_type_bits == 0o120000;
    }
    false
}

pub(super) fn parse_checksums(contents: &str) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();

    for (line_number, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, "  ");
        let checksum = parts
            .next()
            .ok_or_else(|| format!("Missing checksum token on line {}", line_number + 1))?;
        let path = parts
            .next()
            .ok_or_else(|| format!("Missing path token on line {}", line_number + 1))?;

        if checksum.len() != 64 || !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid checksum format on line {}",
                line_number + 1
            ));
        }

        let normalized = normalize_archive_path(path)
            .map_err(|error| format!("Invalid checksum path '{path}': {error}"))?;

        if map.insert(normalized, checksum.to_ascii_lowercase()).is_some() {
            return Err(format!(
                "Duplicate checksum path '{}' on line {}",
                path,
                line_number + 1
            ));
        }
    }

    Ok(map)
}

pub(super) fn read_history_row_count<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
) -> Result<u64, String> {
    let file = archive
        .by_index(index)
        .map_err(|error| format!("Failed to open history payload entry: {error}"))?;
    let reader = BufReader::new(file);

    let mut count = 0_u64;
    let mut reader = reader;
    while let Some(line) = read_history_jsonl_line_bounded(&mut reader)? {
        if line.trim().is_empty() {
            continue;
        }

        let row: HistoryRowV1 = serde_json::from_str(&line)
            .map_err(|error| format!("Invalid history JSONL row: {error}"))?;
        sanitize_relative_file_name(&row.file_name).map_err(|error| {
            format!(
                "Invalid history JSONL row file_name '{}': {error}",
                row.file_name
            )
        })?;

        count = count.saturating_add(1);
        if count > MAX_HISTORY_ROWS {
            return Ok(count);
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_progress_units_are_bounded_and_monotonic() {
        let start = PREFLIGHT_PROGRESS_MANIFEST_VALIDATED_UNITS;
        let end = PREFLIGHT_PROGRESS_CHECKSUM_VALIDATED_UNITS;
        let total = 1_000;

        assert_eq!(map_preflight_stage_progress_units(start, end, 0, total), start);
        assert_eq!(
            map_preflight_stage_progress_units(start, end, total, total),
            end
        );
        assert_eq!(
            map_preflight_stage_progress_units(start, end, total.saturating_mul(2), total),
            end
        );

        let mut previous = map_preflight_stage_progress_units(start, end, 0, total);
        for processed in 1..=total {
            let current = map_preflight_stage_progress_units(start, end, processed, total);
            assert!(current >= previous, "stage progress must be monotonic");
            assert!(
                current >= start && current <= end,
                "stage progress should stay within checksum span"
            );
            previous = current;
        }
    }

    #[test]
    fn checksum_progress_emit_interval_is_never_zero() {
        assert_eq!(checksum_progress_emit_interval(0), 1);
        assert_eq!(checksum_progress_emit_interval(1), 1);
        assert!(checksum_progress_emit_interval(8_000) >= 1);
    }

    #[test]
    fn checksum_progress_update_always_emits_on_completion() {
        let total = 101;
        let interval = checksum_progress_emit_interval(total);
        let mut emitted_steps = Vec::new();
        let mut last_emitted = 0_u64;

        for processed in 1..=total {
            if should_emit_checksum_progress_update(processed, last_emitted, total, interval) {
                emitted_steps.push(processed);
                last_emitted = processed;
            }
        }

        assert!(
            emitted_steps.last().copied() == Some(total),
            "final checksum progress step must always emit completion"
        );
    }
}
