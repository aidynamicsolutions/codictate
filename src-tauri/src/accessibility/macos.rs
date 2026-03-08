//! macOS-specific context capture using Accessibility (AXUIElement) API.
//!
//! This module reads text, selection range, and cursor position from
//! the focused application. Falls back to clipboard simulation when
//! the AX API doesn't return usable data.

use core_foundation::base::{CFGetTypeID, CFRelease, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringGetTypeID, CFStringRef};
use std::time::Instant;
use tracing::{debug, error, info, warn};

use super::{
    CapturedContext, OverlayCursorScreenProbe, OverlayWindowScreenFrame, TextInsertionContext,
};

// ─── AXUIElement FFI ────────────────────────────────────────────────

// These types mirror Apple's ApplicationServices / HIServices definitions.
// AXUIElementRef is an opaque CFTypeRef.
type AXUIElementRef = CFTypeRef;
type AXError = i32;

const K_AX_ERROR_SUCCESS: AXError = 0;
const K_AX_ERROR_CANNOT_COMPLETE: AXError = -25204;
const INSERTION_CONTEXT_AX_TIMEOUT_SECONDS: f32 = 0.15;
const OVERLAY_CURSOR_MONITOR_AX_TIMEOUT_SECONDS: f32 = 0.08;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        parameter: CFTypeRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout_in_seconds: f32) -> AXError;
}

// Attribute name constants
fn ax_focused_application() -> CFString {
    CFString::new("AXFocusedApplication")
}
fn ax_focused_ui_element() -> CFString {
    CFString::new("AXFocusedUIElement")
}
fn ax_focused_window() -> CFString {
    CFString::new("AXFocusedWindow")
}
fn ax_selected_text() -> CFString {
    CFString::new("AXSelectedText")
}
fn ax_value() -> CFString {
    CFString::new("AXValue")
}
fn ax_selected_text_range() -> CFString {
    CFString::new("AXSelectedTextRange")
}
fn ax_bounds_for_range() -> CFString {
    CFString::new("AXBoundsForRange")
}
fn ax_position() -> CFString {
    CFString::new("AXPosition")
}
fn ax_size() -> CFString {
    CFString::new("AXSize")
}

// ─── CFRange / AXValue helpers ──────────────────────────────────────

/// An NSRange / CFRange equivalent.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CFRange {
    location: i64,
    length: i64,
}

/// A CGRect-like struct for reading bounds via AXValue.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AXRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AXPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AXSize {
    width: f64,
    height: f64,
}

// AXValueType constants (from HIServices/AXValue.h)
const K_AX_VALUE_TYPE_CG_POINT: u32 = 1;
const K_AX_VALUE_TYPE_CG_SIZE: u32 = 2;
const K_AX_VALUE_TYPE_CF_RANGE: u32 = 4;
const K_AX_VALUE_TYPE_CG_RECT: u32 = 3;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXValueGetValue(value: CFTypeRef, value_type: u32, out: *mut std::ffi::c_void) -> bool;
    fn AXValueCreate(value_type: u32, data: *const std::ffi::c_void) -> CFTypeRef;
}

// ─── Core functions ─────────────────────────────────────────────────

/// Get the AXUIElement for the focused UI element across all apps.
#[derive(Debug)]
struct AxLookupError {
    message: String,
    code: Option<AXError>,
}

fn classify_ax_error_reason(code: Option<AXError>) -> &'static str {
    match code {
        Some(K_AX_ERROR_CANNOT_COMPLETE) => "ax_cannot_complete",
        Some(_) => "ax_lookup_failed",
        None => "ax_unknown",
    }
}

fn get_focused_element() -> Result<AXUIElementRef, String> {
    get_focused_element_with_timeout(None).map_err(|err| err.message)
}

fn get_focused_element_with_timeout(
    timeout_in_seconds: Option<f32>,
) -> Result<AXUIElementRef, AxLookupError> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            error!("AXUIElementCreateSystemWide returned null");
            return Err(AxLookupError {
                message: "Failed to create system-wide AX element".to_string(),
                code: None,
            });
        }

        let mut timeout_applied = false;
        if let Some(timeout_seconds) = timeout_in_seconds {
            let timeout_err = AXUIElementSetMessagingTimeout(system_wide, timeout_seconds);
            if timeout_err == K_AX_ERROR_SUCCESS {
                timeout_applied = true;
            } else {
                debug!(
                    timeout_seconds,
                    ax_error = timeout_err,
                    "Failed to set AX messaging timeout; using default timeout"
                );
            }
        }

        // First get the focused app
        let mut focused_app: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            ax_focused_application().as_concrete_TypeRef(),
            &mut focused_app,
        );
        if err != K_AX_ERROR_SUCCESS || focused_app.is_null() {
            if timeout_applied {
                let _ = AXUIElementSetMessagingTimeout(system_wide, 0.0);
            }
            CFRelease(system_wide);
            let msg = format!("Failed to get focused application (AXError: {})", err);
            warn!("{}", msg);
            return Err(AxLookupError {
                message: msg,
                code: Some(err),
            });
        }
        debug!("Got focused application AXUIElement");

        // Then get the focused UI element within that app
        let mut focused_element: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            focused_app,
            ax_focused_ui_element().as_concrete_TypeRef(),
            &mut focused_element,
        );

        if timeout_applied {
            let reset_err = AXUIElementSetMessagingTimeout(system_wide, 0.0);
            if reset_err != K_AX_ERROR_SUCCESS {
                debug!(
                    ax_error = reset_err,
                    "Failed to reset AX messaging timeout to default"
                );
            }
        }

        CFRelease(focused_app);
        CFRelease(system_wide);

        if err != K_AX_ERROR_SUCCESS || focused_element.is_null() {
            let msg = format!("Failed to get focused UI element (AXError: {})", err);
            warn!("{}", msg);
            return Err(AxLookupError {
                message: msg,
                code: Some(err),
            });
        }

        debug!("Got focused UI element");
        Ok(focused_element)
    }
}

