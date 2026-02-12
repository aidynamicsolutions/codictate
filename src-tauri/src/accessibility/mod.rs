//! macOS Accessibility API integration for text context capture.
//!
//! Uses AXUIElement to read text, selection, and cursor position from
//! the currently focused application. Falls back to clipboard simulation
//! when the AX API fails (e.g. in Electron apps).

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::*;

/// Captured text context from the focused application.
#[derive(Debug, Clone)]
pub struct CapturedContext {
    /// The currently selected text (if any).
    pub selected_text: Option<String>,
    /// Surrounding text context (±radius characters around cursor).
    pub context: String,
    /// Screen position (x, y) of the text cursor for overlay positioning.
    /// Populated but not yet consumed — will be used for cursor-relative overlay positioning.
    #[allow(dead_code)]
    pub cursor_screen_position: (f64, f64),
}

/// Result of an AI correction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorrectionResult {
    /// The original text that was targeted for correction.
    pub original: String,
    /// The corrected text from the AI.
    pub corrected: String,
    /// Whether the AI actually changed anything.
    pub has_changes: bool,
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn capture_context(_app_handle: &tauri::AppHandle) -> Result<CapturedContext, String> {
    Err("Context capture is only supported on macOS".to_string())
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn replace_text_in_app(
    _app_handle: &tauri::AppHandle,
    _original: &str,
    _replacement: &str,
) -> Result<(), String> {
    Err("Text replacement is only supported on macOS".to_string())
}
