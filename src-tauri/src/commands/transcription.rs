use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, write_settings, ModelUnloadTimeout};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Serialize, Type)]
pub struct ModelLoadStatus {
    is_loaded: bool,
    is_loading: bool,
    is_warmed: bool,
    is_warming: bool,
    current_model: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub fn set_model_unload_timeout(app: AppHandle, timeout: ModelUnloadTimeout) {
    let mut settings = get_settings(&app);
    settings.model_unload_timeout = timeout;
    write_settings(&app, settings);
}

#[tauri::command]
#[specta::specta]
pub fn get_model_load_status(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
) -> Result<ModelLoadStatus, String> {
    Ok(ModelLoadStatus {
        is_loaded: transcription_manager.is_model_loaded(),
        is_loading: transcription_manager.is_model_loading(),
        is_warmed: transcription_manager.is_model_warmed(),
        is_warming: transcription_manager.is_model_warming(),
        current_model: transcription_manager.get_current_model(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn warm_up_transcription_model(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    model_id: Option<String>,
) -> Result<(), String> {
    match model_id {
        Some(id) if !id.is_empty() => {
            transcription_manager.initiate_model_warmup_for_model(id, "frontend_onboarding");
        }
        _ => {
            transcription_manager.initiate_model_warmup("frontend_onboarding");
        }
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn unload_model_manually(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
) -> Result<(), String> {
    transcription_manager
        .unload_model()
        .map_err(|e| format!("Failed to unload model: {}", e))
}