/// Read the AXSelectedText attribute (the currently selected text).
fn get_selected_text(element: AXUIElementRef) -> Option<String> {
    unsafe {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            ax_selected_text().as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            debug!("No selected text via AX (AXError: {})", err);
            return None;
        }
        // Guard: verify the returned value is actually a CFString before casting.
        // Some apps return AXValue or CFNumber here, which would segfault.
        if CFGetTypeID(value) != CFStringGetTypeID() {
            warn!(
                type_id = CFGetTypeID(value),
                expected = CFStringGetTypeID(),
                "AXSelectedText returned non-CFString type, skipping"
            );
            CFRelease(value);
            return None;
        }
        let cf_string = CFString::wrap_under_create_rule(value as CFStringRef);
        let s = cf_string.to_string();
        if s.is_empty() {
            debug!("Selected text is empty");
            None
        } else {
            debug!(chars = s.len(), "Got selected text from AX");
            Some(s)
        }
    }
}

/// Read the full AXValue (the entire text content of the element).
fn get_full_text(element: AXUIElementRef) -> Option<String> {
    unsafe {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            ax_value().as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            debug!("Failed to get AXValue (full text) (AXError: {})", err);
            return None;
        }
        // Guard: verify the returned value is actually a CFString before casting.
        // Some apps return AXValue or other CF types here, which would segfault.
        if CFGetTypeID(value) != CFStringGetTypeID() {
            warn!(
                type_id = CFGetTypeID(value),
                expected = CFStringGetTypeID(),
                "AXValue returned non-CFString type, skipping"
            );
            CFRelease(value);
            return None;
        }
        let cf_string = CFString::wrap_under_create_rule(value as CFStringRef);
        let s = cf_string.to_string();
        debug!(chars = s.len(), "Got full text from AX");
        Some(s)
    }
}

/// Read the AXSelectedTextRange → CFRange (location + length of the selection/cursor).
fn get_selected_text_range(element: AXUIElementRef) -> Option<CFRange> {
    unsafe {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            ax_selected_text_range().as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            debug!("Failed to get AXSelectedTextRange (AXError: {})", err);
            return None;
        }
        let mut range = CFRange {
            location: 0,
            length: 0,
        };
        let ok = AXValueGetValue(
            value,
            K_AX_VALUE_TYPE_CF_RANGE,
            &mut range as *mut CFRange as *mut std::ffi::c_void,
        );
        CFRelease(value);
        if ok {
            debug!(
                location = range.location,
                length = range.length,
                "Got selected text range"
            );
            Some(range)
        } else {
            warn!("AXValueGetValue failed for CFRange");
            None
        }
    }
}

/// Get the screen bounds of a text range via AXBoundsForRange.
/// Returns the CGRect bounds for the requested range.
fn get_rect_for_range(element: AXUIElementRef, range: CFRange) -> Option<AXRect> {
    unsafe {
        // Create an AXValue wrapping the CFRange
        let range_value = AXValueCreate(
            K_AX_VALUE_TYPE_CF_RANGE,
            &range as *const CFRange as *const std::ffi::c_void,
        );
        if range_value.is_null() {
            warn!("AXValueCreate for range failed");
            return None;
        }

        let mut bounds_value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyParameterizedAttributeValue(
            element,
            ax_bounds_for_range().as_concrete_TypeRef(),
            range_value,
            &mut bounds_value,
        );
        CFRelease(range_value);

        if err != K_AX_ERROR_SUCCESS || bounds_value.is_null() {
            debug!(
                "AXBoundsForRange failed (AXError: {}), falling back to mouse position",
                err
            );
            return None;
        }

        let mut rect = AXRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        let ok = AXValueGetValue(
            bounds_value,
            K_AX_VALUE_TYPE_CG_RECT,
            &mut rect as *mut AXRect as *mut std::ffi::c_void,
        );
        CFRelease(bounds_value);

        if ok {
            debug!(x = rect.x, y = rect.y, w = rect.width, h = rect.height, "Got bounds for range");
            Some(rect)
        } else {
            warn!("AXValueGetValue failed for CGRect bounds");
            None
        }
    }
}

fn get_attribute_ax_point(element: AXUIElementRef, attribute: CFString) -> Option<AXPoint> {
    unsafe {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            attribute.as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            debug!("Failed to get AXPoint attribute (AXError: {})", err);
            return None;
        }

        let mut point = AXPoint { x: 0.0, y: 0.0 };
        let ok = AXValueGetValue(
            value,
            K_AX_VALUE_TYPE_CG_POINT,
            &mut point as *mut AXPoint as *mut std::ffi::c_void,
        );
        CFRelease(value);

        if ok {
            Some(point)
        } else {
            warn!("AXValueGetValue failed for CGPoint attribute");
            None
        }
    }
}

fn get_attribute_ax_size(element: AXUIElementRef, attribute: CFString) -> Option<AXSize> {
    unsafe {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            attribute.as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            debug!("Failed to get AXSize attribute (AXError: {})", err);
            return None;
        }

        let mut size = AXSize {
            width: 0.0,
            height: 0.0,
        };
        let ok = AXValueGetValue(
            value,
            K_AX_VALUE_TYPE_CG_SIZE,
            &mut size as *mut AXSize as *mut std::ffi::c_void,
        );
        CFRelease(value);

        if ok {
            Some(size)
        } else {
            warn!("AXValueGetValue failed for CGSize attribute");
            None
        }
    }
}

fn get_focused_window_rect_with_timeout(timeout_in_seconds: Option<f32>) -> Result<AXRect, AxLookupError> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return Err(AxLookupError {
                message: "Failed to create system-wide AX element".to_string(),
                code: None,
            });
        }

        let mut timeout_applied = false;
        if let Some(timeout_seconds) = timeout_in_seconds {
            let timeout_err = AXUIElementSetMessagingTimeout(system_wide, timeout_seconds);
            if timeout_err == K_AX_ERROR_SUCCESS {
                timeout_applied = true;
            }
        }

        let mut focused_app: CFTypeRef = std::ptr::null();
        let focused_app_err = AXUIElementCopyAttributeValue(
            system_wide,
            ax_focused_application().as_concrete_TypeRef(),
            &mut focused_app,
        );
        if focused_app_err != K_AX_ERROR_SUCCESS || focused_app.is_null() {
            if timeout_applied {
                let _ = AXUIElementSetMessagingTimeout(system_wide, 0.0);
            }
            CFRelease(system_wide);
            return Err(AxLookupError {
                message: format!(
                    "Failed to get focused application for focused window (AXError: {})",
                    focused_app_err
                ),
                code: Some(focused_app_err),
            });
        }

        // Propagate the timeout to focused_app so the AXFocusedWindow query
        // is also bounded. AXUIElementSetMessagingTimeout is per-element and
        // does not inherit from the system-wide element.
        if let Some(timeout_seconds) = timeout_in_seconds {
            let _ = AXUIElementSetMessagingTimeout(focused_app as AXUIElementRef, timeout_seconds);
        }

        let mut focused_window: CFTypeRef = std::ptr::null();
        let focused_window_err = AXUIElementCopyAttributeValue(
            focused_app,
            ax_focused_window().as_concrete_TypeRef(),
            &mut focused_window,
        );

        if timeout_applied {
            let _ = AXUIElementSetMessagingTimeout(system_wide, 0.0);
        }

        CFRelease(focused_app);
        CFRelease(system_wide);

        if focused_window_err != K_AX_ERROR_SUCCESS || focused_window.is_null() {
            return Err(AxLookupError {
                message: format!(
                    "Failed to get focused window (AXError: {})",
                    focused_window_err
                ),
                code: Some(focused_window_err),
            });
        }

        let position = get_attribute_ax_point(focused_window, ax_position());
        let size = get_attribute_ax_size(focused_window, ax_size());
        CFRelease(focused_window);

        match (position, size) {
            (Some(position), Some(size)) => Ok(AXRect {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            }),
            _ => Err(AxLookupError {
                message: "Focused window frame attributes unavailable".to_string(),
                code: None,
            }),
        }
    }
}

