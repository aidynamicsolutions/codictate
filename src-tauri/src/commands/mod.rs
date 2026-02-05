pub mod audio;
pub mod history;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod mlx;
pub mod models;
pub mod transcription;
pub mod window;


use crate::settings::{get_settings, write_settings, AppSettings, LogLevel};
use crate::utils::cancel_current_operation;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
#[specta::specta]
pub fn cancel_operation(app: AppHandle) {
    cancel_current_operation(&app);
}

#[tauri::command]
#[specta::specta]
pub fn get_app_dir_path(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    Ok(app_data_dir.to_string_lossy().to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    Ok(get_settings(&app))
}

#[tauri::command]
#[specta::specta]
pub fn get_default_settings() -> Result<AppSettings, String> {
    Ok(crate::settings::get_default_settings())
}

#[tauri::command]
#[specta::specta]
pub fn reset_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    // Get current settings to preserve custom words
    let current_settings = get_settings(&app);
    let custom_words = current_settings.custom_words;

    // Get default settings
    let mut default_settings = crate::settings::get_default_settings();
    
    // Restore custom words
    default_settings.custom_words = custom_words;
    
    write_settings(&app, default_settings.clone());
    Ok(default_settings)
}

#[tauri::command]
#[specta::specta]
pub fn get_log_dir_path(app: AppHandle) -> Result<String, String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get log directory: {}", e))?;

    Ok(log_dir.to_string_lossy().to_string())
}

#[specta::specta]
#[tauri::command]
pub fn set_log_level(app: AppHandle, level: LogLevel) -> Result<(), String> {
    // Convert LogLevel to tracing::Level and update the file log level
    let tracing_level = match level {
        LogLevel::Error => tracing::Level::ERROR,
        LogLevel::Warn => tracing::Level::WARN,
        LogLevel::Info => tracing::Level::INFO,
        LogLevel::Debug => tracing::Level::DEBUG,
        LogLevel::Trace => tracing::Level::TRACE,
    };
    crate::tracing_config::set_file_log_level(tracing_level);

    let mut settings = get_settings(&app);
    settings.log_level = level;
    write_settings(&app, settings);

    Ok(())
}

#[specta::specta]
#[tauri::command]
pub fn open_recordings_folder(app: AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let recordings_dir = app_data_dir.join("recordings");

    let path = recordings_dir.to_string_lossy().as_ref().to_string();
    app.opener()
        .open_path(path, None::<String>)
        .map_err(|e| format!("Failed to open recordings folder: {}", e))?;

    Ok(())
}

#[specta::specta]
#[tauri::command]
pub fn open_log_dir(app: AppHandle) -> Result<(), String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get log directory: {}", e))?;

    let path = log_dir.to_string_lossy().as_ref().to_string();
    app.opener()
        .open_path(path, None::<String>)
        .map_err(|e| format!("Failed to open log directory: {}", e))?;

    Ok(())
}

#[specta::specta]
#[tauri::command]
pub fn open_app_data_dir(app: AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let path = app_data_dir.to_string_lossy().as_ref().to_string();
    app.opener()
        .open_path(path, None::<String>)
        .map_err(|e| format!("Failed to open app data directory: {}", e))?;

    Ok(())
}

/// Check if Apple Intelligence is available on this device.
/// Called by the frontend when the user selects Apple Intelligence provider.
#[specta::specta]
#[tauri::command]
pub fn check_apple_intelligence_available() -> bool {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        crate::apple_intelligence::check_apple_intelligence_availability()
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        false
    }
}

/// Log a message from the frontend to the unified log file.
/// This enables session-correlated logging across Rust, Python, and Frontend.
#[specta::specta]
#[tauri::command]
pub fn log_from_frontend(
    level: String,
    session_id: Option<String>,
    target: String,
    message: String,
) {
    crate::tracing_config::log_from_frontend(
        &level,
        session_id.as_deref(),
        &target,
        &message,
    );
}

/// Set the onboarding paste override.
/// When true, forces Direct paste method to work around WebView not receiving
/// CGEvent-simulated Cmd+V keystrokes from the same process.
#[specta::specta]
#[tauri::command]
pub fn set_onboarding_paste_override(app: AppHandle, enabled: bool) {
    use crate::OnboardingPasteOverride;
    if let Some(state) = app.try_state::<OnboardingPasteOverride>() {
        if let Ok(mut override_enabled) = state.lock() {
            *override_enabled = enabled;
            tracing::info!("Onboarding paste override set to: {}", enabled);
        }
    }
}

/// Try to initialize Enigo (keyboard/mouse simulation).
/// Uses the lazy-init EnigoState - calls try_init() to initialize if not already done.
/// On macOS, this will fail if accessibility permissions are not granted.
#[specta::specta]
#[tauri::command]
pub fn initialize_enigo(app: AppHandle) -> Result<(), String> {
    use crate::input::EnigoState;

    // Get the managed EnigoState (which was already registered in initialize_core_logic)
    let enigo_state = app.state::<EnigoState>();
    
    // Check if already initialized
    if enigo_state.is_available() {
        tracing::debug!("Enigo already initialized");
        return Ok(());
    }

    // Try to initialize
    if enigo_state.try_init() {
        tracing::info!("Enigo initialized successfully after permission grant");
        Ok(())
    } else {
        if cfg!(target_os = "macos") {
            tracing::warn!("Failed to initialize Enigo (accessibility permissions may not be granted)");
        } else {
            tracing::warn!("Failed to initialize Enigo");
        }
        Err("Failed to initialize input system".to_string())
    }
}

/// Marker state to track if shortcuts have been initialized.
pub struct ShortcutsInitialized;

/// Initialize keyboard shortcuts.
/// On macOS, this should be called after accessibility permissions are granted.
/// This is idempotent - calling it multiple times is safe.
#[specta::specta]
#[tauri::command]
pub fn initialize_shortcuts(app: AppHandle) -> Result<(), String> {
    // Check if already initialized
    if app.try_state::<ShortcutsInitialized>().is_some() {
        tracing::debug!("Shortcuts already initialized");
        return Ok(());
    }

    // Initialize shortcuts
    crate::shortcut::init_shortcuts(&app);

    // Mark as initialized
    app.manage(ShortcutsInitialized);

    tracing::info!("Shortcuts initialized successfully");
    Ok(())
}
