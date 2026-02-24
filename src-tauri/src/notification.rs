//! Notification utility for sending native OS notifications with error/info differentiation.
//!
//! This module provides a simple API for showing native notifications with proper
//! localization and urgency differentiation between error and info notifications.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tracing::{debug, error, warn};
use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

use crate::i18n;

/// Notification type for differentiating urgency and display style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    /// Informational notification (e.g., recording limit warning)
    Info,
    /// Error notification (e.g., post-processing failed)
    Error,
}

const PERMISSION_NOTIFICATION_COOLDOWN: Duration = Duration::from_secs(10);
const MIC_PERMISSION_NOTIFICATION_ID: &str = "microphone-permission-denied";
const ACCESSIBILITY_PERMISSION_NOTIFICATION_ID: &str = "accessibility-permission-lost";

static NOTIFICATION_COOLDOWN_STATE: OnceLock<Mutex<HashMap<&'static str, Instant>>> = OnceLock::new();

fn should_emit_with_cooldown(
    tracker: &mut HashMap<&'static str, Instant>,
    notification_id: &'static str,
    now: Instant,
    cooldown: Duration,
) -> bool {
    match tracker.get(notification_id) {
        Some(last_shown_at) if now.duration_since(*last_shown_at) < cooldown => false,
        _ => {
            tracker.insert(notification_id, now);
            true
        }
    }
}

fn should_emit_permission_notification(notification_id: &'static str, now: Instant) -> bool {
    let cooldown_state = NOTIFICATION_COOLDOWN_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut tracker = match cooldown_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Notification cooldown state was poisoned, recovering");
            poisoned.into_inner()
        }
    };

    should_emit_with_cooldown(
        &mut tracker,
        notification_id,
        now,
        PERMISSION_NOTIFICATION_COOLDOWN,
    )
}

/// Show a native notification with the given type, title key, and body key.
///
/// # Arguments
/// * `app` - The Tauri AppHandle
/// * `notification_type` - Whether this is an Info or Error notification
/// * `title_key` - Translation key for the notification title (e.g., "notifications.errorTitle")
/// * `body_key` - Translation key for the notification body (e.g., "errors.postProcessFailed")
///
/// # Example
/// ```ignore
/// show_notification(&app, NotificationType::Error, "notifications.errorTitle", "errors.postProcessFailed");
/// ```
pub fn show_notification(
    app: &AppHandle,
    notification_type: NotificationType,
    title_key: &str,
    body_key: &str,
) {
    let title = i18n::t(app, title_key);
    let body = i18n::t(app, body_key);
    show_notification_with_text(app, notification_type, &title, &body);
}

/// Show a native notification with the given type and pre-resolved text.
///
/// Use this when you already have the translated/dynamic text.
///
/// # Arguments
/// * `app` - The Tauri AppHandle
/// * `notification_type` - Whether this is an Info or Error notification
/// * `title` - The notification title text
/// * `body` - The notification body text
pub fn show_notification_with_text(
    app: &AppHandle,
    notification_type: NotificationType,
    title: &str,
    body: &str,
) {
    // Format title with type prefix for visual differentiation
    let formatted_title = match notification_type {
        NotificationType::Info => title.to_string(),
        NotificationType::Error => {
            // Add error indicator if not already present
            if title.starts_with("⚠") || title.starts_with("❌") || title.starts_with("Error") {
                title.to_string()
            } else {
                format!("⚠ {}", title)
            }
        }
    };

    // Check permission status
    let permission = app.notification().permission_state();

    match permission {
        Ok(PermissionState::Granted) => {
            if let Err(e) = app
                .notification()
                .builder()
                .title(&formatted_title)
                .body(body)
                .show()
            {
                error!("Failed to show notification: {}", e);
            }
        }
        Ok(PermissionState::Denied) => {
            warn!("Notification permission denied by user");
        }
        Ok(PermissionState::Prompt) | Ok(PermissionState::PromptWithRationale) => {
            warn!("Notification permission not yet granted, attempting to show...");
            // Try to show anyway - this may trigger a permission prompt
            let _ = app
                .notification()
                .builder()
                .title(&formatted_title)
                .body(body)
                .show();
        }
        Err(e) => {
            error!("Failed to check notification permission: {}", e);
        }
    }
}

/// Convenience function to show an error notification.
///
/// # Arguments
/// * `app` - The Tauri AppHandle
/// * `body_key` - Translation key for the error message
pub fn show_error(app: &AppHandle, body_key: &str) {
    show_notification(
        app,
        NotificationType::Error,
        "notifications.errorTitle",
        body_key,
    );
}