/// Get the screen bounds of a text range via AXBoundsForRange.
/// Returns (x, y) of the bottom-left corner of that range's bounding rect.
fn get_bounds_for_range(element: AXUIElementRef, range: CFRange) -> Option<(f64, f64)> {
    let rect = get_rect_for_range(element, range)?;
    Some((rect.x, rect.y + rect.height))
}

/// Get the text cursor's screen position.  
/// Tries AXBoundsForRange first, falls back to mouse cursor position.
fn get_cursor_screen_position(element: AXUIElementRef) -> (f64, f64) {
    // Try to get position from the text cursor via AXBoundsForRange
    if let Some(range) = get_selected_text_range(element) {
        // Use a zero-length range at the cursor position
        let cursor_range = CFRange {
            location: range.location,
            length: 1.coerced_to(range.length.max(1)), // At least 1 char for bounds
        };
        if let Some(pos) = get_bounds_for_range(element, cursor_range) {
            debug!(x = pos.0, y = pos.1, "Cursor position from AXBoundsForRange");
            return pos;
        }
    }

    // Fallback: use mouse cursor position
    let mouse_pos = get_mouse_position();
    debug!(
        x = mouse_pos.0,
        y = mouse_pos.1,
        "Cursor position from mouse fallback"
    );
    mouse_pos
}

/// Get mouse cursor position via CGEvent.
fn get_mouse_position() -> (f64, f64) {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState);
    match source {
        Ok(src) => {
            let event = CGEvent::new(src);
            match event {
                Ok(e) => {
                    let loc = e.location();
                    (loc.x, loc.y)
                }
                Err(_) => {
                    warn!("Failed to create CGEvent for mouse position");
                    (0.0, 0.0)
                }
            }
        }
        Err(_) => {
            warn!("Failed to create CGEventSource for mouse position");
            (0.0, 0.0)
        }
    }
}

// ─── Smart Selection ────────────────────────────────────────────────

/// Expand a cursor position to the nearest word boundaries.
/// Returns (word_start_offset, word_text) in the given text.
pub fn expand_to_word_boundaries(text: &str, cursor_pos: usize) -> Option<(usize, String)> {
    if text.is_empty() || cursor_pos > text.len() {
        return None;
    }

    let chars: Vec<char> = text.chars().collect();
    let char_count = chars.len();
    
    // Convert byte offset to char offset (approximate)
    let char_pos = text[..cursor_pos.min(text.len())]
        .chars()
        .count()
        .min(char_count);

    // Walk left to find word start
    let mut start = char_pos;
    while start > 0 && chars[start - 1].is_alphanumeric() {
        start -= 1;
    }

    // Walk right to find word end
    let mut end = char_pos;
    while end < char_count && chars[end].is_alphanumeric() {
        end += 1;
    }

    if start == end {
        debug!("No word found at cursor position {}", cursor_pos);
        return None;
    }

    let word: String = chars[start..end].iter().collect();
    // Convert char offset back to byte offset
    let byte_start: usize = chars[..start].iter().map(|c| c.len_utf8()).sum();
    debug!(
        word = word,
        byte_start = byte_start,
        "Expanded cursor to word boundaries"
    );
    Some((byte_start, word))
}

// ─── Context extraction ─────────────────────────────────────────────

/// Extract surrounding context text around the cursor position.
/// Returns up to `radius` characters before and after the cursor.
fn extract_context(full_text: &str, cursor_pos: usize, radius: usize) -> String {
    let len = full_text.len();
    let start = cursor_pos.saturating_sub(radius);
    let end = (cursor_pos + radius).min(len);

    // Clamp to valid UTF-8 boundaries
    let start = full_text
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= start)
        .unwrap_or(0);
    let end = full_text
        .char_indices()
        .map(|(i, c)| i + c.len_utf8())
        .rev()
        .find(|&i| i <= end)
        .unwrap_or(len);

    full_text[start..end].to_string()
}

/// Convert a UTF-16 code-unit offset into a UTF-8 byte index.
///
/// AXSelectedTextRange uses UTF-16 units. For insertion/selection start we
/// round down to the nearest scalar boundary when an offset lands inside a
/// surrogate pair.
fn utf16_offset_to_byte_floor(text: &str, utf16_offset: usize) -> Option<usize> {
    if utf16_offset == 0 {
        return Some(0);
    }

    let mut utf16_units_seen = 0usize;
    for (byte_index, ch) in text.char_indices() {
        if utf16_units_seen == utf16_offset {
            return Some(byte_index);
        }
        let next = utf16_units_seen.checked_add(ch.len_utf16())?;
        if next > utf16_offset {
            return Some(byte_index);
        }
        utf16_units_seen = next;
    }

    if utf16_units_seen == utf16_offset {
        Some(text.len())
    } else {
        None
    }
}

/// Convert a UTF-16 code-unit offset into a UTF-8 byte index.
///
/// For insertion/selection end we round up to the nearest scalar boundary
/// when an offset lands inside a surrogate pair.
fn utf16_offset_to_byte_ceil(text: &str, utf16_offset: usize) -> Option<usize> {
    if utf16_offset == 0 {
        return Some(0);
    }

    let mut utf16_units_seen = 0usize;
    for (byte_index, ch) in text.char_indices() {
        if utf16_units_seen == utf16_offset {
            return Some(byte_index);
        }
        let next = utf16_units_seen.checked_add(ch.len_utf16())?;
        if next >= utf16_offset {
            return Some(byte_index + ch.len_utf8());
        }
        utf16_units_seen = next;
    }

    if utf16_units_seen == utf16_offset {
        Some(text.len())
    } else {
        None
    }
}

