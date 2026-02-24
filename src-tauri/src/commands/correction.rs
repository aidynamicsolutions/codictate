use crate::managers::correction::CorrectionManager;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tracing::warn;

#[tauri::command]
#[specta::specta]
pub fn accept_correction(app: AppHandle) -> Result<(), String> {
    let correction_manager = app.state::<Arc<CorrectionManager>>();
    if let Some(result) = correction_manager.get_last_result() {
        // This command path is UI-driven. Shortcut-driven acceptance is handled in fn_key_monitor.rs
        // and records `FeatureEntrypoint::Shortcut` directly on the manager.
        if let Err(error) =
            correction_manager.accept_correction(&result, crate::growth::FeatureEntrypoint::Ui)
        {
            // Manager already shows a user-visible notification on replacement failure.
            // Avoid propagating here to prevent double-error UX in command callers.
            warn!("accept_correction command handled replacement error: {error}");
        }
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
