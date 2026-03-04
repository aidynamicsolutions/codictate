//! Undo checkpoint lifecycle and undo-restore application flow.
//!
//! Manages single-slot checkpoint publication/validation/pruning and applies a
//! replace-only rollback from the retained restore snapshot.

use super::*;

const UNDO_PROGRESS_TOTAL_UNITS: u64 = 1_000;
const UNDO_PROGRESS_PREPARE_UNITS: u64 = 50;
const UNDO_PROGRESS_STAGE_CHECKPOINT_START_UNITS: u64 = 150;
const UNDO_PROGRESS_STAGE_CHECKPOINT_DONE_UNITS: u64 = 350;
const UNDO_PROGRESS_SNAPSHOT_CURRENT_UNITS: u64 = 550;
const UNDO_PROGRESS_SWAP_START_UNITS: u64 = 700;
const UNDO_PROGRESS_SWAP_COMMITTED_UNITS: u64 = 850;
const UNDO_PROGRESS_CLEANUP_UNITS: u64 = 950;
const UNDO_PROGRESS_FINALIZE_UNITS: u64 = 1_000;

fn emit_undo_progress<R: tauri::Runtime>(app: &AppHandle<R>, phase: &str, current: u64) {
    emit_progress(
        app,
        "undo",
        phase,
        current.min(UNDO_PROGRESS_TOTAL_UNITS),
        UNDO_PROGRESS_TOTAL_UNITS,
    );
}