fn build_insertion_context(full_text: &str, range: CFRange) -> Option<TextInsertionContext> {
    if range.location < 0 || range.length < 0 {
        return None;
    }

    // AXSelectedTextRange is UTF-16 based; convert to UTF-8 byte indices
    // before slicing Rust strings.
    let start_utf16 = range.location as usize;
    let end_utf16 = start_utf16.checked_add(range.length as usize)?;
    let start = utf16_offset_to_byte_floor(full_text, start_utf16)?;
    let end = utf16_offset_to_byte_ceil(full_text, end_utf16)?;
    let (start, end) = if end < start {
        (start, start)
    } else {
        (start, end)
    };

    let left_slice = &full_text[..start];
    let right_slice = &full_text[end..];
    let mut left_non_whitespace_chars = left_slice.chars().rev().filter(|c| !c.is_whitespace());
    let left_non_whitespace_char = left_non_whitespace_chars.next();
    let left_second_non_whitespace_char = left_non_whitespace_chars.next();
    let left_sentence_boundary_char = left_slice
        .chars()
        .rev()
        .filter(|c| !c.is_whitespace())
        .find(|c| !super::is_sentence_boundary_prefix_delimiter(*c));
    let right_has_line_break_before_non_whitespace = right_slice
        .chars()
        .take_while(|c| c.is_whitespace())
        .any(super::is_hard_line_break);
    let mut right_non_whitespace_chars = right_slice.char_indices().filter(|(_, c)| !c.is_whitespace());
    let first_right_non_whitespace = right_non_whitespace_chars.next();
    let second_right_non_whitespace = right_non_whitespace_chars.next();
    let right_non_whitespace_char = first_right_non_whitespace.map(|(_, c)| c);
    let right_second_non_whitespace_char = second_right_non_whitespace.map(|(_, c)| c);
    let right_has_line_break_before_second_non_whitespace = first_right_non_whitespace
        .zip(second_right_non_whitespace)
        .map(|((first_idx, first_char), (second_idx, _))| {
            right_slice[first_idx + first_char.len_utf8()..second_idx]
                .chars()
                .any(super::is_hard_line_break)
        })
        .unwrap_or(false);

    Some(TextInsertionContext {
        left_char: left_slice.chars().next_back(),
        left_non_whitespace_char,
        left_second_non_whitespace_char,
        left_sentence_boundary_char,
        right_char: right_slice.chars().next(),
        right_non_whitespace_char,
        right_second_non_whitespace_char,
        right_has_line_break_before_non_whitespace,
        right_has_line_break_before_second_non_whitespace,
        has_selection: end > start,
    })
}

// ─── Clipboard fallback ─────────────────────────────────────────────

/// Fallback: simulate Cmd+C to capture selection via clipboard.
/// Saves and restores the original clipboard content.
fn capture_via_clipboard(app_handle: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager;
    use tauri_plugin_clipboard_manager::ClipboardExt;
    
    info!("AX API failed, attempting clipboard fallback");

    // 1. Save current clipboard
    let clipboard_backup = {
        let clip = app_handle.clipboard();
        clip.read_text().ok()
    };
    debug!(
        has_backup = clipboard_backup.is_some(),
        "Saved clipboard for restore"
    );

    // 2. Simulate Cmd+C
    let enigo_state = app_handle.try_state::<crate::input::EnigoState>();
    if let Some(enigo_state) = enigo_state {
        let mut guard = enigo_state.0.lock().unwrap();
        if let Some(ref mut enigo) = *guard {
            if let Err(e) = crate::input::send_copy_cmd_c(enigo) {
                warn!("Failed to simulate Cmd+C: {}", e);
                return None;
            }
        } else {
            warn!("Enigo not initialized, clipboard fallback failed");
            return None;
        }
    } else {
        warn!("EnigoState not available, clipboard fallback failed");
        return None;
    }

    // 3. Wait for clipboard to update
    std::thread::sleep(std::time::Duration::from_millis(100));

    // 4. Read the newly copied text
    let captured: Option<String> = {
        let clip = app_handle.clipboard();
        clip.read_text().ok()
    };

    // 5. Restore original clipboard
    if let Some(backup) = clipboard_backup {
        let clip = app_handle.clipboard();
        let _ = clip.write_text(backup);
        debug!("Restored original clipboard content");
    }

    if let Some(ref text) = captured {
        debug!(chars = text.len(), "Captured text via clipboard fallback");
    } else {
        debug!("Clipboard fallback returned no text");
    }

    captured
}

// ─── Public API ─────────────────────────────────────────────────────

