//! Tauri commands for MLX Local AI Model Management
//!
//! These commands expose the MlxModelManager functionality to the frontend.

use crate::managers::mlx::{MlxModelInfo, MlxModelManager};
use std::sync::Arc;
use tauri::State;

/// List all available MLX models with their current status
#[tauri::command]
#[specta::specta]
pub async fn mlx_list_models(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
) -> Result<Vec<MlxModelInfo>, String> {
    Ok(mlx_manager.list_models())
}

/// Get the status of a specific MLX model
#[tauri::command]
#[specta::specta]
pub async fn mlx_get_model_status(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
    model_id: String,
) -> Result<Option<MlxModelInfo>, String> {
    Ok(mlx_manager.get_model_status(&model_id))
}

/// Start downloading an MLX model from Hugging Face Hub
#[tauri::command]
#[specta::specta]
pub async fn mlx_download_model(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
    model_id: String,
) -> Result<(), String> {
    mlx_manager
        .download_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

/// Cancel an in-progress MLX model download
#[tauri::command]
#[specta::specta]
pub fn mlx_cancel_download(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
) -> Result<(), String> {
    mlx_manager.cancel_download().map_err(|e| e.to_string())
}

/// Retry a failed MLX model download (max 3 attempts)
#[tauri::command]
#[specta::specta]
pub async fn mlx_retry_download(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
) -> Result<(), String> {
    mlx_manager
        .retry_download()
        .await
        .map_err(|e| e.to_string())
}

/// Delete a downloaded MLX model
/// Returns an error if the model is currently in use
#[tauri::command]
#[specta::specta]
pub fn mlx_delete_model(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
    model_id: String,
) -> Result<(), String> {
    mlx_manager
        .delete_model(&model_id)
        .map_err(|e| e.to_string())
}

/// Process text using the loaded MLX model
/// This is used for transcription post-processing
#[tauri::command]
#[specta::specta]
pub async fn mlx_process_text(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
    prompt: String,
) -> Result<String, String> {
    mlx_manager
        .process_text(&prompt)
        .await
        .map_err(|e| e.to_string())
}

/// Check if any MLX model operation is currently in progress
#[tauri::command]
#[specta::specta]
pub fn mlx_is_busy(mlx_manager: State<'_, Arc<MlxModelManager>>) -> bool {
    mlx_manager.is_busy()
}

/// Unload the currently loaded MLX model to free memory
#[tauri::command]
#[specta::specta]
pub fn mlx_unload_model(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
) -> Result<(), String> {
    mlx_manager.unload_model().map_err(|e| e.to_string())
}

/// Switch to a different MLX model
/// Unloads the current model and prepares for the new one to be loaded on next use
#[tauri::command]
#[specta::specta]
pub fn mlx_switch_model(
    mlx_manager: State<'_, Arc<MlxModelManager>>,
    model_id: String,
) -> Result<(), String> {
    mlx_manager
        .switch_model(&model_id)
        .map_err(|e| e.to_string())
}