pub fn undo_last_restore<R: tauri::Runtime>(
    app: &AppHandle<R>,
    _request: UndoLastRestoreRequest,
) -> Result<UndoLastRestoreReport, String> {
    ensure_supported_platform()?;
    let _op_guard = start_operation(app)?;
    emit_undo_progress(app, "prepare", UNDO_PROGRESS_PREPARE_UNITS);

    let app_data_dir = app_data_dir(app)?;
    prune_expired_checkpoint(&app_data_dir);

    let meta_path = undo_checkpoint_meta_path(&app_data_dir);
    if !meta_path.exists() {
        return Ok(UndoLastRestoreReport {
            restored: false,
            message: "Undo Last Restore is unavailable because no checkpoint exists.".to_string(),
        });
    }

    let metadata = read_json_file::<UndoCheckpointMeta>(&meta_path)
        .map_err(|error| format!("Failed to read undo checkpoint metadata: {error}"))?;

    let Some(snapshot_layout) = metadata.snapshot_layout.as_ref() else {
        let _ = remove_file_with_parent_sync(&meta_path);
        return Ok(UndoLastRestoreReport {
            restored: false,
            message:
                "Undo Last Restore is unavailable because checkpoint metadata is incomplete."
                    .to_string(),
        });
    };

    let expires_at = parse_rfc3339(&metadata.expires_at);
    if expires_at.map(|expiry| expiry <= Utc::now()).unwrap_or(true) {
        let _ = remove_file_with_parent_sync(&meta_path);
        return Ok(UndoLastRestoreReport {
            restored: false,
            message: "Undo Last Restore is unavailable because the checkpoint has expired.".to_string(),
        });
    }

    let source_snapshot = match validated_runtime_snapshot_path(&app_data_dir, &metadata.snapshot_path)
    {
        Ok(path) => path,
        Err(error) => {
            warn!(
                error = %error,
                snapshot_path = %metadata.snapshot_path,
                "Undo checkpoint snapshot path is invalid; removing checkpoint metadata"
            );
            let _ = remove_file_with_parent_sync(&meta_path);
            return Ok(UndoLastRestoreReport {
                restored: false,
                message: "Undo Last Restore is unavailable because checkpoint data is missing."
                    .to_string(),
            });
        }
    };

    if let Err(error) = validate_undo_checkpoint_snapshot(&source_snapshot, snapshot_layout) {
        warn!(
            error = %error,
            snapshot_path = %source_snapshot.display(),
            "Undo checkpoint snapshot is incomplete or invalid; removing checkpoint"
        );
        let _ = remove_file_with_parent_sync(&meta_path);
        if let Err(remove_error) = remove_snapshot_dir(&source_snapshot) {
            warn!(
                error = %remove_error,
                path = %source_snapshot.display(),
                "Failed to remove invalid undo checkpoint snapshot"
            );
        }
        return Ok(UndoLastRestoreReport {
            restored: false,
            message: "Undo Last Restore is unavailable because checkpoint data is invalid."
                .to_string(),
        });
    }

    let runtime_dir = runtime_dir(&app_data_dir);
    fs::create_dir_all(&runtime_dir)
        .map_err(|error| format!("Failed to create undo runtime directory: {error}"))?;

    let work_root = runtime_dir.join(format!("undo-work-{}", timestamp_millis()));
    let new_data_dir = work_root.join("new-data");
    fs::create_dir_all(&new_data_dir)
        .map_err(|error| format!("Failed to create undo staging directory: {error}"))?;

    let undo_result = (|| -> Result<UndoLastRestoreReport, String> {
        emit_undo_progress(
            app,
            "stage-checkpoint",
            UNDO_PROGRESS_STAGE_CHECKPOINT_START_UNITS,
        );
        ensure_not_cancelled(app)?;
        copy_managed_data(&source_snapshot, &new_data_dir)?;
        emit_undo_progress(
            app,
            "stage-checkpoint",
            UNDO_PROGRESS_STAGE_CHECKPOINT_DONE_UNITS,
        );

        ensure_not_cancelled(app)?;
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
        emit_undo_progress(
            app,
            "snapshot-current",
            UNDO_PROGRESS_SNAPSHOT_CURRENT_UNITS,
        );

        ensure_not_cancelled(app)?;
        emit_undo_progress(app, "swap", UNDO_PROGRESS_SWAP_START_UNITS);
        if let Err(error) = perform_staged_swap(&app_data_dir, &new_data_dir, &runtime_dir) {
            return rollback_after_swap_failure(
                &app_data_dir,
                &marker_path,
                &snapshot_path,
                &snapshot_layout,
                "Undo restore failed during swap",
                &error.to_string(),
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
                "Undo restore failed after swap while persisting commit marker",
                &error,
            );
        }
        emit_undo_progress(app, "swap", UNDO_PROGRESS_SWAP_COMMITTED_UNITS);

        if let Err(error) = remove_file_with_parent_sync(&marker_path) {
            warn!(
                error = %error,
                path = %marker_path.display(),
                "Failed to remove committed undo marker; startup will reconcile"
            );
        }
        if let Err(error) = remove_file_with_parent_sync(&meta_path) {
            warn!(
                error = %error,
                path = %meta_path.display(),
                "Failed to remove undo checkpoint metadata after successful undo"
            );
        }
        if let Err(error) = remove_snapshot_dir(&source_snapshot) {
            warn!(
                error = %error,
                path = %source_snapshot.display(),
                "Failed to remove consumed undo checkpoint snapshot"
            );
        }
        if let Err(error) = remove_snapshot_dir(&snapshot_path) {
            warn!(
                error = %error,
                path = %snapshot_path.display(),
                "Failed to remove undo rollback snapshot after successful undo"
            );
        }
        emit_undo_progress(app, "cleanup", UNDO_PROGRESS_CLEANUP_UNITS);

        if let Err(error) = user_dictionary::reload_dictionary_state(app) {
            warn!(error = %error, "Failed to refresh in-memory dictionary after undo");
        }
        reload_user_store_state(app);

        let _ = app.emit("history-updated", ());
        let _ = app.emit("dictionary-updated", ());
        let _ = app.emit("user-profile-updated", ());
        emit_undo_progress(app, "finalize", UNDO_PROGRESS_FINALIZE_UNITS);

        Ok(UndoLastRestoreReport {
            restored: true,
            message: "Undo Last Restore completed successfully.".to_string(),
        })
    })();

    if let Err(error) = fs::remove_dir_all(&work_root) {
        warn!(
            error = %error,
            path = %work_root.display(),
            "Failed to clean undo work directory"
        );
    }
    prune_orphan_snapshot_dirs_keep_latest(&app_data_dir);

    undo_result
}