/// Capture lightweight boundary context used for smart transcript insertion.
///
/// This intentionally avoids clipboard fallback to prevent side-effects during
/// regular paste operations.
pub fn capture_insertion_context(_app_handle: &tauri::AppHandle) -> Option<TextInsertionContext> {
    let capture_start = Instant::now();

    if !crate::permissions::check_accessibility_permission() {
        debug!(
            event_code = "insertion_context_capture_completed",
            context_available = false,
            fallback_reason = "accessibility_permission_missing",
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Skipping insertion context capture: accessibility permission missing"
        );
        return None;
    }

    let focused_element_lookup_start = Instant::now();
    let element = match get_focused_element_with_timeout(Some(INSERTION_CONTEXT_AX_TIMEOUT_SECONDS))
    {
        Ok(el) => el,
        Err(err) => {
            let lookup_elapsed_ms = focused_element_lookup_start.elapsed().as_millis();
            let fallback_reason = classify_ax_error_reason(err.code);
            debug!(
                event_code = "insertion_context_capture_completed",
                context_available = false,
                fallback_reason,
                focused_lookup_elapsed_ms = lookup_elapsed_ms,
                total_elapsed_ms = capture_start.elapsed().as_millis(),
                "Skipping insertion context capture: focused element unavailable ({})",
                err.message
            );
            return None;
        }
    };
    let focused_lookup_elapsed_ms = focused_element_lookup_start.elapsed().as_millis();

    let full_text_lookup_start = Instant::now();
    let full_text = get_full_text(element);
    let full_text_lookup_elapsed_ms = full_text_lookup_start.elapsed().as_millis();
    let range_lookup_start = Instant::now();
    let range = get_selected_text_range(element);
    let range_lookup_elapsed_ms = range_lookup_start.elapsed().as_millis();

    unsafe { CFRelease(element) };

    let Some(full_text) = full_text else {
        debug!(
            event_code = "insertion_context_capture_completed",
            context_available = false,
            fallback_reason = "ax_value_unavailable",
            focused_lookup_elapsed_ms,
            full_text_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Skipping insertion context capture: AXValue text unavailable"
        );
        return None;
    };
    let Some(range) = range else {
        debug!(
            event_code = "insertion_context_capture_completed",
            context_available = false,
            fallback_reason = "ax_selected_text_range_unavailable",
            focused_lookup_elapsed_ms,
            full_text_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Skipping insertion context capture: AXSelectedTextRange unavailable"
        );
        return None;
    };

    let context_build_start = Instant::now();
    let context = build_insertion_context(&full_text, range);
    let context_build_elapsed_ms = context_build_start.elapsed().as_millis();
    if context.is_none() {
        debug!(
            event_code = "insertion_context_capture_completed",
            context_available = false,
            fallback_reason = "invalid_text_range",
            focused_lookup_elapsed_ms,
            full_text_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            context_build_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Skipping insertion context capture: invalid text range"
        );
    } else {
        debug!(
            event_code = "insertion_context_capture_completed",
            context_available = true,
            fallback_reason = "none",
            focused_lookup_elapsed_ms,
            full_text_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            context_build_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Completed insertion context capture"
        );
    }
    context
}

/// Capture only the active insertion point screen position for overlay monitor routing.
///
/// This is intentionally lightweight:
/// - uses a strict AX timeout to avoid blocking overlay show
/// - does not fall back to clipboard or mouse position
/// - returns `None` when cursor geometry cannot be resolved quickly
pub fn capture_active_cursor_screen_probe(
    _app_handle: &tauri::AppHandle,
) -> Option<OverlayCursorScreenProbe> {
    let capture_start = Instant::now();

    let focused_lookup_start = Instant::now();
    let element = match get_focused_element_with_timeout(Some(
        OVERLAY_CURSOR_MONITOR_AX_TIMEOUT_SECONDS,
    )) {
        Ok(element) => element,
        Err(err) => {
            debug!(
                event_code = "overlay_cursor_position_capture_completed",
                position_available = false,
                fallback_reason = classify_ax_error_reason(err.code),
                focused_lookup_elapsed_ms = focused_lookup_start.elapsed().as_millis(),
                total_elapsed_ms = capture_start.elapsed().as_millis(),
                "Skipping overlay cursor position capture: focused element unavailable ({})",
                err.message
            );
            return None;
        }
    };
    let focused_lookup_elapsed_ms = focused_lookup_start.elapsed().as_millis();

    let range_lookup_start = Instant::now();
    let range = get_selected_text_range(element);
    let range_lookup_elapsed_ms = range_lookup_start.elapsed().as_millis();
    let Some(range) = range else {
        unsafe { CFRelease(element) };
        debug!(
            event_code = "overlay_cursor_position_capture_completed",
            position_available = false,
            fallback_reason = "missing_selected_text_range",
            focused_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Skipping overlay cursor position capture: selected text range unavailable"
        );
        return None;
    };

    let bounds_lookup_start = Instant::now();
    let cursor_range = CFRange {
        location: range.location,
        length: 1.coerced_to(range.length.max(1)),
    };
    let rect = get_rect_for_range(element, cursor_range);
    let bounds_lookup_elapsed_ms = bounds_lookup_start.elapsed().as_millis();
    unsafe { CFRelease(element) };

    if let Some(rect) = rect {
        // Probe slightly inside the caret rect to avoid monitor seam/border ambiguity.
        let vertical_inset = if rect.height >= 1.0 {
            0.5
        } else {
            (rect.height / 2.0).max(0.0)
        };
        let x = rect.x;
        let y = rect.y + rect.height - vertical_inset;
        debug!(
            event_code = "overlay_cursor_position_capture_completed",
            position_available = true,
            fallback_reason = "none",
            focused_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            bounds_lookup_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            rect_x = rect.x,
            rect_y = rect.y,
            rect_width = rect.width,
            rect_height = rect.height,
            cursor_x = x,
            cursor_y = y,
            "Captured overlay cursor position"
        );
        Some(OverlayCursorScreenProbe {
            rect_x: rect.x,
            rect_y: rect.y,
            rect_width: rect.width,
            rect_height: rect.height,
            point_x: x,
            point_y: y,
        })
    } else {
        debug!(
            event_code = "overlay_cursor_position_capture_completed",
            position_available = false,
            fallback_reason = "bounds_unavailable",
            focused_lookup_elapsed_ms,
            range_lookup_elapsed_ms,
            bounds_lookup_elapsed_ms,
            total_elapsed_ms = capture_start.elapsed().as_millis(),
            "Skipping overlay cursor position capture: AX bounds unavailable"
        );
        None
    }
}

/// Capture the focused window frame in screen coordinates for overlay monitor routing.
///
/// This is a low-cost fallback used when caret geometry is unavailable.
pub fn capture_focused_window_screen_frame(
    _app_handle: &tauri::AppHandle,
) -> Option<OverlayWindowScreenFrame> {
    let capture_start = Instant::now();

    match get_focused_window_rect_with_timeout(Some(
        OVERLAY_CURSOR_MONITOR_AX_TIMEOUT_SECONDS,
    )) {
        Ok(rect) => {
            debug!(
                event_code = "overlay_focused_window_capture_completed",
                frame_available = true,
                fallback_reason = "none",
                total_elapsed_ms = capture_start.elapsed().as_millis(),
                rect_x = rect.x,
                rect_y = rect.y,
                rect_width = rect.width,
                rect_height = rect.height,
                "Captured focused window frame for overlay routing"
            );
            Some(OverlayWindowScreenFrame {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            })
        }
        Err(err) => {
            debug!(
                event_code = "overlay_focused_window_capture_completed",
                frame_available = false,
                fallback_reason = classify_ax_error_reason(err.code),
                total_elapsed_ms = capture_start.elapsed().as_millis(),
                "Skipping focused window frame capture: {}",
                err.message
            );
            None
        }
    }
}

