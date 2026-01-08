//! MLX Local AI Model Manager for Apple Silicon Macs
//!
//! This module provides local LLM inference using Apple's MLX framework
//! via a Python sidecar process running mlx-lm for transcription post-processing
//! on Apple Silicon Macs.

use anyhow::{anyhow, Result};
use hf_hub::api::tokio::{Api, Progress};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::settings::get_settings;

/// Port range to search for available port
const PORT_RANGE_START: u16 = 5000;
const PORT_RANGE_END: u16 = 5100;

/// Minimum required disk space buffer (100 MB) before downloading
const DISK_SPACE_BUFFER_BYTES: u64 = 100 * 1024 * 1024;

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
    DownloadStarted { model_id: String, total_bytes: u64 },
    #[serde(rename = "download_progress")]
    DownloadProgress {
        model_id: String,
        progress: f64,
        downloaded_bytes: u64,
        total_bytes: u64,
        /// Download speed in bytes per second
        speed_bytes_per_sec: u64,
        /// Current file being downloaded
        current_file: String,
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

/// Shared state for download progress tracking across async tasks
#[derive(Clone)]
struct DownloadProgressTracker {
    app_handle: AppHandle,
    model_id: String,
    current_file: Arc<Mutex<String>>,
    /// Total bytes for the current file
    current_file_total: Arc<AtomicU64>,
    /// Bytes downloaded for the current file
    current_file_downloaded: Arc<AtomicU64>,
    /// Cumulative bytes from previous files
    previous_files_bytes: Arc<AtomicU64>,
    /// Grand total of all files
    grand_total_bytes: Arc<AtomicU64>,
    /// Track download start time for speed calculation
    start_time: Arc<Mutex<Option<Instant>>>,
    /// Last emit time for throttling
    last_emit: Arc<Mutex<Instant>>,
    /// Is cancelled flag
    cancelled: Arc<AtomicBool>,
    /// Cancel signal receiver
    cancel_rx: watch::Receiver<bool>,
}

impl DownloadProgressTracker {
    fn new(app_handle: AppHandle, model_id: String, cancel_rx: watch::Receiver<bool>) -> Self {
        Self {
            app_handle,
            model_id,
            current_file: Arc::new(Mutex::new(String::new())),
            current_file_total: Arc::new(AtomicU64::new(0)),
            current_file_downloaded: Arc::new(AtomicU64::new(0)),
            previous_files_bytes: Arc::new(AtomicU64::new(0)),
            grand_total_bytes: Arc::new(AtomicU64::new(0)),
            start_time: Arc::new(Mutex::new(None)),
            last_emit: Arc::new(Mutex::new(Instant::now())),
            cancelled: Arc::new(AtomicBool::new(false)),
            cancel_rx,
        }
    }

    fn set_grand_total(&self, total: u64) {
        self.grand_total_bytes.store(total, Ordering::SeqCst);
    }

    fn mark_file_complete(&self) {
        let downloaded = self.current_file_downloaded.load(Ordering::SeqCst);
        self.previous_files_bytes.fetch_add(downloaded, Ordering::SeqCst);
        self.current_file_downloaded.store(0, Ordering::SeqCst);
        self.current_file_total.store(0, Ordering::SeqCst);
    }

    fn get_total_downloaded(&self) -> u64 {
        self.previous_files_bytes.load(Ordering::SeqCst)
            + self.current_file_downloaded.load(Ordering::SeqCst)
    }

    fn emit_progress(&self) {
        let total_downloaded = self.get_total_downloaded();
        let grand_total = self.grand_total_bytes.load(Ordering::SeqCst);
        let progress = if grand_total > 0 {
            total_downloaded as f64 / grand_total as f64
        } else {
            0.0
        };

        // Calculate speed
        let speed = {
            let start = self.start_time.lock().unwrap();
            if let Some(start_time) = *start {
                let elapsed = start_time.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    (total_downloaded as f64 / elapsed) as u64
                } else {
                    0
                }
            } else {
                0
            }
        };

        let current_file = self.current_file.lock().unwrap().clone();

        let event = MlxModelStateEvent::DownloadProgress {
            model_id: self.model_id.clone(),
            progress,
            downloaded_bytes: total_downloaded,
            total_bytes: grand_total,
            speed_bytes_per_sec: speed,
            current_file,
        };

        if let Err(e) = self.app_handle.emit("mlx-model-state-changed", &event) {
            error!("Failed to emit MLX download progress: {}", e);
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Progress for DownloadProgressTracker {
    async fn init(&mut self, size: usize, filename: &str) {
        // Initialize start time on first file
        {
            let mut start = self.start_time.lock().unwrap();
            if start.is_none() {
                *start = Some(Instant::now());
            }
        }

        // Set current file info
        *self.current_file.lock().unwrap() = filename.to_string();
        self.current_file_total.store(size as u64, Ordering::SeqCst);
        self.current_file_downloaded.store(0, Ordering::SeqCst);

        debug!("Starting download of {} ({} bytes)", filename, size);
    }

    async fn update(&mut self, size: usize) {
        // Check for cancellation signal
        if *self.cancel_rx.borrow() {
            self.cancelled.store(true, Ordering::SeqCst);
            // Panic to abort the download - this will be caught by the caller
            panic!("Download cancelled by user");
        }

        // Update downloaded bytes
        self.current_file_downloaded.fetch_add(size as u64, Ordering::SeqCst);

        // Throttle emissions to every 100ms
        let should_emit = {
            let mut last = self.last_emit.lock().unwrap();
            if last.elapsed() >= Duration::from_millis(100) {
                *last = Instant::now();
                true
            } else {
                false
            }
        };

        if should_emit {
            self.emit_progress();
        }
    }

    async fn finish(&mut self) {
        // Mark file as complete
        self.mark_file_complete();
        debug!("Finished downloading {}", self.current_file.lock().unwrap());
        // Emit final progress for this file
        self.emit_progress();
    }
}

/// Internal state for tracking download operations
struct DownloadState {
    model_id: String,
    cancel_sender: watch::Sender<bool>,
    retry_count: u8,
    /// Handle to the spawned download task for abort support
    download_handle: Option<tokio::task::JoinHandle<Result<()>>>,
}

/// Internal state for a loaded model (metadata only, Generator stored separately)
struct LoadedModelState {
    model_id: String,
    last_used: Instant,
}

/// Manager for MLX-based local AI models
pub struct MlxModelManager {
    app_handle: AppHandle,
    models_dir: PathBuf,
    /// Available models registry
    models: RwLock<HashMap<String, MlxModelInfo>>,
    /// Currently active download operation
    current_download: Mutex<Option<DownloadState>>,
    /// Currently loaded model metadata
    loaded_model: RwLock<Option<LoadedModelState>>,
    /// Currently active server port (0 if not running)
    active_port: AtomicU16,
    /// Python sidecar server child process (for graceful shutdown)
    server_process: Mutex<Option<Child>>,
    /// HTTP client for sidecar communication
    http_client: reqwest::Client,
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
            active_port: AtomicU16::new(0),
            server_process: Mutex::new(None),
            http_client: reqwest::Client::new(),
            unload_task: Mutex::new(None),
        };

        // Update status based on what's already downloaded
        manager.update_download_status()?;

        Ok(manager)
    }

    /// Get system memory in gigabytes using macOS sysctl
    pub fn get_system_memory_gb() -> u64 {
        // Use sysctl to get total physical memory on macOS
        let output = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let mem_str = String::from_utf8_lossy(&output.stdout);
                    if let Ok(mem_bytes) = mem_str.trim().parse::<u64>() {
                        let mem_gb = mem_bytes / (1024 * 1024 * 1024);
                        debug!("Detected system memory: {} GB", mem_gb);
                        return mem_gb;
                    }
                }
                warn!("Failed to parse sysctl output, defaulting to 16GB");
                16 // Default to 16GB if parsing fails
            }
            Err(e) => {
                warn!("Failed to run sysctl: {}, defaulting to 16GB", e);
                16 // Default to 16GB if command fails
            }
        }
    }

    /// Create the initial model registry with all available models
    /// The recommended model is chosen based on system RAM:
    /// - 8GB or less: Qwen 3 Base 1.7B (~2-3 GB runtime, leaves headroom for system)
    /// - 9-16GB: Qwen 3 Base 4B (~4-5 GB runtime, good balance)
    /// - More than 16GB: Qwen 3 Base 8B (~7-8 GB runtime, best quality)
    fn create_model_registry() -> HashMap<String, MlxModelInfo> {
        let mut models = HashMap::new();
        
        // Determine the recommended model based on system RAM
        // Memory usage estimates (4-bit quantized):
        // - 0.6B: ~1-1.5 GB runtime
        // - 1.7B: ~2-3 GB runtime
        // - 4B: ~4-5 GB runtime  
        // - 8B: ~7-8 GB runtime
        let system_ram_gb = Self::get_system_memory_gb();
        let recommended_model_id = if system_ram_gb > 16 {
            "qwen3_base_8b" // Best quality for high-memory systems
        } else if system_ram_gb > 8 {
            "qwen3_base_4b" // Good balance for 16GB systems
        } else {
            "qwen3_base_1.7b" // Lightweight for 8GB systems
        };
        info!(
            "System has {}GB RAM, recommending model: {}",
            system_ram_gb, recommended_model_id
        );

        // Qwen 3 family - excellent instruction following and reasoning
        models.insert(
            "qwen3_base_0.6b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_0.6b".to_string(),
                display_name: "Qwen 3 Base 0.6B".to_string(),
                description: "Ultra-fast responses. Best for simple corrections.".to_string(),
                hf_repo: "mlx-community/Qwen3-0.6B-4bit".to_string(),
                size_bytes: 400 * 1024 * 1024, // ~400 MB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: "qwen3_base_0.6b" == recommended_model_id,
                parameters: "~1 GB".to_string(),
            },
        );

        models.insert(
            "qwen3_base_1.7b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_1.7b".to_string(),
                display_name: "Qwen 3 Base 1.7B".to_string(),
                description: "Good speed and quality. Great for 8GB Macs.".to_string(),
                hf_repo: "mlx-community/Qwen3-1.7B-4bit".to_string(),
                size_bytes: 1024 * 1024 * 1024, // ~1 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: "qwen3_base_1.7b" == recommended_model_id,
                parameters: "~2-3 GB".to_string(),
            },
        );

        models.insert(
            "qwen3_base_4b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_4b".to_string(),
                display_name: "Qwen 3 Base 4B".to_string(),
                description: "Strong reasoning and writing quality.".to_string(),
                hf_repo: "mlx-community/Qwen3-4B-4bit".to_string(),
                size_bytes: 2300 * 1024 * 1024, // ~2.3 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: "qwen3_base_4b" == recommended_model_id,
                parameters: "~4-5 GB".to_string(),
            },
        );

        models.insert(
            "qwen3_base_8b".to_string(),
            MlxModelInfo {
                id: "qwen3_base_8b".to_string(),
                display_name: "Qwen 3 Base 8B".to_string(),
                description: "Best quality and complex reasoning.".to_string(),
                hf_repo: "mlx-community/Qwen3-8B-4bit".to_string(),
                size_bytes: 4700 * 1024 * 1024, // ~4.7 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: "qwen3_base_8b" == recommended_model_id,
                parameters: "~7-8 GB".to_string(),
            },
        );

        // Gemma 3 family - Google's open model, strong multi-language
        models.insert(
            "gemma3_base_1b".to_string(),
            MlxModelInfo {
                id: "gemma3_base_1b".to_string(),
                display_name: "Gemma 3 Base 1B".to_string(),
                description: "Lightweight with good multi-language support.".to_string(),
                hf_repo: "mlx-community/gemma-3-1b-it-4bit".to_string(),
                size_bytes: 800 * 1024 * 1024, // ~800 MB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "~1 GB".to_string(),
            },
        );

        models.insert(
            "gemma3_base_4b".to_string(),
            MlxModelInfo {
                id: "gemma3_base_4b".to_string(),
                display_name: "Gemma 3 Base 4B".to_string(),
                description: "Excellent multi-language and translation.".to_string(),
                hf_repo: "mlx-community/gemma-3-4b-it-4bit".to_string(),
                size_bytes: 2300 * 1024 * 1024, // ~2.3 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "~3 GB".to_string(),
            },
        );

        // SmolLM 3 - HuggingFace's efficient small model
        models.insert(
            "smollm3_base_3b".to_string(),
            MlxModelInfo {
                id: "smollm3_base_3b".to_string(),
                display_name: "SmolLM 3 Base 3B".to_string(),
                description: "HuggingFace's efficient small model.".to_string(),
                hf_repo: "mlx-community/SmolLM2-1.7B-Instruct-4bit".to_string(),
                size_bytes: 1800 * 1024 * 1024, // ~1.8 GB
                status: MlxModelStatus::NotDownloaded,
                download_progress: 0.0,
                is_default: false,
                parameters: "~2 GB".to_string(),
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
                // Check if all required files are present (complete download)
                if Self::is_model_download_complete_internal(&model_path) {
                    // Model directory exists with all required files - mark as downloaded
                    if model.status == MlxModelStatus::NotDownloaded
                        || model.status == MlxModelStatus::DownloadFailed
                    {
                        model.status = MlxModelStatus::Downloaded;
                        model.download_progress = 1.0;
                    }
                } else {
                    // Incomplete download - clean up and mark as not downloaded
                    warn!(
                        "Found incomplete model directory for {}, cleaning up",
                        model.id
                    );
                    let _ = fs::remove_dir_all(&model_path);
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

    /// Check if a model download is complete by verifying all required files exist.
    /// This is used to detect interrupted downloads on app startup.
    fn is_model_download_complete_internal(model_path: &PathBuf) -> bool {
        // Required files for a complete MLX model download
        const REQUIRED_FILES: &[&str] = &[
            "config.json",
            "tokenizer.json",
            "model.safetensors",
        ];

        REQUIRED_FILES.iter().all(|filename| {
            let file_path = model_path.join(filename);
            file_path.exists() && file_path.is_file()
        })
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

    /// Get the base URL for the sidecar server using the active port
    fn get_base_url(&self) -> String {
        let port = self.active_port.load(Ordering::SeqCst);
        format!("http://127.0.0.1:{}", port)
    }

    /// Find an available port in the configured range
    fn find_available_port() -> Result<u16> {
        for port in PORT_RANGE_START..=PORT_RANGE_END {
            if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)) {
                // Successfully bound, port is available
                drop(listener); // Release the port immediately
                info!("Found available port: {}", port);
                return Ok(port);
            }
        }
        Err(anyhow!(
            "No available port found in range {}-{}",
            PORT_RANGE_START,
            PORT_RANGE_END
        ))
    }

    /// Check if there's enough disk space for a download
    fn check_disk_space(&self, required_bytes: u64) -> Result<()> {
        // Use std::process::Command to run `df` on macOS
        let output = Command::new("df")
            .args(["-k", self.models_dir.to_string_lossy().as_ref()])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse df output: second line, fourth column (available space in KB)
                if let Some(line) = stdout.lines().nth(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        if let Ok(available_kb) = parts[3].parse::<u64>() {
                            let available_bytes = available_kb * 1024;
                            let required_with_buffer = required_bytes + DISK_SPACE_BUFFER_BYTES;
                            
                            if available_bytes < required_with_buffer {
                                return Err(anyhow!(
                                    "Insufficient disk space. Required: {} MB, Available: {} MB",
                                    required_with_buffer / (1024 * 1024),
                                    available_bytes / (1024 * 1024)
                                ));
                            }
                            debug!(
                                "Disk space check passed: {} MB available, {} MB required",
                                available_bytes / (1024 * 1024),
                                required_with_buffer / (1024 * 1024)
                            );
                            return Ok(());
                        }
                    }
                }
                warn!("Could not parse df output, skipping disk space check");
                Ok(()) // Continue anyway if we can't parse
            }
            _ => {
                warn!("Could not run df command, skipping disk space check");
                Ok(()) // Continue anyway if command fails
            }
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

        // Check disk space before downloading
        self.check_disk_space(model_info.size_bytes)?;

        // Set up cancellation channel
        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Store download state
        {
            let mut current = self.current_download.lock().unwrap();
            *current = Some(DownloadState {
                model_id: model_id.to_string(),
                cancel_sender: cancel_tx,
                retry_count: 0,
                download_handle: None, // Will be set if spawned
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

        // Note: DownloadStarted event is emitted in perform_download with total_bytes


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

        // Use hf-hub API to download model files (Api created inside each spawn for each file)
        let _api = Api::new()?;

        // Get list of files to download
        // For MLX models, we typically need: config.json, tokenizer files, and model weights
        let required_files = vec![
            ("config.json", true),
            ("tokenizer.json", true),
            ("tokenizer_config.json", false),
            ("model.safetensors", true),
            ("model.safetensors.index.json", false),
        ];

        // Get the registered model size as an estimate for total bytes
        let total_bytes = {
            let models = self.models.read().unwrap();
            models.get(model_id).map(|m| m.size_bytes).unwrap_or(0)
        };

        // Create progress tracker
        let progress_tracker = DownloadProgressTracker::new(
            self.app_handle.clone(),
            model_id.to_string(),
            cancel_rx.clone(),
        );
        progress_tracker.set_grand_total(total_bytes);

        // Emit download started with total size
        self.emit_event(MlxModelStateEvent::DownloadStarted {
            model_id: model_id.to_string(),
            total_bytes,
        });

        for (filename, is_required) in required_files.iter() {
            // Check for cancellation
            if *cancel_rx.borrow() || progress_tracker.is_cancelled() {
                // Clean up partial download
                let _ = fs::remove_dir_all(&model_dir);
                return Err(anyhow!("Download cancelled"));
            }

            debug!("Downloading {} for model {}", filename, model_id);

            // Download with progress tracking - spawn task to enable cancellation via panic
            let tracker_clone = progress_tracker.clone();
            let filename_str = filename.to_string();
            let hf_repo_str = hf_repo.to_string();
            
            let download_handle = tokio::spawn(async move {
                // Create new API client inside spawn (ApiRepo doesn't implement Clone)
                let api = Api::new()?;
                let repo = api.model(hf_repo_str);
                repo.download_with_progress(&filename_str, tracker_clone).await
            });
            
            // Wait for download, handling cancellation panics
            let download_result = match download_handle.await {
                Ok(result) => result,
                Err(e) if e.is_panic() => {
                    // Panic from Progress::update (cancellation)
                    let _ = fs::remove_dir_all(&model_dir);
                    return Err(anyhow!("Download cancelled"));
                }
                Err(e) if e.is_cancelled() => {
                    let _ = fs::remove_dir_all(&model_dir);
                    return Err(anyhow!("Download cancelled"));
                }
                Err(e) => {
                    return Err(anyhow!("Download task error: {}", e));
                }
            };

            match download_result {
                Ok(cached_path) => {
                    // Copy from cache to our model directory
                    let dest_path = model_dir.join(filename);
                    if let Err(e) = fs::copy(&cached_path, &dest_path) {
                        if !*is_required {
                            debug!("Optional file {} copy failed: {}", filename, e);
                            continue;
                        }
                        warn!("Failed to copy {}: {}", filename, e);
                    }
                }
                Err(e) => {
                    if *is_required {
                        error!("Failed to download required file {}: {}", filename, e);
                        let _ = fs::remove_dir_all(&model_dir);
                        return Err(anyhow!("Failed to download {}: {}", filename, e));
                    }
                    // Optional files can fail
                    debug!("Optional file {} not available: {}", filename, e);
                }
            }

            // Update model progress in state
            {
                let total_downloaded = progress_tracker.get_total_downloaded();
                let progress = if total_bytes > 0 {
                    total_downloaded as f64 / total_bytes as f64
                } else {
                    0.0
                };
                
                let mut models = self.models.write().unwrap();
                if let Some(model) = models.get_mut(model_id) {
                    model.download_progress = progress;
                }
            }
        }

        info!("Successfully downloaded model {} to {:?}", model_id, model_dir);
        Ok(())
    }

    /// Cancel an in-progress download
    pub fn cancel_download(&self) -> Result<()> {
        let mut current = self.current_download.lock().unwrap();
        if let Some(state) = current.as_mut() {
            // First send the cancel signal (for cooperative cancellation)
            let _ = state.cancel_sender.send(true);
            
            // Then abort the task (for immediate interruption)
            if let Some(handle) = state.download_handle.take() {
                handle.abort();
                info!("Aborted download task for {}", state.model_id);
            }
            
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

    /// Process text using the loaded model via Python sidecar
    pub async fn process_text(&self, prompt: &str) -> Result<String> {
        // Check if a model is loaded, determine which model to load if needed
        let model_to_load: Option<String> = {
            let loaded = self.loaded_model.read().unwrap();
            if loaded.is_none() {
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
                        Some(selected)
                    }
                    Some(info) if info.status == MlxModelStatus::Ready => {
                        None // Already ready
                    }
                    _ => {
                        return Err(anyhow!(
                            "Selected MLX model is not downloaded: {}",
                            selected
                        ));
                    }
                }
            } else {
                None // Model already loaded
            }
        }; // Lock is released here

        // Load model if needed (outside of lock scope)
        if let Some(model_id) = model_to_load {
            self.load_model_async(&model_id).await?;
        }

        // Verify sidecar actually has model loaded (may have been unloaded by idle timer)
        if !self.is_sidecar_model_loaded().await? {
            info!("Sidecar model was unloaded, reloading...");
            self.reload_model().await?;
        }

        // Update last_used timestamp
        {
            let mut loaded = self.loaded_model.write().unwrap();
            if let Some(ref mut state) = *loaded {
                state.last_used = Instant::now();
            }
        }

        // Generate text using the Python sidecar
        match self.generate_with_sidecar(prompt).await {
            Ok(response) => {
                // Reset unload timer on success
                self.reset_unload_timer();
                Ok(response)
            }
            Err(e) => {
                let error_msg = e.to_string();
                // Handle "No model loaded" by reloading and retrying once
                if error_msg.contains("No model loaded") {
                    warn!("Sidecar reports no model loaded, reloading and retrying...");
                    if let Err(reload_err) = self.reload_model().await {
                        error!("Failed to reload model: {}", reload_err);
                        return Err(anyhow!("Post-processing unavailable: model reload failed"));
                    }
                    // Retry once - if this also fails, error bubbles up
                    let result = self.generate_with_sidecar(prompt).await?;
                    self.reset_unload_timer();
                    return Ok(result);
                }
                Err(e)
            }
        }
    }

    /// Internal method to generate text via sidecar HTTP call
    async fn generate_with_sidecar(&self, prompt: &str) -> Result<String> {
        let model_id = {
            let loaded = self.loaded_model.read().unwrap();
            loaded.as_ref().map(|s| s.model_id.clone())
        };

        debug!("Generating text with model {:?}", model_id);

        #[derive(Serialize)]
        struct GenerateRequest {
            prompt: String,
            max_tokens: i32,
            temperature: f32,
            system_ram_gb: u64,
        }

        #[derive(Deserialize)]
        struct GenerateResponse {
            response: String,
        }

        let response = self
            .http_client
            .post(format!("{}/generate", self.get_base_url()))
            .json(&GenerateRequest {
                prompt: prompt.to_string(),
                max_tokens: -1,  // Sentinel: Python calculates dynamically based on input + RAM
                temperature: 0.7,
                system_ram_gb: Self::get_system_memory_gb(),
            })
            .timeout(Duration::from_secs(60))  // 60s timeout for local LLM inference
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call sidecar: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Sidecar generation failed: {}", error_text));
        }

        let result: GenerateResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse sidecar response: {}", e))?;

        Ok(result.response)
    }

    /// Load a model into memory via Python sidecar
    async fn load_model_async(&self, model_id: &str) -> Result<()> {
        info!("Loading MLX model via sidecar: {}", model_id);

        // Start server if not running
        self.ensure_server_running_async().await?;

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

        // Load model via HTTP POST to sidecar
        #[derive(Serialize)]
        struct LoadRequest {
            model_path: String,
        }

        let response = self
            .http_client
            .post(format!("{}/load", self.get_base_url()))
            .json(&LoadRequest {
                model_path: model_path.to_string_lossy().to_string(),
            })
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                // Store loaded model state (metadata)
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

                info!("MLX model {} loaded successfully via sidecar", model_id);
                Ok(())
            }
            Ok(resp) => {
                let error_text = resp.text().await.unwrap_or_default();
                error!("Failed to load model {}: {}", model_id, error_text);
                let mut models = self.models.write().unwrap();
                if let Some(model) = models.get_mut(model_id) {
                    model.status = MlxModelStatus::LoadFailed;
                }
                self.emit_event(MlxModelStateEvent::LoadingFailed {
                    model_id: model_id.to_string(),
                    error: error_text.clone(),
                });
                Err(anyhow!("Failed to load model: {}", error_text))
            }
            Err(e) => {
                error!("Failed to load model {}: {}", model_id, e);
                let mut models = self.models.write().unwrap();
                if let Some(model) = models.get_mut(model_id) {
                    model.status = MlxModelStatus::LoadFailed;
                }
                self.emit_event(MlxModelStateEvent::LoadingFailed {
                    model_id: model_id.to_string(),
                    error: e.to_string(),
                });
                Err(anyhow!("Failed to load model: {}", e))
            }
        }
    }

    /// Ensure the Python sidecar server is running
    /// Ensure the Python sidecar server is running (async version)
    async fn ensure_server_running_async(&self) -> Result<()> {
        // Check if server is already running by checking active port
        let current_port = self.active_port.load(Ordering::SeqCst);
        if current_port != 0 {
            // Verify the process is still running
            let mut process_guard = self.server_process.lock().unwrap();
            if let Some(ref mut child) = *process_guard {
                match child.try_wait() {
                    Ok(None) => {
                        // Process still running
                        return Ok(());
                    }
                    Ok(Some(status)) => {
                        warn!("MLX sidecar process exited with status: {:?}", status);
                    }
                    Err(e) => {
                        warn!("Failed to check sidecar process status: {}", e);
                    }
                }
            }
            // Process died, reset state
            self.active_port.store(0, Ordering::SeqCst);
        }

        // Find an available port
        let port = Self::find_available_port()?;

        // Start the server
        info!("Starting MLX Python sidecar server on port {}...", port);
        
        // Get the python-backend path - try bundled resource first, then dev path
        let resource_dir = self.app_handle.path().resource_dir()
            .map_err(|e| anyhow!("Failed to get resource dir: {}", e))?;
        
        let (server_script, is_dev_mode) = {
            let bundled = resource_dir.join("python-backend").join("server.py");
            if bundled.exists() {
                debug!("Using bundled server script: {:?}", bundled);
                (bundled, false)
            } else {
                // Dev mode: use absolute path from CARGO_MANIFEST_DIR
                let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                    .unwrap_or_else(|_| ".".to_string());
                let project_root = PathBuf::from(&manifest_dir).parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                let dev_path = project_root.join("python-backend").join("server.py");
                debug!("Using dev server script: {:?}", dev_path);
                (dev_path, true)
            }
        };

        // Get the uv binary path - try bundled sidecar first, then system uv
        let uv_binary = {
            // Tauri sidecars are at: resources/_up_/binaries/<name>-<target>
            let bundled = resource_dir.join("binaries").join("uv-aarch64-apple-darwin");
            if bundled.exists() {
                debug!("Using bundled uv binary: {:?}", bundled);
                bundled.to_string_lossy().to_string()
            } else {
                // Dev mode: fallback to system uv
                debug!("Using system uv binary");
                "uv".to_string()
            }
        };

        let mut cmd = Command::new(&uv_binary);
        cmd.arg("run")
            .arg(&server_script)
            .arg("--port")
            .arg(port.to_string());
        
        // In dev mode, inherit stderr so we can see errors in terminal
        if is_dev_mode {
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
        
        let child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to start sidecar with '{}': {}. Make sure 'uv' is installed.", uv_binary, e))?;

        let pid = child.id();
        
        // Store port and process
        self.active_port.store(port, Ordering::SeqCst);
        {
            let mut process_guard = self.server_process.lock().unwrap();
            *process_guard = Some(child);
        }
        
        info!("MLX sidecar process started with PID {}, waiting for server to be ready...", pid);
        
        // Wait for server to be ready (up to 60 seconds for first run installing deps) - async!
        self.wait_for_server_ready_async().await?;
        
        info!("MLX sidecar server is ready on port {}", port);
        Ok(())
    }

    /// Wait for the server to be ready by polling /status endpoint (async version)
    async fn wait_for_server_ready_async(&self) -> Result<()> {
        let max_attempts = 60; // 60 seconds total
        let poll_interval = Duration::from_millis(1000);
        let base_url = self.get_base_url();
        
        for attempt in 1..=max_attempts {
            // Use async client with timeout for status check
            match self.http_client
                .get(format!("{}/status", base_url))
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    debug!("Server ready after {} seconds", attempt);
                    return Ok(());
                }
                Ok(resp) => {
                    debug!("Server returned non-success status: {} (attempt {}/{})", resp.status(), attempt, max_attempts);
                }
                Err(e) => {
                    if attempt == 1 {
                        info!("Waiting for server to start (first run may take ~30s to install dependencies)...");
                    } else if attempt % 10 == 0 {
                        debug!("Still waiting for server... (attempt {}/{}, error: {})", attempt, max_attempts, e);
                    }
                }
            }
            
            tokio::time::sleep(poll_interval).await;
        }
        
        Err(anyhow!("Server failed to start within {} seconds", max_attempts))
    }

    /// Stop the Python sidecar server gracefully
    pub fn stop_server(&self) -> Result<()> {
        let mut process_guard = self.server_process.lock().unwrap();
        if let Some(mut child) = process_guard.take() {
            let pid = child.id();
            info!("Stopping MLX sidecar server (PID {})...", pid);
            
            // Try graceful shutdown first via SIGTERM
            #[cfg(unix)]
            {
                let _ = Command::new("kill")
                    .args(["-15", &pid.to_string()]) // SIGTERM
                    .status();
            }
            
            // Give it a moment to shutdown gracefully.
            // Note: Using blocking sleep here is intentional because this method is called from
            // the synchronous RunEvent::Exit handler in lib.rs. During app exit, a brief block
            // is acceptable and avoids the complexity of spawning a tokio runtime.
            std::thread::sleep(Duration::from_millis(500));
            
            // Force kill if still running
            match child.try_wait() {
                Ok(Some(_)) => {
                    info!("MLX sidecar server stopped gracefully");
                }
                Ok(None) => {
                    warn!("Server didn't stop gracefully, forcing kill");
                    let _ = child.kill();
                    let _ = child.wait();
                    info!("MLX sidecar server force stopped");
                }
                Err(e) => {
                    warn!("Error checking server status: {}, forcing kill", e);
                    let _ = child.kill();
                }
            }
            
            // Reset the port
            self.active_port.store(0, Ordering::SeqCst);
        }
        Ok(())
    }



    /// Synchronous unload for compatibility with existing code
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

    /// Check if the sidecar server has a model currently loaded.
    /// This helps detect when the sidecar has auto-unloaded the model due to its own idle timer.
    async fn is_sidecar_model_loaded(&self) -> Result<bool> {
        // If server isn't running, model definitely isn't loaded
        let port = self.active_port.load(Ordering::SeqCst);
        if port == 0 {
            return Ok(false);
        }

        #[derive(Deserialize)]
        struct StatusResponse {
            model_loaded: bool,
        }

        match self.http_client
            .get(format!("{}/status", self.get_base_url()))
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<StatusResponse>().await {
                    Ok(status) => Ok(status.model_loaded),
                    Err(e) => {
                        warn!("Failed to parse sidecar status response: {}", e);
                        Ok(false)
                    }
                }
            }
            Ok(resp) => {
                debug!("Sidecar /status returned non-success: {}", resp.status());
                Ok(false)
            }
            Err(e) => {
                debug!("Failed to check sidecar status: {}", e);
                Ok(false)
            }
        }
    }

    /// Reload the model by clearing local state and loading from settings.
    /// Used when sidecar has auto-unloaded the model.
    async fn reload_model(&self) -> Result<()> {
        // Clear local loaded state
        {
            let mut loaded = self.loaded_model.write().unwrap();
            if let Some(state) = loaded.take() {
                info!("Clearing stale model state for: {}", state.model_id);
                // Update status to downloaded (not ready, since it was unloaded)
                let mut models = self.models.write().unwrap();
                if let Some(model) = models.get_mut(&state.model_id) {
                    model.status = MlxModelStatus::Downloaded;
                }
            }
        }

        // Determine which model to load
        let settings = get_settings(&self.app_handle);
        let selected = settings
            .post_process_models
            .get(LOCAL_MLX_PROVIDER_ID)
            .cloned()
            .unwrap_or_else(|| DEFAULT_MLX_MODEL_ID.to_string());

        // Load the model
        self.load_model_async(&selected).await
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
                // First, call sidecar /unload to free memory in Python
                let port = manager.active_port.load(Ordering::SeqCst);
                if port != 0 {
                    let base_url = format!("http://127.0.0.1:{}", port);
                    if let Ok(resp) = manager.http_client
                        .post(format!("{}/unload", base_url))
                        .timeout(Duration::from_secs(5))
                        .send()
                        .await
                    {
                        if resp.status().is_success() {
                            info!("Sidecar model unloaded, memory freed");
                        } else {
                            warn!("Sidecar /unload returned: {}", resp.status());
                        }
                    }
                }
                
                // Then update Rust-side state
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

        // Check that exactly one model is marked as default (the recommended one)
        let default_models: Vec<_> = registry.values().filter(|m| m.is_default).collect();
        assert_eq!(default_models.len(), 1, "Should have exactly one default model");

        // The default should be 1.7B (≤8GB), 4B (9-16GB), or 8B (>16GB) based on RAM
        let default = default_models[0];
        assert!(
            default.id == "qwen3_base_1.7b" || default.id == "qwen3_base_4b" || default.id == "qwen3_base_8b",
            "Default model should be qwen3_base_1.7b, qwen3_base_4b, or qwen3_base_8b, got: {}",
            default.id
        );
    }
}