pub fn undo_last_restore_availability<R: tauri::Runtime>(
    app: &AppHandle<R>,
) -> Result<UndoLastRestoreAvailabilityReport, String> {
    ensure_supported_platform()?;
    let app_data_dir = app_data_dir(app)?;
    prune_expired_checkpoint(&app_data_dir);

    let meta_path = undo_checkpoint_meta_path(&app_data_dir);
    if !meta_path.exists() {
        return Ok(UndoLastRestoreAvailabilityReport {
            available: false,
            expires_at: None,
            message: "Undo Last Restore is unavailable.".to_string(),
        });
    }

    let metadata = read_json_file::<UndoCheckpointMeta>(&meta_path)
        .map_err(|error| format!("Failed to read undo checkpoint metadata: {error}"))?;

    let Some(snapshot_layout) = metadata.snapshot_layout.as_ref() else {
        let _ = remove_file_with_parent_sync(&meta_path);
        return Ok(UndoLastRestoreAvailabilityReport {
            available: false,
            expires_at: None,
            message:
                "Undo Last Restore is unavailable because checkpoint metadata is incomplete."
                    .to_string(),
        });
    };

    let expires_at = parse_rfc3339(&metadata.expires_at);
    if expires_at.map(|expiry| expiry <= Utc::now()).unwrap_or(true) {
        let _ = remove_file_with_parent_sync(&meta_path);
        return Ok(UndoLastRestoreAvailabilityReport {
            available: false,
            expires_at: None,
            message: "Undo Last Restore is unavailable because the checkpoint expired.".to_string(),
        });
    }

    let source_snapshot = match validated_runtime_snapshot_path(&app_data_dir, &metadata.snapshot_path)
    {
        Ok(path) => path,
        Err(error) => {
            warn!(
                error = %error,
                snapshot_path = %metadata.snapshot_path,
                "Undo checkpoint snapshot path is invalid; removing checkpoint metadata"
            );
            let _ = remove_file_with_parent_sync(&meta_path);
            return Ok(UndoLastRestoreAvailabilityReport {
                available: false,
                expires_at: None,
                message: "Undo Last Restore is unavailable because checkpoint files are missing."
                    .to_string(),
            });
        }
    };

    if let Err(error) = validate_undo_checkpoint_snapshot(&source_snapshot, snapshot_layout) {
        warn!(
            error = %error,
            snapshot_path = %source_snapshot.display(),
            "Undo checkpoint snapshot is incomplete or invalid; removing checkpoint"
        );
        let _ = remove_file_with_parent_sync(&meta_path);
        if let Err(remove_error) = remove_snapshot_dir(&source_snapshot) {
            warn!(
                error = %remove_error,
                path = %source_snapshot.display(),
                "Failed to remove invalid undo checkpoint snapshot"
            );
        }
        return Ok(UndoLastRestoreAvailabilityReport {
            available: false,
            expires_at: None,
            message: "Undo Last Restore is unavailable because checkpoint data is invalid."
                .to_string(),
        });
    }

    Ok(UndoLastRestoreAvailabilityReport {
        available: true,
        expires_at: Some(metadata.expires_at),
        message: "Undo Last Restore is available.".to_string(),
    })
}
pub(super) fn publish_undo_checkpoint(app_data_dir: &Path, snapshot_path: &Path) -> Result<(), String> {
    let meta_path = undo_checkpoint_meta_path(app_data_dir);

    if meta_path.exists() {
        if let Ok(previous_meta) = read_json_file::<UndoCheckpointMeta>(&meta_path) {
            match validated_runtime_snapshot_path(app_data_dir, &previous_meta.snapshot_path) {
                Ok(previous_snapshot) if previous_snapshot != snapshot_path => {
                    if let Err(error) = remove_snapshot_dir(&previous_snapshot) {
                        warn!(
                            error = %error,
                            path = %previous_snapshot.display(),
                            "Failed to remove previous undo checkpoint snapshot"
                        );
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    warn!(
                        error = %error,
                        snapshot_path = %previous_meta.snapshot_path,
                        "Previous undo checkpoint snapshot path is invalid; skipping cleanup"
                    );
                }
            }
        }
    }

    let now = Utc::now();
    let expires_at = now + Duration::days(UNDO_RETENTION_DAYS);
    let snapshot_layout = build_undo_snapshot_layout(snapshot_path)?;

    durable_write_json(
        &meta_path,
        &UndoCheckpointMeta {
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
            created_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            snapshot_layout: Some(snapshot_layout),
        },
    )
}

pub(super) fn build_undo_snapshot_layout(snapshot_path: &Path) -> Result<UndoCheckpointSnapshotLayout, String> {
    Ok(UndoCheckpointSnapshotLayout {
        history_db: snapshot_component_present(snapshot_path, HISTORY_DB_FILE, false)?,
        user_dictionary: snapshot_component_present(snapshot_path, USER_DICTIONARY_FILE, false)?,
        user_store: snapshot_component_present(snapshot_path, USER_STORE_DB_FILE, false)?,
        recordings_dir: snapshot_component_present(snapshot_path, RECORDINGS_DIR, true)?,
    })
}

pub(super) fn validate_undo_checkpoint_snapshot(
    snapshot_path: &Path,
    layout: &UndoCheckpointSnapshotLayout,
) -> Result<(), String> {
    if !layout.history_db && !layout.user_dictionary && !layout.user_store && !layout.recordings_dir
    {
        return Err("Checkpoint snapshot layout is empty".to_string());
    }

    validate_snapshot_component(
        snapshot_path,
        HISTORY_DB_FILE,
        false,
        layout.history_db,
    )?;
    validate_snapshot_component(
        snapshot_path,
        USER_DICTIONARY_FILE,
        false,
        layout.user_dictionary,
    )?;
    validate_snapshot_component(
        snapshot_path,
        USER_STORE_DB_FILE,
        false,
        layout.user_store,
    )?;
    validate_snapshot_component(snapshot_path, RECORDINGS_DIR, true, layout.recordings_dir)?;
    Ok(())
}

pub(super) fn validate_snapshot_component(
    root: &Path,
    relative_path: &str,
    expect_directory: bool,
    required: bool,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let present = snapshot_component_present(root, relative_path, expect_directory)?;
    if required && !present {
        return Err(format!(
            "Checkpoint snapshot is missing required path '{}'",
            path.display()
        ));
    }
    Ok(())
}

pub(super) fn snapshot_component_present(
    root: &Path,
    relative_path: &str,
    expect_directory: bool,
) -> Result<bool, String> {
    let path = root.join(relative_path);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Failed to inspect checkpoint path '{}': {error}",
                path.display()
            ));
        }
    };

    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Checkpoint path '{}' is a symbolic link",
            path.display()
        ));
    }

    if expect_directory {
        if !metadata.is_dir() {
            return Err(format!(
                "Checkpoint path '{}' is not a directory",
                path.display()
            ));
        }
    } else if !metadata.is_file() {
        return Err(format!(
            "Checkpoint path '{}' is not a file",
            path.display()
        ));
    }

    Ok(true)
}

