use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::shortcut;
use crate::ManagedToggleState;
use crate::TranscriptionCoordinator;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tracing::{debug, info, warn};

// Re-export all utility modules for easy access
// pub use crate::audio_feedback::*;
pub use crate::clipboard::*;
pub use crate::overlay::*;
pub use crate::tray::*;

const CANCEL_REOPEN_SUPPRESS_MS: u64 = 2_000;
static CANCEL_REOPEN_SUPPRESSION_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn suppression_remaining_ms_at(now_ms: u64, until_ms: u64) -> u64 {
    until_ms.saturating_sub(now_ms)
}

pub fn mark_cancel_reopen_suppression() {
    let now_ms = now_epoch_ms();
    let until_ms = now_ms.saturating_add(CANCEL_REOPEN_SUPPRESS_MS);
    CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.store(until_ms, Ordering::SeqCst);

    info!(
        event_code = "cancel_foreground_suppression_set",
        duration_ms = CANCEL_REOPEN_SUPPRESS_MS,
        until_ms,
        "Set foreground suppression window after cancellation"
    );
}

pub fn cancel_reopen_suppression_remaining_ms() -> u64 {
    let now_ms = now_epoch_ms();
    let until_ms = CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.load(Ordering::SeqCst);
    suppression_remaining_ms_at(now_ms, until_ms)
}

pub fn is_cancel_reopen_suppressed() -> bool {
    cancel_reopen_suppression_remaining_ms() > 0
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

    mark_cancel_reopen_suppression();

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
    let recording_was_active = audio_manager.is_recording();
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

        // Notify coordinator so it can keep lifecycle state coherent.
        if let Some(coordinator) = app_clone.try_state::<TranscriptionCoordinator>() {
            coordinator.notify_cancel(recording_was_active);
        }

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

/// Check if running on KDE Plasma desktop environment
#[cfg(target_os = "linux")]
pub fn is_kde_plasma() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
        || std::env::var("KDE_SESSION_VERSION").is_ok()
}

/// Check if running on KDE Plasma with Wayland
#[cfg(target_os = "linux")]
pub fn is_kde_wayland() -> bool {
    is_wayland() && is_kde_plasma()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static SUPPRESSION_TEST_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn suppression_remaining_clamps_at_zero() {
        assert_eq!(suppression_remaining_ms_at(100, 90), 0);
        assert_eq!(suppression_remaining_ms_at(100, 100), 0);
        assert_eq!(suppression_remaining_ms_at(100, 150), 50);
    }

    #[test]
    fn suppression_state_expires_at_boundary() {
        let _guard = SUPPRESSION_TEST_GUARD.lock().unwrap();
        let now_ms = now_epoch_ms();

        CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.store(now_ms.saturating_add(10), Ordering::SeqCst);
        assert!(is_cancel_reopen_suppressed());
        assert!(cancel_reopen_suppression_remaining_ms() > 0);

        CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.store(now_ms, Ordering::SeqCst);
        assert!(!is_cancel_reopen_suppressed());
        assert_eq!(cancel_reopen_suppression_remaining_ms(), 0);

        CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.store(0, Ordering::SeqCst);
    }

    #[test]
    fn mark_sets_active_suppression_window() {
        let _guard = SUPPRESSION_TEST_GUARD.lock().unwrap();
        let before_mark_ms = now_epoch_ms();
        mark_cancel_reopen_suppression();
        let after_mark_ms = now_epoch_ms();

        let until_ms = CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.load(Ordering::SeqCst);
        assert!(until_ms >= before_mark_ms.saturating_add(CANCEL_REOPEN_SUPPRESS_MS));
        assert!(until_ms <= after_mark_ms.saturating_add(CANCEL_REOPEN_SUPPRESS_MS));

        let remaining_ms = cancel_reopen_suppression_remaining_ms();
        assert!(remaining_ms > 0);
        assert!(remaining_ms <= CANCEL_REOPEN_SUPPRESS_MS);

        CANCEL_REOPEN_SUPPRESSION_UNTIL_MS.store(0, Ordering::SeqCst);
    }
}
