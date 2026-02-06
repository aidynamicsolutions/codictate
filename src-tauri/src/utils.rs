use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::shortcut;
use crate::ManagedToggleState;
use tracing::{debug, info, warn};
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
    info!("Cancellation initiated");

    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    let tm = app.state::<Arc<TranscriptionManager>>();

    // Guard: Only proceed if there is actually something to cancel
    // This prevents "ghost" cancellations when the shortcut remains registered but idle
    let is_recording = audio_manager.is_recording();
    let is_transcribing = tm.is_any_session_active();
    let is_overlay_active = crate::overlay::is_overlay_active();

    if !is_recording && !is_transcribing && !is_overlay_active {
        debug!("Cancellation: Ignored (idle state)");
        // Just in case, try to unregister the shortcut if it's stuck
        shortcut::unregister_cancel_shortcut(app);
        return;
    }

    // CRITICAL: Clear active session IMMEDIATELY to prevent pending transcriptions from pasting
    // Do not defer this to the background thread!
    tm.clear_active_session();

    // Show cancelling state on overlay IMMEDIATELY to prevent race conditions
    // where other threads (e.g. action.stop) might try to hide the overlay.
    // By setting state to Cancelling now, we block hide_overlay_if_recording from working.
    crate::overlay::show_cancelling_overlay(app);

    // Unregister the cancel shortcut asynchronously
    shortcut::unregister_cancel_shortcut(app);

    // First, reset all shortcut toggle states.
    // This is critical for non-push-to-talk mode where shortcuts toggle on/off
    let toggle_state_manager = app.state::<ManagedToggleState>();
    if let Ok(mut states) = toggle_state_manager.lock() {
        states.active_toggles.values_mut().for_each(|v| *v = false);
    } else {
        warn!("Cancellation: Failed to lock toggle state manager");
    }

    // Cancel any ongoing recording
    audio_manager.cancel_recording();
    
    // Spawn a thread for the delay and cleanup to avoid blocking the main thread/event loop
    // Blocking here prevents the 'show-overlay' event from being processed by the frontend!
    let app_clone = app.clone();
    std::thread::spawn(move || {
        // info!("Cancellation: Sleeping for 600ms (in background thread)");
        std::thread::sleep(std::time::Duration::from_millis(600));

        // Update tray icon and hide overlay
        change_tray_icon(&app_clone, crate::tray::TrayIconState::Idle);
        hide_recording_overlay(&app_clone);

        // Unload model if immediate unload is enabled
        let tm = app_clone.state::<Arc<TranscriptionManager>>();

        // Session is already cleared, but unload might be needed
        tm.maybe_unload_immediately("cancellation");

        info!("Cancellation completed");
    });
}

/// Check if using the Wayland display server protocol
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.to_lowercase() == "wayland")
            .unwrap_or(false)
}
