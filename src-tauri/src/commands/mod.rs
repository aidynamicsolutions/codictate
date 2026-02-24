pub mod audio;
pub mod correction;
pub mod history;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod mlx;
pub mod models;
pub mod transcription;
pub mod window;
pub mod menu;


use crate::settings::{get_settings, write_settings, AppSettings, LogLevel};
use crate::utils::cancel_current_operation;
use std::collections::HashMap;
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
    // Get current settings to preserve dictionary
    let current_settings = get_settings(&app);
    let dictionary = current_settings.dictionary;

    // Get default settings
    let mut default_settings = crate::settings::get_default_settings();
    
    // Restore dictionary
    default_settings.dictionary = dictionary;
    
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

#[specta::specta]
#[tauri::command]
pub fn track_ui_analytics_event(
    app: AppHandle,
    event: String,
    props: Option<HashMap<String, String>>,
) -> Result<(), String> {
    crate::analytics::track_ui_event(&app, &event, props)
}

#[specta::specta]
#[tauri::command]
pub fn get_upgrade_prompt_eligibility(
    app: AppHandle,
) -> Result<crate::growth::UpgradePromptEligibility, String> {
    Ok(crate::growth::get_upgrade_prompt_eligibility(&app))
}

#[specta::specta]
#[tauri::command]
pub fn consume_upgrade_prompt_open_request(app: AppHandle) -> Result<bool, String> {
    Ok(crate::growth::consume_pending_upgrade_prompt_open_request(&app))
}

#[specta::specta]
#[tauri::command]
pub fn record_upgrade_prompt_shown(
    app: AppHandle,
    trigger: String,
    variant: String,
) -> Result<(), String> {
    crate::growth::mark_upgrade_prompt_shown(&app, &trigger, &variant)
}

#[specta::specta]
#[tauri::command]
pub fn record_upgrade_prompt_action(
    app: AppHandle,
    action: String,
    trigger: String,
) -> Result<(), String> {
    crate::growth::mark_upgrade_prompt_action(&app, &action, &trigger)
}

#[specta::specta]
#[tauri::command]
pub fn record_upgrade_checkout_result(
    app: AppHandle,
    result: String,
    source: String,
) -> Result<(), String> {
    crate::growth::mark_upgrade_checkout_result(&app, &result, &source)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutInitGate {
    AlreadyInitialized,
    DeferredMissingAccessibility,
    Proceed,
}

fn evaluate_shortcut_init_gate(
    already_initialized: bool,
    accessibility_required: bool,
    accessibility_granted: bool,
) -> ShortcutInitGate {
    if already_initialized {
        return ShortcutInitGate::AlreadyInitialized;
    }

    if accessibility_required && !accessibility_granted {
        return ShortcutInitGate::DeferredMissingAccessibility;
    }

    ShortcutInitGate::Proceed
}

fn should_mark_shortcuts_initialized(gate: ShortcutInitGate, failed_count: usize) -> bool {
    matches!(gate, ShortcutInitGate::Proceed) && failed_count == 0
}

pub(crate) fn initialize_shortcuts_with_source(
    app: &AppHandle,
    source: &str,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let accessibility_required = true;
    #[cfg(not(target_os = "macos"))]
    let accessibility_required = false;

    #[cfg(target_os = "macos")]
    let accessibility_granted = crate::permissions::check_accessibility_permission();
    #[cfg(not(target_os = "macos"))]
    let accessibility_granted = true;

    let gate = evaluate_shortcut_init_gate(
        app.try_state::<ShortcutsInitialized>().is_some(),
        accessibility_required,
        accessibility_granted,
    );

    match gate {
        ShortcutInitGate::AlreadyInitialized => {
            tracing::debug!(
                event_code = "shortcut_init_attempt",
                source = %source,
                accessibility_granted = accessibility_granted,
                outcome = "already_initialized",
                "Shortcuts already initialized"
            );
            return Ok(());
        }
        ShortcutInitGate::DeferredMissingAccessibility => {
            tracing::info!(
                event_code = "shortcut_init_attempt",
                source = %source,
                accessibility_granted = accessibility_granted,
                "Attempting shortcut initialization"
            );
            tracing::info!(
                event_code = "shortcut_init_deferred",
                source = %source,
                accessibility_granted = accessibility_granted,
                attempted_count = 0,
                success_count = 0,
                failed_count = 0,
                failed_ids = "-",
                reason = "accessibility_permission_missing",
                "Shortcut initialization deferred"
            );
            return Err(
                "Shortcut initialization deferred: accessibility permission not granted"
                    .to_string(),
            );
        }
        ShortcutInitGate::Proceed => {}
    }

    tracing::info!(
        event_code = "shortcut_init_attempt",
        source = %source,
        accessibility_granted = accessibility_granted,
        "Attempting shortcut initialization"
    );

    let report = crate::shortcut::init_shortcuts(app);
    let failed_ids_csv = report.failed_ids_csv();

    if !should_mark_shortcuts_initialized(gate, report.failed_count()) {
        tracing::warn!(
            event_code = "shortcut_init_failure",
            source = %source,
            accessibility_granted = accessibility_granted,
            attempted_count = report.attempted_count(),
            success_count = report.success_count(),
            failed_count = report.failed_count(),
            failed_ids = %failed_ids_csv,
            "Shortcut initialization completed with failures"
        );
        return Err(format!(
            "Shortcut initialization failed for bindings: {}",
            failed_ids_csv
        ));
    }

    app.manage(ShortcutsInitialized);

    tracing::info!(
        event_code = "shortcut_init_success",
        source = %source,
        accessibility_granted = accessibility_granted,
        attempted_count = report.attempted_count(),
        success_count = report.success_count(),
        failed_count = report.failed_count(),
        failed_ids = %failed_ids_csv,
        "Shortcuts initialized successfully"
    );

    Ok(())
}

/// Initialize keyboard shortcuts.
/// On macOS, this should be called after accessibility permissions are granted.
/// This is idempotent - calling it multiple times is safe.
#[specta::specta]
#[tauri::command]
pub fn initialize_shortcuts(app: AppHandle) -> Result<(), String> {
    initialize_shortcuts_with_source(&app, "frontend_command")
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_shortcut_init_gate, should_mark_shortcuts_initialized, ShortcutInitGate,
    };

    #[test]
    fn gate_is_already_initialized_when_marker_is_present() {
        let gate = evaluate_shortcut_init_gate(true, true, false);
        assert_eq!(gate, ShortcutInitGate::AlreadyInitialized);
    }

    #[test]
    fn gate_defers_when_accessibility_is_required_and_missing() {
        let gate = evaluate_shortcut_init_gate(false, true, false);
        assert_eq!(gate, ShortcutInitGate::DeferredMissingAccessibility);
    }

    #[test]
    fn gate_proceeds_when_accessibility_not_required() {
        let gate = evaluate_shortcut_init_gate(false, false, false);
        assert_eq!(gate, ShortcutInitGate::Proceed);
    }

    #[test]
    fn mark_shortcuts_initialized_only_when_gate_proceeds_and_no_failures() {
        assert!(should_mark_shortcuts_initialized(
            ShortcutInitGate::Proceed,
            0
        ));
        assert!(!should_mark_shortcuts_initialized(
            ShortcutInitGate::Proceed,
            1
        ));
        assert!(!should_mark_shortcuts_initialized(
            ShortcutInitGate::DeferredMissingAccessibility,
            0
        ));
        assert!(!should_mark_shortcuts_initialized(
            ShortcutInitGate::AlreadyInitialized,
            0
        ));
    }
}
