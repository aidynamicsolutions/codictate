use tauri::{AppHandle, Window};

use crate::backup_restore::{
    self, ApplyRestoreReport, ApplyRestoreRequest, CreateBackupReport, CreateBackupRequest,
    BackupEstimateReport, PreflightRestoreReport, PreflightRestoreRequest,
    UndoLastRestoreAvailabilityReport,
    UndoLastRestoreReport, UndoLastRestoreRequest,
};

fn ensure_main_window<R: tauri::Runtime>(window: &Window<R>) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Backup and restore are only available from the main window.".to_string());
    }
    Ok(())
}

async fn run_backup_restore_task<T, F>(task_name: &str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("{task_name} task failed: {error}"))?
}

#[tauri::command]
#[specta::specta]
pub async fn create_backup(
    window: Window,
    app: AppHandle,
    request: CreateBackupRequest,
) -> Result<CreateBackupReport, String> {
    ensure_main_window(&window)?;
    run_backup_restore_task("create_backup", move || {
        backup_restore::create_backup(&app, request)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_backup_estimate(
    window: Window,
    app: AppHandle,
) -> Result<BackupEstimateReport, String> {
    ensure_main_window(&window)?;
    run_backup_restore_task("get_backup_estimate", move || {
        backup_restore::get_backup_estimate(&app)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn preflight_restore(
    window: Window,
    app: AppHandle,
    request: PreflightRestoreRequest,
) -> Result<PreflightRestoreReport, String> {
    ensure_main_window(&window)?;
    run_backup_restore_task("preflight_restore", move || {
        backup_restore::preflight_restore(&app, request)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_restore(
    window: Window,
    app: AppHandle,
    request: ApplyRestoreRequest,
) -> Result<ApplyRestoreReport, String> {
    ensure_main_window(&window)?;
    run_backup_restore_task("apply_restore", move || {
        backup_restore::apply_restore(&app, request)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn undo_last_restore(
    window: Window,
    app: AppHandle,
    request: UndoLastRestoreRequest,
) -> Result<UndoLastRestoreReport, String> {
    ensure_main_window(&window)?;
    run_backup_restore_task("undo_last_restore", move || {
        backup_restore::undo_last_restore(&app, request)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_undo_last_restore_availability(
    window: Window,
    app: AppHandle,
) -> Result<UndoLastRestoreAvailabilityReport, String> {
    ensure_main_window(&window)?;
    run_backup_restore_task("get_undo_last_restore_availability", move || {
        backup_restore::undo_last_restore_availability(&app)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::WebviewUrl;

    #[test]
    fn backup_command_rejects_non_main_window() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("build test app");

        tauri::WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
            .build()
            .expect("create main window");
        let secondary = tauri::WebviewWindowBuilder::new(&app, "secondary", WebviewUrl::default())
            .build()
            .expect("create secondary window");

        let secondary_window = secondary.as_ref().window();
        let result = ensure_main_window(&secondary_window);
        assert_eq!(
            result.expect_err("secondary window should be rejected"),
            "Backup and restore are only available from the main window."
        );
    }
}