/// Capture text context from the currently focused application.
///
/// Strategy:
/// 1. Try AX API to get selected text + context + cursor position.
/// 2. If no selection, use Smart Selection (expand cursor to word boundaries).
/// 3. If AX API fails entirely, fall back to clipboard simulation.
pub fn capture_context(app_handle: &tauri::AppHandle) -> Result<CapturedContext, String> {
    info!("Starting context capture for correction");

    // Check accessibility permission first
    if !crate::permissions::check_accessibility_permission() {
        error!("Accessibility permission not granted");
        return Err("Accessibility permission required".to_string());
    }
    debug!("Accessibility permission confirmed");

    let element = match get_focused_element() {
        Ok(el) => el,
        Err(e) => {
            warn!("AX API failed to get focused element: {}", e);
            // Try clipboard fallback
            if let Some(text) = capture_via_clipboard(app_handle) {
                let mouse_pos = get_mouse_position();
                return Ok(CapturedContext {
                    selected_text: Some(text.clone()),
                    context: text,
                    cursor_screen_position: mouse_pos,
                });
            }
            return Err(e);
        }
    };

    // Get cursor position for overlay placement
    let cursor_pos = get_cursor_screen_position(element);

    // Try to get selected text directly
    let selected = get_selected_text(element);

    // Get full text + cursor range for context
    let full_text = get_full_text(element);
    let range = get_selected_text_range(element);

    // Release the AX element
    unsafe { CFRelease(element); }

    // Build the context
    let context = if let Some(ref ft) = full_text {
        if let Some(ref r) = range {
            extract_context(ft, r.location as usize, 500)
        } else {
            // No range info, use full text (clamped)
            ft.chars().take(1000).collect()
        }
    } else {
        String::new()
    };

    // If no selected text, try smart selection
    let selected_text = if selected.is_none() || selected.as_ref().map_or(true, |s| s.is_empty()) {
        if let (Some(ref ft), Some(ref r)) = (&full_text, &range) {
            if let Some((_offset, word)) = expand_to_word_boundaries(ft, r.location as usize) {
                debug!(word = word, "Smart-selected word at cursor");
                Some(word)
            } else {
                debug!("No word at cursor for smart selection");
                // Last resort: clipboard fallback
                if let Some(text) = capture_via_clipboard(app_handle) {
                    Some(text)
                } else {
                    None
                }
            }
        } else {
            // No full text or range — clipboard fallback
            if let Some(text) = capture_via_clipboard(app_handle) {
                Some(text)
            } else {
                None
            }
        }
    } else {
        selected
    };

    info!(
        has_selection = selected_text.is_some(),
        context_len = context.len(),
        cursor_x = cursor_pos.0,
        cursor_y = cursor_pos.1,
        "Context capture complete"
    );

    Ok(CapturedContext {
        selected_text,
        context,
        cursor_screen_position: cursor_pos,
    })
}

/// Replace text in the focused application.
///
/// Strategy:
/// 1. Try to re-select the original text via AX API (robust against cursor movement)
/// 2. Stage replacement text on the general pasteboard
/// 3. Simulate Cmd+V to paste
/// 4. Restore the original clipboard only after AppKit no longer needs the staged item
pub fn replace_text_in_app(
    app_handle: &tauri::AppHandle,
    original: &str,
    replacement: &str,
) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_clipboard_manager::ClipboardExt;

    info!(chars = replacement.len(), "Replacing text in focused app");

    // 0. Re-select the original text via AX API to ensure correct replacement
    //    This handles the case where the user clicked elsewhere between
    //    triggering correction and accepting it.
    if let Err(e) = select_text_in_app(app_handle, original) {
        warn!("Could not re-select original text via AX API: {}. Proceeding with current selection.", e);
        // Fall through — the current selection may still be correct
    }

    // 1. Save current clipboard
    let clipboard_backup = {
        let clip = app_handle.clipboard();
        clip.read_text().ok()
    };

    // 2. Stage the replacement and paste it into the current selection.
    let enigo_state = app_handle
        .try_state::<crate::input::EnigoState>()
        .ok_or("EnigoState not available")?;
    {
        let mut guard = enigo_state.0.lock().unwrap();
        if let Some(ref mut enigo) = *guard {
            match crate::macos_transient_clipboard::stage_transient_text(
                replacement,
                clipboard_backup.as_deref(),
            ) {
                Ok(()) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    crate::input::send_paste_ctrl_v(enigo)?;
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "Transient clipboard staging failed for text replacement; falling back to direct typing"
                    );
                    crate::input::paste_text_direct(enigo, replacement)?;
                }
            }
        } else {
            return Err("Enigo not initialized".to_string());
        }
    }

    debug!("Paste command sent for text replacement");

    info!("Text replacement complete");
    Ok(())
}

/// Select text in the focused application by finding it in the element's value
/// and setting the AXSelectedTextRange attribute.
///
/// This allows the correction system to re-select the original text before
/// pasting the replacement, even if the user moved the cursor.
pub fn select_text_in_app(
    _app_handle: &tauri::AppHandle,
    text_to_find: &str,
) -> Result<(), String> {
    select_text_in_app_with_start_policy(text_to_find, SelectionStartPolicy::FirstOccurrence)
}

