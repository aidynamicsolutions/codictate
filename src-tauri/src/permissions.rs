//! macOS permission checking utilities
//!
//! This module provides functions to check system permissions for accessibility
//! and microphone access on macOS. These are used to detect when permissions
//! are revoked at runtime and handle graceful degradation.

#[cfg(target_os = "macos")]
use tracing::{debug, warn};

/// Check if the app has accessibility permission on macOS.
///
/// This calls `AXIsProcessTrusted()` from ApplicationServices.framework.
/// Returns true if granted, false otherwise.
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        // macOS Boolean is actually u8 (unsigned char), not Rust bool
        fn AXIsProcessTrusted() -> u8;
    }

    let result = unsafe { AXIsProcessTrusted() };
    let is_trusted = result != 0;
    debug!("Accessibility permission check: {} (raw: {})", is_trusted, result);
    is_trusted
}

/// Check if the app has microphone permission on macOS.
///
/// Returns the current authorization status for audio capture.
/// Note: Using cpal, we can only reliably detect Authorized vs Denied.
/// For full AVCaptureDevice status, would need objc bindings.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophonePermission {
    /// User has granted permission (device is accessible)
    Authorized,
    /// User has denied permission or access is otherwise blocked
    Denied,
}

/// Check the current microphone permission status.
///
/// Uses tauri_plugin_macos_permissions which internally uses objc2 to properly
/// call AVCaptureDevice.authorizationStatus(for: .audio).
#[cfg(target_os = "macos")]
pub fn check_microphone_permission() -> MicrophonePermission {
    // The plugin's check_microphone_permission is an async command function,
    // but we need a sync version. We can replicate the logic directly here
    // using the same objc2 approach the plugin uses.
    
    use objc2::{class, msg_send};
    use objc2_foundation::NSString;
    
    let authorized = unsafe {
        let av_media_type = NSString::from_str("soun");
        let status: i32 = msg_send![
            class!(AVCaptureDevice),
            authorizationStatusForMediaType: &*av_media_type
        ];
        
        debug!("Microphone permission check: AVAuthorizationStatus = {}", status);
        
        // AVAuthorizationStatus values:
        // 0 = NotDetermined
        // 1 = Restricted
        // 2 = Denied
        // 3 = Authorized
        status == 3
    };
    
    if authorized {
        debug!("Microphone permission: Authorized");
        MicrophonePermission::Authorized
    } else {
        warn!("Microphone permission: Denied");
        MicrophonePermission::Denied
    }
}

// Stub implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    true // Assume always granted on non-macOS
}

#[cfg(not(target_os = "macos"))]
pub fn prompt_accessibility_permission() {
    // No-op on non-macOS
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophonePermission {
    Authorized,
    Denied,
}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone_permission() -> MicrophonePermission {
    MicrophonePermission::Authorized // Assume always granted on non-macOS
}

/// Open System Settings to the Accessibility privacy pane
#[cfg(target_os = "macos")]
#[tauri::command]
#[specta::specta]
pub fn open_accessibility_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map_err(|e| format!("Failed to open Accessibility settings: {}", e))?;
    Ok(())
}

/// Open System Settings to the Microphone privacy pane
#[cfg(target_os = "macos")]
#[tauri::command]
#[specta::specta]
pub fn open_microphone_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        .spawn()
        .map_err(|e| format!("Failed to open Microphone settings: {}", e))?;
    Ok(())
}

// Stub implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[specta::specta]
pub fn open_accessibility_settings() -> Result<(), String> {
    Ok(()) // No-op on non-macOS
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[specta::specta]
pub fn open_microphone_settings() -> Result<(), String> {
    Ok(()) // No-op on non-macOS
}