pub(super) fn prune_expired_checkpoint(app_data_dir: &Path) {
    let meta_path = undo_checkpoint_meta_path(app_data_dir);
    if !meta_path.exists() {
        return;
    }

    let metadata = match read_json_file::<UndoCheckpointMeta>(&meta_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            warn!(
                error = %error,
                "Failed to parse undo checkpoint metadata; removing checkpoint"
            );
            let _ = remove_file_with_parent_sync(&meta_path);
            return;
        }
    };

    let Some(expiry) = parse_rfc3339(&metadata.expires_at) else {
        let _ = remove_file_with_parent_sync(&meta_path);
        return;
    };

    if expiry <= Utc::now() {
        match validated_runtime_snapshot_path(app_data_dir, &metadata.snapshot_path) {
            Ok(snapshot_path) => {
                if let Err(error) = remove_snapshot_dir(&snapshot_path) {
                    warn!(
                        error = %error,
                        path = %snapshot_path.display(),
                        "Failed to remove expired undo checkpoint snapshot"
                    );
                }
            }
            Err(error) => {
                warn!(
                    error = %error,
                    snapshot_path = %metadata.snapshot_path,
                    "Expired undo checkpoint snapshot path is invalid; skipping snapshot cleanup"
                );
            }
        }
        let _ = remove_file_with_parent_sync(&meta_path);
    }
}

pub(super) fn remove_snapshot_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    if !path.is_dir() {
        return Err(format!(
            "Snapshot path '{}' is not a directory",
            path.display()
        ));
    }

    fs::remove_dir_all(path)
        .map_err(|error| format!("Failed to remove snapshot directory '{}': {error}", path.display()))
}

pub(super) fn prune_stale_runtime_artifacts(app_data_dir: &Path) {
    let runtime_root = runtime_dir(app_data_dir);
    if !runtime_root.exists() {
        return;
    }

    let entries = match fs::read_dir(&runtime_root) {
        Ok(entries) => entries,
        Err(error) => {
            warn!(
                error = %error,
                path = %runtime_root.display(),
                "Failed to scan backup/restore runtime directory for stale artifacts"
            );
            return;
        }
    };

    let retention = StdDuration::from_secs((UNDO_RETENTION_DAYS.max(1) as u64) * 24 * 60 * 60);
    let now = SystemTime::now();
    let protected_paths = protected_runtime_paths(app_data_dir);

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warn!(error = %error, "Failed to read runtime directory entry");
                continue;
            }
        };

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        if protected_paths.contains(&path) {
            continue;
        }
        if !name.starts_with("snapshot-")
            && !name.starts_with("swap-old-")
            && !name.starts_with("restore-work-")
            && !name.starts_with("undo-work-")
            && !name.starts_with("export-workspace-")
        {
            continue;
        }

        let is_stale = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| now.duration_since(modified).ok())
            .map(|age| age >= retention)
            .unwrap_or(false);
        if !is_stale {
            continue;
        }

        if let Err(error) = fs::remove_dir_all(&path) {
            warn!(
                error = %error,
                path = %path.display(),
                "Failed to remove stale backup/restore runtime artifact"
            );
        }
    }
}

