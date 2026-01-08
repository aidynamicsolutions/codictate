use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::shortcut;
use crate::ManagedToggleState;
use log::{debug, info, warn};
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

// Re-export all utility modules for easy access
// pub use crate::audio_feedback::*;
pub use crate::clipboard::*;
pub use crate::overlay::*;
pub use crate::tray::*;

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

/// Get the maximum recording time in seconds based on system RAM.
/// RAM-based limits prevent memory exhaustion during long recordings:
/// - ≤8GB: 6 minutes (360s)
/// - 9-16GB: 8 minutes (480s)
/// - >16GB: 12 minutes (720s)
pub fn get_recording_limit_seconds() -> u32 {
    let ram_gb = get_system_memory_gb();
    if ram_gb <= 8 {
        360 // 6 minutes
    } else if ram_gb <= 16 {
        480 // 8 minutes
    } else {
        720 // 12 minutes
    }
}

/// Centralized cancellation function that can be called from anywhere in the app.
/// Handles cancelling both recording and transcription operations and updates UI state.
pub fn cancel_current_operation(app: &AppHandle) {
    info!("Initiating operation cancellation...");

    // Unregister the cancel shortcut asynchronously
    shortcut::unregister_cancel_shortcut(app);

    // First, reset all shortcut toggle states.
    // This is critical for non-push-to-talk mode where shortcuts toggle on/off
    let toggle_state_manager = app.state::<ManagedToggleState>();
    if let Ok(mut states) = toggle_state_manager.lock() {
        states.active_toggles.values_mut().for_each(|v| *v = false);
    } else {
        warn!("Failed to lock toggle state manager during cancellation");
    }

    // Cancel any ongoing recording
    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    audio_manager.cancel_recording();

    // Update tray icon and hide overlay
    change_tray_icon(app, crate::tray::TrayIconState::Idle);
    hide_recording_overlay(app);

    // Unload model if immediate unload is enabled
    let tm = app.state::<Arc<TranscriptionManager>>();
    tm.maybe_unload_immediately("cancellation");

    info!("Operation cancellation completed - returned to idle state");
}

/// Check if using the Wayland display server protocol
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.to_lowercase() == "wayland")
            .unwrap_or(false)
}
