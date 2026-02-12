use crate::managers::correction::CorrectionManager;
use tauri::{AppHandle, Manager};
use std::sync::Arc;

#[tauri::command]
#[specta::specta]
pub fn accept_correction(app: AppHandle) -> Result<(), String> {
    let correction_manager = app.state::<Arc<CorrectionManager>>();
    if let Some(result) = correction_manager.get_last_result() {
        correction_manager.accept_correction(&result);
        // Hide overlay after accepting
        crate::utils::hide_recording_overlay(&app);
        Ok(())
    } else {
        Err("No correction result available".to_string())
    }
}

#[tauri::command]
#[specta::specta]
pub fn dismiss_correction(app: AppHandle) {
    let correction_manager = app.state::<Arc<CorrectionManager>>();
    correction_manager.dismiss_correction();
    crate::utils::hide_recording_overlay(&app);
}
