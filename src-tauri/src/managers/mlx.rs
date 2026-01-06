//! MLX Local AI Model Manager for Apple Silicon Macs
//!
//! This module provides local LLM inference using Apple's MLX framework
//! for transcription post-processing on Apple Silicon Macs.

use anyhow::{anyhow, Result};
use hf_hub::api::tokio::Api;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::settings::get_settings;

/// Unique identifier for the MLX local provider
pub const LOCAL_MLX_PROVIDER_ID: &str = "local_mlx";

/// Default model to use if none is selected
pub const DEFAULT_MLX_MODEL_ID: &str = "qwen3_base_1.7b";

/// Model status enum representing the lifecycle of a model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MlxModelStatus {
    /// Model is not downloaded
    NotDownloaded,
    /// Model is currently being downloaded
    Downloading,
    /// Model download failed
    DownloadFailed,
    /// Model is downloaded but not loaded
    Downloaded,
    /// Model is currently being loaded into memory
    Loading,
    /// Model is loaded and ready for inference
    Ready,
    /// Model loading failed
    LoadFailed,
}

/// Information about an available MLX model
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MlxModelInfo {
    /// Unique model identifier
    pub id: String,
    /// Display name for the UI
    pub display_name: String,
    /// Model description
    pub description: String,
    /// Hugging Face repository ID
    pub hf_repo: String,
    /// Approximate size in bytes when downloaded
    pub size_bytes: u64,
    /// Current status of the model
    pub status: MlxModelStatus,
    /// Download progress (0.0 - 1.0) when downloading
    pub download_progress: f64,
    /// Whether this is the default recommended model
    pub is_default: bool,
    /// Number of parameters (for display)
    pub parameters: String,
}

/// Event payload for model state changes
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "event_type")]
pub enum MlxModelStateEvent {
    #[serde(rename = "download_started")]
    DownloadStarted { model_id: String },
    #[serde(rename = "download_progress")]
    DownloadProgress {
        model_id: String,
        progress: f64,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    #[serde(rename = "download_completed")]
    DownloadCompleted { model_id: String },
    #[serde(rename = "download_failed")]
    DownloadFailed { model_id: String, error: String },
    #[serde(rename = "download_cancelled")]
    DownloadCancelled { model_id: String },
    #[serde(rename = "loading_started")]
    LoadingStarted { model_id: String },
    #[serde(rename = "loading_completed")]
    LoadingCompleted { model_id: String },
    #[serde(rename = "loading_failed")]
    LoadingFailed { model_id: String, error: String },
    #[serde(rename = "unloaded")]
    Unloaded { model_id: String },
    #[serde(rename = "error")]
    Error { model_id: String, error: String },
}

/// Internal state for tracking download operations
struct DownloadState {
    model_id: String,
    cancel_sender: watch::Sender<bool>,
    retry_count: u8,
}

/// Internal state for a loaded model
struct LoadedModelState {
    model_id: String,
    last_used: Instant,
    // The actual model will be stored here when mlx-lm API is implemented
    // For now, we'll use a placeholder
}

/// Manager for MLX-based local AI models
pub struct MlxModelManager {
    app_handle: AppHandle,
    models_dir: PathBuf,
    /// Available models registry
    models: RwLock<HashMap<String, MlxModelInfo>>,
    /// Currently active download operation
    current_download: Mutex<Option<DownloadState>>,
    /// Currently loaded model
    loaded_model: RwLock<Option<LoadedModelState>>,
    /// Unload timer handle
    unload_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl MlxModelManager {
    /// Create a new MlxModelManager instance
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create models directory in app data
        let models_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow!("Failed to get app data dir: {}", e))?
            .join("mlx-models");

        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            models_dir,
            models: RwLock::new(Self::create_model_registry()),
            current_download: Mutex::new(None),
            loaded_model: RwLock::new(None),
            unload_task: Mutex::new(None),
        };

        // Update status based on what's already downloaded
        manager.update_download_status()?;

