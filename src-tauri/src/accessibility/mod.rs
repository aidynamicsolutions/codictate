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

/// Minimal boundary context for smart transcript insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextInsertionContext {
    /// Character immediately left of insertion/selection start.
    pub left_char: Option<char>,
    /// Nearest non-whitespace character to the left of insertion/selection start.
    pub left_non_whitespace_char: Option<char>,
    /// Second non-whitespace character to the left of insertion/selection start.
    /// Used for sentence-boundary checks across opening delimiters/quotes.
    pub left_second_non_whitespace_char: Option<char>,
    /// Effective non-whitespace sentence-boundary character to the left.
    /// This skips any run of opening delimiters/quotes to better match
    /// Unicode sentence-boundary behavior around punctuation clusters.
    pub left_sentence_boundary_char: Option<char>,
    /// Character immediately right of insertion/selection end.
    pub right_char: Option<char>,
    /// Nearest non-whitespace character to the right of insertion/selection end.
    pub right_non_whitespace_char: Option<char>,
    /// Second non-whitespace character to the right of insertion/selection end.
    /// Used for delimiter-aware smart insertion decisions (for example `.) word`).
    pub right_second_non_whitespace_char: Option<char>,
    /// True when a hard line-break appears before the next non-whitespace char
    /// on the right side of the insertion/selection end.
    pub right_has_line_break_before_non_whitespace: bool,
    /// True when a hard line-break appears between the first and second
    /// right-side non-whitespace characters.
    /// Used to prevent delimiter-aware sentence punctuation stripping across
    /// paragraph boundaries (for example `)...\nWord`).
    pub right_has_line_break_before_second_non_whitespace: bool,
    /// True if insertion target currently has a non-empty selection.
    pub has_selection: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayCursorScreenProbe {
    pub rect_x: f64,
    pub rect_y: f64,
    pub rect_width: f64,
    pub rect_height: f64,
    pub point_x: f64,
    pub point_y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayWindowScreenFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub(crate) fn is_sentence_boundary_prefix_delimiter(c: char) -> bool {
    matches!(
        c,
        '('
            | '['
            | '{'
            | '<'
            | '（'
            | '［'
            | '｛'
            | '「'
            | '『'
            | '【'
            | '〈'
            | '《'
            | '〘'
            | '〖'
            | '〚'
            | '“'
            | '‘'
            | '"'
            | '\''
    )
}

pub(crate) fn is_hard_line_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
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

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn select_text_in_app(
    _app_handle: &tauri::AppHandle,
    _text_to_find: &str,
) -> Result<(), String> {
    Err("Text selection is only supported on macOS".to_string())
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn select_text_in_app_last_occurrence(
    _app_handle: &tauri::AppHandle,
    _text_to_find: &str,
) -> Result<(), String> {
    Err("Text selection is only supported on macOS".to_string())
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn capture_insertion_context(
    _app_handle: &tauri::AppHandle,
) -> Option<TextInsertionContext> {
    None
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn capture_active_cursor_screen_probe(
    _app_handle: &tauri::AppHandle,
) -> Option<OverlayCursorScreenProbe> {
    None
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn capture_focused_window_screen_frame(
    _app_handle: &tauri::AppHandle,
) -> Option<OverlayWindowScreenFrame> {
    None
}