/// Convenience function to show an info notification.
///
/// # Arguments
/// * `app` - The Tauri AppHandle
/// * `body_key` - Translation key for the info message
#[allow(dead_code)] // Part of public notification API, mirrors show_error
pub fn show_info(app: &AppHandle, body_key: &str) {
    show_notification(
        app,
        NotificationType::Info,
        "notifications.infoTitle",
        body_key,
    );
}

/// Convenience function to show an error notification with custom body text.
///
/// # Arguments
/// * `app` - The Tauri AppHandle
/// * `body` - The error message text (not a translation key)
#[allow(dead_code)] // Part of public notification API, mirrors show_info_with_text
pub fn show_error_with_text(app: &AppHandle, body: &str) {
    let title = i18n::t(app, "notifications.errorTitle");
    show_notification_with_text(app, NotificationType::Error, &title, body);
}

/// Convenience function to show an info notification with custom body text.
///
/// # Arguments
/// * `app` - The Tauri AppHandle
/// * `body` - The info message text (not a translation key)
pub fn show_info_with_text(app: &AppHandle, body: &str) {
    let title = i18n::t(app, "notifications.infoTitle");
    show_notification_with_text(app, NotificationType::Info, &title, body);
}

/// Notify the user that an upgrade prompt is available after the aha moment.
pub fn show_upgrade_prompt_nudge(app: &AppHandle) {
    let title = i18n::t(app, "growth.upgradePrompt.title");
    let body = i18n::t(app, "growth.upgradePrompt.description");
    let fallback_title = "You're in the flow. Get even more done.";
    let fallback_body =
        "Unlock advanced features to speed up editing, cleanup, and correction in your daily workflow.";
    let resolved_title = if title == "growth.upgradePrompt.title" {
        fallback_title.to_string()
    } else {
        title
    };
    let resolved_body = if body == "growth.upgradePrompt.description" {
        fallback_body.to_string()
    } else {
        body
    };
    show_notification_with_text(app, NotificationType::Info, &resolved_title, &resolved_body);
}

/// Notify user that microphone permission is required without forcing the app window to focus.
pub fn show_microphone_permission_denied(app: &AppHandle) {
    if !should_emit_permission_notification(MIC_PERMISSION_NOTIFICATION_ID, Instant::now()) {
        debug!("Skipping microphone permission notification due to cooldown");
        return;
    }

    show_notification(
        app,
        NotificationType::Info,
        "permissions.microphoneDeniedTitle",
        "permissions.microphoneDenied",
    );
}

/// Notify user that accessibility permission was revoked without forcing the app window to focus.
pub fn show_accessibility_permission_lost(app: &AppHandle) {
    if !should_emit_permission_notification(ACCESSIBILITY_PERMISSION_NOTIFICATION_ID, Instant::now())
    {
        debug!("Skipping accessibility permission notification due to cooldown");
        return;
    }

    show_notification(
        app,
        NotificationType::Info,
        "permissions.accessibilityLostTitle",
        "permissions.accessibilityLost",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_allows_first_notification() {
        let mut tracker = HashMap::new();
        let now = Instant::now();
        assert!(should_emit_with_cooldown(
            &mut tracker,
            "mic",
            now,
            Duration::from_secs(10),
        ));
    }

    #[test]
    fn cooldown_blocks_repeat_within_window() {
        let mut tracker = HashMap::new();
        let now = Instant::now();

        assert!(should_emit_with_cooldown(
            &mut tracker,
            "mic",
            now,
            Duration::from_secs(10),
        ));
        assert!(!should_emit_with_cooldown(
            &mut tracker,
            "mic",
            now + Duration::from_secs(3),
            Duration::from_secs(10),
        ));
    }

    #[test]
    fn cooldown_allows_after_window_expires() {
        let mut tracker = HashMap::new();
        let now = Instant::now();

        assert!(should_emit_with_cooldown(
            &mut tracker,
            "mic",
            now,
            Duration::from_secs(10),
        ));
        assert!(should_emit_with_cooldown(
            &mut tracker,
            "mic",
            now + Duration::from_secs(11),
            Duration::from_secs(10),
        ));
    }

    #[test]
    fn cooldown_is_tracked_per_notification_id() {
        let mut tracker = HashMap::new();
        let now = Instant::now();

        assert!(should_emit_with_cooldown(
            &mut tracker,
            "mic",
            now,
            Duration::from_secs(10),
        ));
        assert!(should_emit_with_cooldown(
            &mut tracker,
            "accessibility",
            now + Duration::from_secs(1),
            Duration::from_secs(10),
        ));
    }
}