        Ok(manager)
    }

    /// Create the initial model registry with all available models
    fn create_model_registry() -> HashMap<String, MlxModelInfo> {
        let mut models = HashMap::new();

        // Qwen 3 family
        models.insert(
            "qwen3_base_0.6b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_0.6b".to_string(),
                display_name: "Qwen 3 Base 0.6B".to_string(),
                description: "Fastest, smallest model. Good for quick corrections.".to_string(),
                hf_repo: "mlx-community/Qwen3-0.6B-4bit".to_string(),
                size_bytes: 400 * 1024 * 1024, // ~400 MB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "0.6B".to_string(),
            },
        );

        models.insert(
            "qwen3_base_1.7b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_1.7b".to_string(),
                display_name: "Qwen 3 Base 1.7B".to_string(),
                description: "Best balance of speed and quality. Recommended for most users."
                    .to_string(),
                hf_repo: "mlx-community/Qwen3-1.7B-4bit".to_string(),
                size_bytes: 1024 * 1024 * 1024, // ~1 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: true,
                parameters: "1.7B".to_string(),
            },
        );

        models.insert(
            "qwen3_base_4b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_4b".to_string(),
                display_name: "Qwen 3 Base 4B".to_string(),
                description: "Higher quality for powerful machines.".to_string(),
                hf_repo: "mlx-community/Qwen3-4B-4bit".to_string(),
                size_bytes: 2300 * 1024 * 1024, // ~2.3 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "4B".to_string(),
            },
        );

        models.insert(
            "qwen3_base_8b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_8b".to_string(),
                display_name: "Qwen 3 Base 8B".to_string(),
                description: "Best quality. Requires 16GB+ RAM.".to_string(),
                hf_repo: "mlx-community/Qwen3-8B-4bit".to_string(),
                size_bytes: 4700 * 1024 * 1024, // ~4.7 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "8B".to_string(),
            },
        );

        // Gemma 3 family
        models.insert(
            "gemma3_base_1b".to_string(),
            MlxModelInfo {
                id: "gemma3_base_1b".to_string(),
                display_name: "Gemma 3 Base 1B".to_string(),
                description: "Small multi-language model.".to_string(),
                hf_repo: "mlx-community/gemma-3-1b-it-4bit".to_string(),
                size_bytes: 800 * 1024 * 1024, // ~800 MB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "1B".to_string(),
            },
        );

        models.insert(
            "gemma3_base_4b".to_string(),
            MlxModelInfo {
                id: "gemma3_base_4b".to_string(),
                display_name: "Gemma 3 Base 4B".to_string(),
                description: "Multi-language support with good quality.".to_string(),
                hf_repo: "mlx-community/gemma-3-4b-it-4bit".to_string(),
                size_bytes: 2300 * 1024 * 1024, // ~2.3 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "4B".to_string(),
            },
        );

        // SmolLM 3
        models.insert(
            "smollm3_base_3b".to_string(),
            MlxModelInfo {
                id: "smollm3_base_3b".to_string(),
                display_name: "SmolLM 3 Base 3B".to_string(),
                description: "Multi-language alternative with good performance.".to_string(),
                hf_repo: "mlx-community/SmolLM2-1.7B-Instruct-4bit".to_string(),
                size_bytes: 1800 * 1024 * 1024, // ~1.8 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "3B".to_string(),
            },
        );

        models
    }

    /// Update the download status of all models based on what's on disk
    fn update_download_status(&self) -> Result<()> {
        let mut models = self.models.write().map_err(|e| anyhow!("Lock error: {}", e))?;

        for model in models.values_mut() {
            let model_path = self.models_dir.join(&model.id);

            if model_path.exists() && model_path.is_dir() {
                // Check if the directory has files (not empty)
                let has_files = fs::read_dir(&model_path)
                    .map(|mut entries| entries.next().is_some())
                    .unwrap_or(false);

                if has_files {
                    // Model directory exists with files - mark as downloaded
                    if model.status == MlxModelStatus::NotDownloaded
                        || model.status == MlxModelStatus::DownloadFailed
                    {
                        model.status = MlxModelStatus::Downloaded;
                        model.download_progress = 1.0;
                    }
                } else {
                    // Empty directory - clean up and mark as not downloaded
                    warn!(
                        "Found empty model directory for {}, cleaning up",
                        model.id
                    );
                    let _ = fs::remove_dir(&model_path);
                    model.status = MlxModelStatus::NotDownloaded;
                    model.download_progress = 0.0;
                }
            } else {
                // No directory - not downloaded
                if model.status != MlxModelStatus::Downloading {
                    model.status = MlxModelStatus::NotDownloaded;
                    model.download_progress = 0.0;
                }
            }
        }

        Ok(())
    }

    /// Get all available models
    pub fn list_models(&self) -> Vec<MlxModelInfo> {
        let models = self.models.read().unwrap();
        let mut list: Vec<MlxModelInfo> = models.values().cloned().collect();
        // Sort by size (smallest first), but put default first
        list.sort_by(|a, b| {
            if a.is_default && !b.is_default {
                std::cmp::Ordering::Less
            } else if !a.is_default && b.is_default {
                std::cmp::Ordering::Greater
            } else {
                a.size_bytes.cmp(&b.size_bytes)
            }
        });
        list
    }

    /// Get status of a specific model
    pub fn get_model_status(&self, model_id: &str) -> Option<MlxModelInfo> {
        let models = self.models.read().unwrap();
        models.get(model_id).cloned()
    }

    /// Emit a model state event to the frontend
    fn emit_event(&self, event: MlxModelStateEvent) {
        if let Err(e) = self.app_handle.emit("mlx-model-state-changed", &event) {
            error!("Failed to emit MLX model event: {}", e);
        }
    }

    /// Start downloading a model from Hugging Face Hub
    pub async fn download_model(&self, model_id: &str) -> Result<()> {
        // Check if model exists in registry
        let model_info = {
            let models = self.models.read().map_err(|e| anyhow!("Lock error: {}", e))?;
            models
                .get(model_id)
                .cloned()
                .ok_or_else(|| anyhow!("Model not found: {}", model_id))?
        };

        // Check if already downloading
        {
            let current = self.current_download.lock().unwrap();
            if current.is_some() {
                return Err(anyhow!("Another download is already in progress"));
            }
        }

        // Check if already downloaded
        if model_info.status == MlxModelStatus::Downloaded
            || model_info.status == MlxModelStatus::Ready
        {
            info!("Model {} is already downloaded", model_id);
            return Ok(());
        }

        // Set up cancellation channel
        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Store download state
        {
            let mut current = self.current_download.lock().unwrap();
            *current = Some(DownloadState {
                model_id: model_id.to_string(),
                cancel_sender: cancel_tx,
                retry_count: 0,
            });
        }

        // Update status to downloading
        {
            let mut models = self.models.write().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.status = MlxModelStatus::Downloading;
                model.download_progress = 0.0;
            }
        }

        self.emit_event(MlxModelStateEvent::DownloadStarted {
            model_id: model_id.to_string(),
        });

        // Perform the download
        let result = self
            .perform_download(model_id, &model_info.hf_repo, cancel_rx)
            .await;

        // Clear download state
        {
            let mut current = self.current_download.lock().unwrap();
            *current = None;
        }

        match result {
            Ok(()) => {
                // Update status to downloaded
                {
                    let mut models = self.models.write().unwrap();
                    if let Some(model) = models.get_mut(model_id) {
                        model.status = MlxModelStatus::Downloaded;
                        model.download_progress = 1.0;
                    }
                }
                self.emit_event(MlxModelStateEvent::DownloadCompleted {
                    model_id: model_id.to_string(),
                });
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("cancelled") {
                    // Download was cancelled
                    {
                        let mut models = self.models.write().unwrap();
                        if let Some(model) = models.get_mut(model_id) {
                            model.status = MlxModelStatus::NotDownloaded;
                            model.download_progress = 0.0;
                        }
                    }
                    self.emit_event(MlxModelStateEvent::DownloadCancelled {
                        model_id: model_id.to_string(),
                    });
                } else {
                    // Download failed
                    {
                        let mut models = self.models.write().unwrap();
                        if let Some(model) = models.get_mut(model_id) {
                            model.status = MlxModelStatus::DownloadFailed;
                        }
                    }
                    self.emit_event(MlxModelStateEvent::DownloadFailed {
                        model_id: model_id.to_string(),
                        error: error_msg.clone(),
                    });
                }
                Err(e)
            }
        }
    }

    /// Internal method to perform the actual download
    async fn perform_download(
        &self,
        model_id: &str,
        hf_repo: &str,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        info!("Starting download of model {} from {}", model_id, hf_repo);

        let model_dir = self.models_dir.join(model_id);

        // Create model directory
        if !model_dir.exists() {
            fs::create_dir_all(&model_dir)?;
        }

        // Use hf-hub API to download model files
        let api = Api::new()?;
        let repo = api.model(hf_repo.to_string());

        // Get list of files to download
        // For MLX models, we typically need: config.json, tokenizer files, and model weights
        let required_files = vec![
            "config.json",
            "tokenizer.json",
            "tokenizer_config.json",
            "model.safetensors",
            "model.safetensors.index.json",
        ];

        let mut downloaded_bytes: u64 = 0;
        let total_files = required_files.len();

        for (idx, filename) in required_files.iter().enumerate() {
            // Check for cancellation
            if *cancel_rx.borrow() {
                // Clean up partial download
                let _ = fs::remove_dir_all(&model_dir);
                return Err(anyhow!("Download cancelled"));
            }

            debug!("Downloading {} for model {}", filename, model_id);

            // Try to download the file
            match repo.get(filename).await {
                Ok(path) => {
                    // Copy to our model directory
                    let dest_path = model_dir.join(filename);
                    if let Err(e) = fs::copy(&path, &dest_path) {
                        // Some files may be optional, log and continue
                        if filename.contains("index") {
                            debug!("Optional file {} not available: {}", filename, e);
                            continue;
                        }
                        warn!("Failed to copy {}: {}", filename, e);
                    }

                    // Update progress
                    downloaded_bytes += 1; // Simplified progress
                    let progress = (idx + 1) as f64 / total_files as f64;

                    {
                        let mut models = self.models.write().unwrap();
                        if let Some(model) = models.get_mut(model_id) {
                            model.download_progress = progress;
                        }
                    }

                    self.emit_event(MlxModelStateEvent::DownloadProgress {
                        model_id: model_id.to_string(),
                        progress,
                        downloaded_bytes,
                        total_bytes: total_files as u64,
                    });
                }
                Err(e) => {
                    // config.json and model weights are required
                    if *filename == "config.json" || filename.starts_with("model.") {
                        error!("Failed to download required file {}: {}", filename, e);
                        let _ = fs::remove_dir_all(&model_dir);
                        return Err(anyhow!("Failed to download {}: {}", filename, e));
                    }
                    // Other files might be optional
                    debug!("Optional file {} not available: {}", filename, e);
                }
            }
        }

        info!("Successfully downloaded model {} to {:?}", model_id, model_dir);
        Ok(())
    }

    /// Cancel an in-progress download
    pub fn cancel_download(&self) -> Result<()> {
        let current = self.current_download.lock().unwrap();
        if let Some(state) = current.as_ref() {
            let _ = state.cancel_sender.send(true);
            info!("Sent cancel signal for download of {}", state.model_id);
            Ok(())
        } else {
            Err(anyhow!("No download in progress"))
        }
    }

    /// Retry a failed download (max 3 attempts)
    pub async fn retry_download(&self) -> Result<()> {
        let model_id = {
            let mut current = self.current_download.lock().unwrap();
            if let Some(ref mut state) = *current {
                if state.retry_count >= 3 {
                    return Err(anyhow!("Maximum retry attempts (3) reached"));
                }
                state.retry_count += 1;
                state.model_id.clone()
            } else {
                // Check if there's a failed download we can retry
                let models = self.models.read().unwrap();
                models
                    .values()
                    .find(|m| m.status == MlxModelStatus::DownloadFailed)
                    .map(|m| m.id.clone())
                    .ok_or_else(|| anyhow!("No failed download to retry"))?
            }
        };

        self.download_model(&model_id).await
    }

    /// Delete a downloaded model
    pub fn delete_model(&self, model_id: &str) -> Result<()> {
        // Check if model is busy
        {
            let current = self.current_download.lock().unwrap();
            if let Some(ref state) = *current {
                if state.model_id == model_id {
                    return Err(anyhow!("Cannot delete model while downloading"));
                }
            }
        }

        {
            let loaded = self.loaded_model.read().unwrap();
            if let Some(ref state) = *loaded {
                if state.model_id == model_id {
                    return Err(anyhow!("Cannot delete model while it is loaded"));
                }
            }
        }

        // Check if model exists in registry
        {
            let models = self.models.read().unwrap();
            if !models.contains_key(model_id) {
                return Err(anyhow!("Model not found: {}", model_id));
            }
        }

        // Delete the model directory
        let model_path = self.models_dir.join(model_id);
        if model_path.exists() {
            fs::remove_dir_all(&model_path)?;
            info!("Deleted model {} at {:?}", model_id, model_path);
        }

        // Update status
        {
            let mut models = self.models.write().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.status = MlxModelStatus::NotDownloaded;
                model.download_progress = 0.0;
            }
        }

        Ok(())
    }

    /// Check if any model is currently busy (downloading, loading, or running)
    pub fn is_busy(&self) -> bool {
        let current = self.current_download.lock().unwrap();
        if current.is_some() {
            return true;
        }

        let models = self.models.read().unwrap();
        models
            .values()
            .any(|m| m.status == MlxModelStatus::Loading)
    }

    /// Process text using the loaded model
    pub async fn process_text(&self, prompt: &str) -> Result<String> {
        // Check if a model is loaded
        let model_id = {
            let loaded = self.loaded_model.read().unwrap();
            match loaded.as_ref() {
                Some(state) => state.model_id.clone(),
                None => {
                    // Try to load the selected model
                    drop(loaded);
                    let settings = get_settings(&self.app_handle);
                    let selected = settings
                        .post_process_models
                        .get(LOCAL_MLX_PROVIDER_ID)
                        .cloned()
                        .unwrap_or_else(|| DEFAULT_MLX_MODEL_ID.to_string());

                    // Check if model is downloaded
                    let model_info = self.get_model_status(&selected);
                    match model_info {
                        Some(info) if info.status == MlxModelStatus::Downloaded => {
                            // Load the model
                            self.load_model(&selected)?;
                            selected
                        }
                        Some(info) if info.status == MlxModelStatus::Ready => selected,
                        _ => {
                            return Err(anyhow!(
                                "Selected MLX model is not downloaded: {}",
                                selected
                            ));
                        }
                    }
                }
            }
        };

        // Update last used time
        {
            let mut loaded = self.loaded_model.write().unwrap();
            if let Some(ref mut state) = *loaded {
                state.last_used = Instant::now();
            }
        }

        // Reset unload timer
        self.reset_unload_timer();

        // TODO: Implement actual text generation using mlx-lm
        // For now, return the original text as a placeholder
        // This will be replaced with actual MLX inference
        debug!(
            "Processing text with model {} (placeholder - MLX inference not yet implemented)",
            model_id
        );

        // Placeholder: Return the input unchanged
        // In production, this will use mlx_lm::generate() or similar
        Ok(prompt.to_string())
    }

    /// Load a model into memory
    fn load_model(&self, model_id: &str) -> Result<()> {
        info!("Loading MLX model: {}", model_id);

        // Update status to loading
        {
            let mut models = self.models.write().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.status = MlxModelStatus::Loading;
            }
        }

        self.emit_event(MlxModelStateEvent::LoadingStarted {
            model_id: model_id.to_string(),
        });

        // TODO: Implement actual model loading using mlx-lm
        // For now, just mark as loaded
        let model_path = self.models_dir.join(model_id);
        if !model_path.exists() {
            let mut models = self.models.write().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.status = MlxModelStatus::DownloadFailed;
            }
            self.emit_event(MlxModelStateEvent::LoadingFailed {
                model_id: model_id.to_string(),
                error: "Model directory not found".to_string(),
            });
            return Err(anyhow!("Model directory not found: {}", model_id));
        }

        // Store loaded model state
        {
            let mut loaded = self.loaded_model.write().unwrap();
            *loaded = Some(LoadedModelState {
                model_id: model_id.to_string(),
                last_used: Instant::now(),
            });
        }

        // Update status to ready
        {
            let mut models = self.models.write().unwrap();
            if let Some(model) = models.get_mut(model_id) {
                model.status = MlxModelStatus::Ready;
            }
        }

        self.emit_event(MlxModelStateEvent::LoadingCompleted {
            model_id: model_id.to_string(),
        });

        // Start unload timer
        self.reset_unload_timer();

        info!("MLX model {} loaded successfully", model_id);
        Ok(())
    }

    /// Unload the currently loaded model
    pub fn unload_model(&self) -> Result<()> {
        let model_id = {
            let mut loaded = self.loaded_model.write().unwrap();
            match loaded.take() {
                Some(state) => state.model_id,
                None => return Ok(()), // No model loaded
            }
        };

        info!("Unloading MLX model: {}", model_id);

        // Update status to downloaded
        {
            let mut models = self.models.write().unwrap();
            if let Some(model) = models.get_mut(&model_id) {
                model.status = MlxModelStatus::Downloaded;
            }
        }

        self.emit_event(MlxModelStateEvent::Unloaded { model_id });

        Ok(())
    }

    /// Reset the unload timer based on settings
    fn reset_unload_timer(&self) {
        let settings = get_settings(&self.app_handle);
        let timeout = settings.model_unload_timeout;

        // Cancel existing timer
        {
            let mut task = self.unload_task.lock().unwrap();
            if let Some(handle) = task.take() {
                handle.abort();
            }
        }

        // If timeout is "never", don't set a timer
        let timeout_secs = match timeout.to_seconds() {
            Some(secs) if secs > 0 => secs,
            _ => return, // Never unload or immediate (handled elsewhere)
        };

        // Clone what we need for the async task
        let app_handle = self.app_handle.clone();
        let loaded_model = Arc::new(RwLock::new(None::<String>));

        // Get the model ID
        {
            let loaded = self.loaded_model.read().unwrap();
            if let Some(ref state) = *loaded {
                let mut lm = loaded_model.write().unwrap();
                *lm = Some(state.model_id.clone());
            }
        }

        // Spawn unload task
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(timeout_secs)).await;

            // Get the manager from app state and unload
            if let Some(manager) = app_handle.try_state::<Arc<MlxModelManager>>() {
                if let Err(e) = manager.unload_model() {
                    error!("Failed to unload MLX model: {}", e);
                }
            }
        });

        // Store the task handle
        {
            let mut task = self.unload_task.lock().unwrap();
            *task = Some(handle);
        }
    }

    /// Get the path to the models directory
    pub fn models_dir(&self) -> &PathBuf {
        &self.models_dir
    }

    /// Handle model switching - unload current and prepare for new
    pub fn switch_model(&self, new_model_id: &str) -> Result<()> {
        // Unload current model if any
        self.unload_model()?;

        // Check if new model is downloaded
        let model_info = self.get_model_status(new_model_id);
        match model_info {
            Some(info) if info.status == MlxModelStatus::Downloaded => {
                // Model is ready to be loaded on next use
                info!("Switched to model {}, will load on first use", new_model_id);
                Ok(())
            }
            Some(info) if info.status == MlxModelStatus::NotDownloaded => {
                Err(anyhow!("Model {} is not downloaded", new_model_id))
            }
            _ => Err(anyhow!("Model {} not found or in invalid state", new_model_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_model_registry() {
        let registry = MlxModelManager::create_model_registry();
        assert!(!registry.is_empty());
        assert!(registry.contains_key("qwen3_base_1.7b"));

        // Check default model
        let default = registry
            .values()
            .find(|m| m.is_default)
            .expect("Should have a default model");
        assert_eq!(default.id, "qwen3_base_1.7b");
    }
}
