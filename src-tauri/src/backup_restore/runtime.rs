//! Runtime coordination for backup/restore operations.
//!
//! This module owns operation lifecycle, maintenance-mode state transitions,
//! cancellation flags, and write gating helpers used by managed mutating paths.

use super::*;
use std::sync::OnceLock;
use std::time::Instant;

const TRANSCRIPTION_START_BLOCK_NOTICE_COOLDOWN: StdDuration = StdDuration::from_secs(5);
static TRANSCRIPTION_BLOCK_NOTICE_LAST_SHOWN: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

pub(super) struct OperationGuard<R: tauri::Runtime> {
    app: AppHandle<R>,
}

impl<R: tauri::Runtime> Drop for OperationGuard<R> {
    fn drop(&mut self) {
        if let Some(runtime) = self.app.try_state::<BackupRestoreRuntime>() {
            runtime.maintenance_mode.store(false, Ordering::SeqCst);
            runtime.cancel_requested.store(false, Ordering::SeqCst);
            runtime.operation_in_progress.store(false, Ordering::SeqCst);
        }
        let _ = self.app.emit("backup-maintenance-mode-changed", false);
    }
}

pub fn is_maintenance_mode<R: tauri::Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<BackupRestoreRuntime>()
        .map(|state| state.maintenance_mode.load(Ordering::SeqCst))
        .unwrap_or(false)
}

pub fn is_operation_in_progress<R: tauri::Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<BackupRestoreRuntime>()
        .map(|state| state.operation_in_progress.load(Ordering::SeqCst))
        .unwrap_or(false)
}

pub fn can_start_transcription<R: tauri::Runtime>(app: &AppHandle<R>) -> bool {
    !is_operation_in_progress(app) && !is_maintenance_mode(app)
}

pub fn ensure_transcription_start_allowed(app: &AppHandle) -> bool {
    if can_start_transcription(app) {
        return true;
    }

    if should_emit_transcription_block_notice_at(Instant::now()) {
        crate::notification::show_info(app, "settings.backup.operation.transcriptionBlocked");
    }

    false
}

pub(super) fn should_emit_transcription_block_notice_at(now: Instant) -> bool {
    let state = TRANSCRIPTION_BLOCK_NOTICE_LAST_SHOWN.get_or_init(|| Mutex::new(None));
    let mut last_shown = match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if let Some(last) = *last_shown {
        if now.saturating_duration_since(last) < TRANSCRIPTION_START_BLOCK_NOTICE_COOLDOWN {
            return false;
        }
    }

    *last_shown = Some(now);
    true
}

#[cfg(test)]
pub(super) fn reset_transcription_block_notice_for_tests() {
    let state = TRANSCRIPTION_BLOCK_NOTICE_LAST_SHOWN.get_or_init(|| Mutex::new(None));
    let mut last_shown = match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *last_shown = None;
}

#[cfg(test)]
pub fn assert_writes_allowed<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if is_operation_in_progress(app) || is_maintenance_mode(app) {
        return Err(WRITES_BLOCKED_MESSAGE.to_string());
    }
    Ok(())
}

pub fn with_write_permit<R, F, T>(app: &AppHandle<R>, operation: F) -> Result<T, String>
where
    R: tauri::Runtime,
    F: FnOnce() -> Result<T, String>,
{
    let runtime = app.state::<BackupRestoreRuntime>();
    if runtime.operation_in_progress.load(Ordering::SeqCst)
        || runtime.maintenance_mode.load(Ordering::SeqCst)
    {
        return Err(WRITES_BLOCKED_MESSAGE.to_string());
    }

    let _guard = runtime
        .write_gate
        .lock()
        .map_err(|_| "Failed to acquire backup/restore write gate.".to_string())?;

    // Re-check after acquiring the gate so writes that passed the optimistic
    // pre-check cannot continue once an operation has started transitioning.
    if runtime.operation_in_progress.load(Ordering::SeqCst)
        || runtime.maintenance_mode.load(Ordering::SeqCst)
    {
        return Err(WRITES_BLOCKED_MESSAGE.to_string());
    }

    operation()
}

pub(super) fn ensure_supported_platform() -> Result<(), String> {
    if current_platform() != "macos" {
        return Err(
            "Backup and restore are currently available on macOS only.".to_string(),
        );
    }

    Ok(())
}

pub fn request_cancel<R: tauri::Runtime>(app: &AppHandle<R>) {
    if let Some(runtime) = app.try_state::<BackupRestoreRuntime>() {
        if runtime.operation_in_progress.load(Ordering::SeqCst) {
            runtime.cancel_requested.store(true, Ordering::SeqCst);
            info!("Backup/restore cancellation requested");
        }
    }
}

pub(super) fn start_operation<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<OperationGuard<R>, String> {
    let runtime = app.state::<BackupRestoreRuntime>();

    if runtime
        .operation_in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Another backup or restore operation is already running.".to_string());
    }

    runtime.cancel_requested.store(false, Ordering::SeqCst);

    if let Err(error) = ensure_idle(app) {
        reset_operation_state(app, &runtime);
        return Err(error);
    }

    if let Err(error) = acquire_operation_gate(&runtime) {
        reset_operation_state(app, &runtime);
        return Err(error);
    }

    runtime.maintenance_mode.store(true, Ordering::SeqCst);
    let _ = app.emit("backup-maintenance-mode-changed", true);

    Ok(OperationGuard { app: app.clone() })
}

fn acquire_operation_gate(runtime: &BackupRestoreRuntime) -> Result<(), String> {
    let guard = runtime
        .write_gate
        .lock()
        .map_err(|_| "Failed to acquire backup/restore operation gate.".to_string())?;
    drop(guard);
    Ok(())
}

fn reset_operation_state<R: tauri::Runtime>(
    app: &AppHandle<R>,
    runtime: &BackupRestoreRuntime,
) {
    runtime.cancel_requested.store(false, Ordering::SeqCst);
    runtime.maintenance_mode.store(false, Ordering::SeqCst);
    runtime.operation_in_progress.store(false, Ordering::SeqCst);
    let _ = app.emit("backup-maintenance-mode-changed", false);
}

pub(super) fn ensure_idle<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(audio_manager) = app.try_state::<Arc<AudioRecordingManager>>() {
        if audio_manager.is_recording() {
            return Err("Stop recording before starting backup or restore.".to_string());
        }
    }

    if let Some(transcription_manager) = app.try_state::<Arc<TranscriptionManager>>() {
        if transcription_manager.is_any_session_active() {
            return Err(
                "Wait until transcription finishes before starting backup or restore.".to_string(),
            );
        }
    }

    Ok(())
}

pub(super) fn ensure_not_cancelled<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(runtime) = app.try_state::<BackupRestoreRuntime>() {
        if runtime.cancel_requested.load(Ordering::SeqCst) {
            return Err("Backup/restore was cancelled safely.".to_string());
        }
    }
    Ok(())
}

pub(super) fn emit_progress<R: tauri::Runtime>(
    app: &AppHandle<R>,
    operation: &str,
    phase: &str,
    current: u64,
    total: u64,
) {
    let _ = app.emit(
        BACKUP_PROGRESS_EVENT,
        BackupProgress {
            operation: operation.to_string(),
            phase: phase.to_string(),
            current,
            total,
        },
    );
}
