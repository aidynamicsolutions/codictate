//! Notification utility for sending native OS notifications with error/info differentiation.
//!
//! This module provides a simple API for showing native notifications with proper
//! localization and urgency differentiation between error and info notifications.

use tracing::{error, warn};
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