/// Select the last occurrence of text in the focused application by setting
/// `AXSelectedTextRange`.
///
/// This is used by refine-last to target the most recently inserted transcript
/// text when duplicate phrases exist in the focused element.
pub fn select_text_in_app_last_occurrence(
    _app_handle: &tauri::AppHandle,
    text_to_find: &str,
) -> Result<(), String> {
    select_text_in_app_with_start_policy(text_to_find, SelectionStartPolicy::LastOccurrence)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionStartPolicy {
    FirstOccurrence,
    LastOccurrence,
}

fn select_text_in_app_with_start_policy(
    text_to_find: &str,
    start_policy: SelectionStartPolicy,
) -> Result<(), String> {
    unsafe {
        let element = get_focused_element()?;

        // Get the full text value
        let full_text = match get_full_text(element) {
            Some(ft) => ft,
            None => {
                CFRelease(element);
                return Err("Could not get text value from focused element".to_string());
            }
        };

        // AXSelectedTextRange expects UTF-16 units, so convert byte positions
        // from Rust string search into UTF-16 CFRange offsets.
        let range = match build_selection_range(&full_text, text_to_find, start_policy) {
            Ok(range) => range,
            Err(e) => {
                CFRelease(element);
                return Err(e);
            }
        };

        // Create an AXValue from the range
        let range_value = AXValueCreate(
            K_AX_VALUE_TYPE_CF_RANGE,
            &range as *const CFRange as *const std::ffi::c_void,
        );
        if range_value.is_null() {
            CFRelease(element);
            return Err("Failed to create AXValue for range".to_string());
        }

        // Set the selected text range
        let err = AXUIElementSetAttributeValue(
            element,
            ax_selected_text_range().as_concrete_TypeRef(),
            range_value,
        );

        CFRelease(range_value);
        CFRelease(element);

        if err != K_AX_ERROR_SUCCESS {
            return Err(format!("AXUIElementSetAttributeValue failed: error {}", err));
        }

        // Small delay for the selection to take effect
        std::thread::sleep(std::time::Duration::from_millis(30));

        debug!(
            location_utf16 = range.location,
            len_utf16 = range.length,
            ?start_policy,
            "Re-selected text via AX API"
        );
        Ok(())
    }
}

fn find_selection_start(
    full_text: &str,
    text_to_find: &str,
    start_policy: SelectionStartPolicy,
) -> Option<usize> {
    if text_to_find.is_empty() {
        return None;
    }

    match start_policy {
        SelectionStartPolicy::FirstOccurrence => full_text.find(text_to_find),
        SelectionStartPolicy::LastOccurrence => full_text.rfind(text_to_find),
    }
}

fn build_selection_range(
    full_text: &str,
    text_to_find: &str,
    start_policy: SelectionStartPolicy,
) -> Result<CFRange, String> {
    let start = find_selection_start(full_text, text_to_find, start_policy)
        .ok_or_else(|| format!("Original text '{}' not found in element", text_to_find))?;
    let end = start
        .checked_add(text_to_find.len())
        .ok_or_else(|| "Selection range overflow".to_string())?;

    if !full_text.is_char_boundary(start) || !full_text.is_char_boundary(end) {
        return Err("Selection bounds are not valid UTF-8 boundaries".to_string());
    }

    let location_utf16 = full_text[..start].encode_utf16().count();
    let length_utf16 = text_to_find.encode_utf16().count();
    let location = i64::try_from(location_utf16)
        .map_err(|_| "Selection start exceeds supported AX range".to_string())?;
    let length = i64::try_from(length_utf16)
        .map_err(|_| "Selection length exceeds supported AX range".to_string())?;

    Ok(CFRange { location, length })
}

// ─── Trait helper ───────────────────────────────────────────────────

trait CoercedTo {
    fn coerced_to(self, other: Self) -> Self;
}

impl CoercedTo for i64 {
    fn coerced_to(self, other: Self) -> Self {
        if other > 0 { other } else { self }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_to_word_boundaries_middle() {
        let text = "hello world";
        // cursor at byte 7 = 'o' in "world"
        let result = expand_to_word_boundaries(text, 7);
        assert_eq!(result, Some((6, "world".to_string())));
    }

    #[test]
    fn test_expand_to_word_boundaries_start() {
        let text = "hello world";
        let result = expand_to_word_boundaries(text, 0);
        assert_eq!(result, Some((0, "hello".to_string())));
    }

    #[test]
    fn test_expand_to_word_boundaries_space() {
        let text = "hello world";
        // Cursor at the whitespace boundary selects the preceding word.
        let result = expand_to_word_boundaries(text, 5);
        assert_eq!(result, Some((0, "hello".to_string())));
    }

    #[test]
    fn test_expand_to_word_boundaries_empty() {
        assert_eq!(expand_to_word_boundaries("", 0), None);
    }

    #[test]
    fn test_extract_context() {
        let text = "The quick brown fox jumps over the lazy dog";
        let ctx = extract_context(text, 16, 10);
        // Should capture "brown fox jum" area
        assert!(ctx.contains("fox"));
        assert!(ctx.len() <= 20 + 10); // roughly radius * 2 + some
    }

    #[test]
    fn test_build_insertion_context_start_of_text() {
        let text = "world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 0,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, None);
        assert_eq!(context.left_non_whitespace_char, None);
        assert_eq!(context.right_char, Some('w'));
        assert_eq!(context.right_non_whitespace_char, Some('w'));
        assert!(!context.right_has_line_break_before_non_whitespace);
        assert!(!context.right_has_line_break_before_second_non_whitespace);
        assert!(!context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_middle_of_text() {
        let text = "hello world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 5,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('o'));
        assert_eq!(context.left_non_whitespace_char, Some('o'));
        assert_eq!(context.left_second_non_whitespace_char, Some('l'));
        assert_eq!(context.left_sentence_boundary_char, Some('o'));
        assert_eq!(context.right_char, Some(' '));
        assert_eq!(context.right_non_whitespace_char, Some('w'));
        assert!(!context.right_has_line_break_before_non_whitespace);
        assert!(!context.right_has_line_break_before_second_non_whitespace);
        assert!(!context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_preserves_right_boundary_punctuation() {
        let text = "hello.world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 5,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('o'));
        assert_eq!(context.left_non_whitespace_char, Some('o'));
        assert_eq!(context.left_second_non_whitespace_char, Some('l'));
        assert_eq!(context.left_sentence_boundary_char, Some('o'));
        assert_eq!(context.right_char, Some('.'));
        assert_eq!(context.right_non_whitespace_char, Some('.'));
        assert_eq!(context.right_second_non_whitespace_char, Some('w'));
        assert!(!context.right_has_line_break_before_non_whitespace);
        assert!(!context.right_has_line_break_before_second_non_whitespace);
        assert!(!context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_tracks_second_right_non_whitespace_char() {
        let text = "hello )world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 5,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.right_char, Some(' '));
        assert_eq!(context.right_non_whitespace_char, Some(')'));
        assert_eq!(context.right_second_non_whitespace_char, Some('w'));
        assert!(!context.right_has_line_break_before_non_whitespace);
        assert!(!context.right_has_line_break_before_second_non_whitespace);
    }

    #[test]
    fn test_build_insertion_context_marks_right_line_break_before_non_whitespace() {
        let text = "hello   \nNext";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 5,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.right_char, Some(' '));
        assert_eq!(context.right_non_whitespace_char, Some('N'));
        assert!(context.right_has_line_break_before_non_whitespace);
        assert!(!context.right_has_line_break_before_second_non_whitespace);
    }

    #[test]
    fn test_build_insertion_context_marks_right_line_break_before_second_non_whitespace() {
        let text = "hello)\nNext";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 5,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.right_char, Some(')'));
        assert_eq!(context.right_non_whitespace_char, Some(')'));
        assert_eq!(context.right_second_non_whitespace_char, Some('N'));
        assert!(!context.right_has_line_break_before_non_whitespace);
        assert!(context.right_has_line_break_before_second_non_whitespace);
    }

    #[test]
    fn test_build_insertion_context_tracks_second_left_non_whitespace_char() {
        let text = ". \"world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 3,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('"'));
        assert_eq!(context.left_non_whitespace_char, Some('"'));
        assert_eq!(context.left_second_non_whitespace_char, Some('.'));
        assert_eq!(context.left_sentence_boundary_char, Some('.'));
    }

    #[test]
    fn test_build_insertion_context_tracks_sentence_boundary_char_across_nested_prefixes() {
        let text = ". (\"world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 4,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('"'));
        assert_eq!(context.left_non_whitespace_char, Some('"'));
        assert_eq!(context.left_second_non_whitespace_char, Some('('));
        assert_eq!(context.left_sentence_boundary_char, Some('.'));
    }

    #[test]
    fn test_build_insertion_context_end_of_text() {
        let text = "hello";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 5,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('o'));
        assert_eq!(context.left_non_whitespace_char, Some('o'));
        assert_eq!(context.left_second_non_whitespace_char, Some('l'));
        assert_eq!(context.left_sentence_boundary_char, Some('o'));
        assert_eq!(context.right_char, None);
        assert_eq!(context.right_non_whitespace_char, None);
        assert!(!context.right_has_line_break_before_non_whitespace);
        assert!(!context.right_has_line_break_before_second_non_whitespace);
        assert!(!context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_with_selection() {
        let text = "hello world";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 6,
                length: 5,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some(' '));
        assert_eq!(context.left_non_whitespace_char, Some('o'));
        assert_eq!(context.left_second_non_whitespace_char, Some('l'));
        assert_eq!(context.left_sentence_boundary_char, Some('o'));
        assert_eq!(context.right_char, None);
        assert_eq!(context.right_non_whitespace_char, None);
        assert!(context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_emoji_cursor_uses_utf16_offsets() {
        let text = "a🙂b";
        // UTF-16 offsets: a=1, 🙂=2, b=1. Cursor after emoji is 3.
        let context = build_insertion_context(
            text,
            CFRange {
                location: 3,
                length: 0,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('🙂'));
        assert_eq!(context.left_non_whitespace_char, Some('🙂'));
        assert_eq!(context.left_second_non_whitespace_char, Some('a'));
        assert_eq!(context.left_sentence_boundary_char, Some('🙂'));
        assert_eq!(context.right_char, Some('b'));
        assert_eq!(context.right_non_whitespace_char, Some('b'));
        assert!(!context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_emoji_selection_uses_utf16_length() {
        let text = "a🙂b";
        // Select just the emoji: start at 1, length 2 (UTF-16 code units).
        let context = build_insertion_context(
            text,
            CFRange {
                location: 1,
                length: 2,
            },
        )
        .unwrap();

        assert_eq!(context.left_char, Some('a'));
        assert_eq!(context.left_non_whitespace_char, Some('a'));
        assert_eq!(context.left_second_non_whitespace_char, None);
        assert_eq!(context.left_sentence_boundary_char, Some('a'));
        assert_eq!(context.right_char, Some('b'));
        assert_eq!(context.right_non_whitespace_char, Some('b'));
        assert!(context.has_selection);
    }

    #[test]
    fn test_build_insertion_context_rejects_out_of_bounds_utf16_offsets() {
        let text = "a🙂b";
        let context = build_insertion_context(
            text,
            CFRange {
                location: 999,
                length: 0,
            },
        );
        assert!(context.is_none());
    }

    #[test]
    fn test_find_selection_start_uses_first_occurrence_when_requested() {
        let full_text = "alpha beta alpha";
        assert_eq!(
            find_selection_start(full_text, "alpha", SelectionStartPolicy::FirstOccurrence),
            Some(0)
        );
    }

    #[test]
    fn test_find_selection_start_uses_last_occurrence_when_requested() {
        let full_text = "alpha beta alpha";
        assert_eq!(
            find_selection_start(full_text, "alpha", SelectionStartPolicy::LastOccurrence),
            Some(11)
        );
    }

    #[test]
    fn test_find_selection_start_returns_none_when_missing() {
        let full_text = "alpha beta";
        assert_eq!(
            find_selection_start(full_text, "gamma", SelectionStartPolicy::FirstOccurrence),
            None
        );
    }

    #[test]
    fn test_find_selection_start_rejects_empty_target() {
        let full_text = "alpha beta";
        assert_eq!(
            find_selection_start(full_text, "", SelectionStartPolicy::FirstOccurrence),
            None
        );
    }

    #[test]
    fn test_build_selection_range_ascii_offsets_match_utf16() {
        let range = build_selection_range(
            "alpha beta",
            "beta",
            SelectionStartPolicy::FirstOccurrence,
        )
        .unwrap();
        assert_eq!(range.location, 6);
        assert_eq!(range.length, 4);
    }

    #[test]
    fn test_build_selection_range_emoji_prefix_uses_utf16_location() {
        let range = build_selection_range(
            "🙂 alpha",
            "alpha",
            SelectionStartPolicy::FirstOccurrence,
        )
        .unwrap();
        assert_eq!(range.location, 3);
        assert_eq!(range.length, 5);
    }

    #[test]
    fn test_build_selection_range_emoji_target_uses_utf16_length_for_last_occurrence() {
        let range = build_selection_range("a🙂b🙂", "🙂", SelectionStartPolicy::LastOccurrence)
            .unwrap();
        assert_eq!(range.location, 4);
        assert_eq!(range.length, 2);
    }

    #[test]
    fn test_build_selection_range_emoji_target_uses_utf16_length_for_first_occurrence() {
        let range = build_selection_range("a🙂b🙂", "🙂", SelectionStartPolicy::FirstOccurrence)
            .unwrap();
        assert_eq!(range.location, 1);
        assert_eq!(range.length, 2);
    }

    #[test]
    fn test_build_selection_range_returns_error_when_target_missing() {
        let err =
            build_selection_range("alpha beta", "gamma", SelectionStartPolicy::FirstOccurrence)
                .unwrap_err();
        assert!(err.contains("not found"));
    }
}