pub(super) fn prune_orphan_snapshot_dirs_keep_latest(app_data_dir: &Path) {
    let runtime_root = runtime_dir(app_data_dir);
    if !runtime_root.exists() {
        return;
    }

    let protected_paths = protected_runtime_paths(app_data_dir);

    let entries = match fs::read_dir(&runtime_root) {
        Ok(entries) => entries,
        Err(error) => {
            warn!(
                error = %error,
                path = %runtime_root.display(),
                "Failed to scan runtime directory for orphaned snapshots"
            );
            return;
        }
    };

    let mut orphan_snapshots: Vec<(u128, PathBuf)> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warn!(
                    error = %error,
                    "Failed to read runtime entry while pruning orphaned snapshots"
                );
                continue;
            }
        };

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if !name.starts_with("snapshot-") {
            continue;
        }
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }

        let canonical = fs::canonicalize(&path).unwrap_or(path.clone());
        if protected_paths.contains(&canonical) || protected_paths.contains(&path) {
            continue;
        }

        let modified_key = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or(0);

        orphan_snapshots.push((modified_key, canonical));
    }

    if orphan_snapshots.len() <= 1 {
        return;
    }

    orphan_snapshots.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
    });

    for (_, path) in orphan_snapshots.into_iter().skip(1) {
        if let Err(error) = remove_snapshot_dir(&path) {
            warn!(
                error = %error,
                path = %path.display(),
                "Failed to remove orphaned snapshot while keeping latest snapshot"
            );
        }
    }
}

pub(super) fn protected_runtime_paths(app_data_dir: &Path) -> BTreeSet<PathBuf> {
    let mut protected = BTreeSet::new();

    let marker_path = marker_path(app_data_dir);
    if marker_path.exists() {
        if let Ok(marker) = read_json_file::<RestoreMarker>(&marker_path) {
            if let Ok(snapshot) =
                validated_runtime_snapshot_path(app_data_dir, &marker.snapshot_path)
            {
                protected.insert(snapshot);
            }
        }
    }

    let checkpoint_meta_path = undo_checkpoint_meta_path(app_data_dir);
    if checkpoint_meta_path.exists() {
        if let Ok(meta) = read_json_file::<UndoCheckpointMeta>(&checkpoint_meta_path) {
            if let Ok(snapshot) =
                validated_runtime_snapshot_path(app_data_dir, &meta.snapshot_path)
            {
                protected.insert(snapshot);
            }
        }
    }

    protected
}

pub(super) fn validated_runtime_snapshot_path(app_data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let runtime_root = runtime_dir(app_data_dir);
    let runtime_root = fs::canonicalize(&runtime_root).map_err(|error| {
        format!(
            "Failed to resolve runtime directory '{}': {error}",
            runtime_root.display()
        )
    })?;

    let candidate_raw = PathBuf::from(raw_path);
    let candidate = if candidate_raw.is_absolute() {
        candidate_raw
    } else {
        runtime_root.join(candidate_raw)
    };

    if !candidate.exists() {
        return Err(format!(
            "Snapshot path '{}' does not exist",
            candidate.display()
        ));
    }

    let candidate_real = fs::canonicalize(&candidate).map_err(|error| {
        format!(
            "Failed to resolve snapshot path '{}': {error}",
            candidate.display()
        )
    })?;

    if !candidate_real.starts_with(&runtime_root) {
        return Err(format!(
            "Snapshot path '{}' is outside runtime directory '{}'",
            candidate_real.display(),
            runtime_root.display()
        ));
    }

    if !candidate_real.is_dir() {
        return Err(format!(
            "Snapshot path '{}' is not a directory",
            candidate_real.display()
        ));
    }

    let file_name = candidate_real
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            format!(
                "Snapshot path '{}' has an invalid directory name",
                candidate_real.display()
            )
        })?;
    if !file_name.starts_with("snapshot-") {
        return Err(format!(
            "Snapshot path '{}' does not use expected snapshot directory prefix",
            candidate_real.display()
        ));
    }

    Ok(candidate_real)
}
