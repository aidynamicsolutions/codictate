//! macOS-specific context capture using Accessibility (AXUIElement) API.
//!
//! This module reads text, selection range, and cursor position from
//! the focused application. Falls back to clipboard simulation when
//! the AX API doesn't return usable data.

use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use tracing::{debug, error, info, warn};

use super::CapturedContext;

// ─── AXUIElement FFI ────────────────────────────────────────────────

// These types mirror Apple's ApplicationServices / HIServices definitions.
// AXUIElementRef is an opaque CFTypeRef.
type AXUIElementRef = CFTypeRef;
type AXError = i32;

const K_AX_ERROR_SUCCESS: AXError = 0;

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
}

// Attribute name constants
fn ax_focused_application() -> CFString {
    CFString::new("AXFocusedApplication")
}
fn ax_focused_ui_element() -> CFString {
    CFString::new("AXFocusedUIElement")
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

// AXValueType constants (from HIServices/AXValue.h)
const K_AX_VALUE_TYPE_CF_RANGE: u32 = 4;
const K_AX_VALUE_TYPE_CG_RECT: u32 = 3;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXValueGetValue(value: CFTypeRef, value_type: u32, out: *mut std::ffi::c_void) -> bool;
    fn AXValueCreate(value_type: u32, data: *const std::ffi::c_void) -> CFTypeRef;
}

// ─── Core functions ─────────────────────────────────────────────────

/// Get the AXUIElement for the focused UI element across all apps.
fn get_focused_element() -> Result<AXUIElementRef, String> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            error!("AXUIElementCreateSystemWide returned null");
            return Err("Failed to create system-wide AX element".to_string());
        }

        // First get the focused app
        let mut focused_app: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            ax_focused_application().as_concrete_TypeRef(),
            &mut focused_app,
        );
        if err != K_AX_ERROR_SUCCESS || focused_app.is_null() {
            CFRelease(system_wide);
            let msg = format!("Failed to get focused application (AXError: {})", err);
            warn!("{}", msg);
            return Err(msg);
        }
        debug!("Got focused application AXUIElement");

        // Then get the focused UI element within that app
        let mut focused_element: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            focused_app,
            ax_focused_ui_element().as_concrete_TypeRef(),
            &mut focused_element,
        );

        CFRelease(focused_app);
        CFRelease(system_wide);

        if err != K_AX_ERROR_SUCCESS || focused_element.is_null() {
            let msg = format!("Failed to get focused UI element (AXError: {})", err);
            warn!("{}", msg);
            return Err(msg);
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
/// Returns (x, y) of the top-left corner of that range's bounding rect.
fn get_bounds_for_range(element: AXUIElementRef, range: CFRange) -> Option<(f64, f64)> {
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
            // Return bottom-left of the range rect (good position for overlay below cursor)
            Some((rect.x, rect.y + rect.height))
        } else {
            warn!("AXValueGetValue failed for CGRect bounds");
            None
        }
    }
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
            use enigo::{Key, Keyboard};
            let _ = enigo.key(Key::Meta, enigo::Direction::Press);
            let _ = enigo.key(Key::Unicode('c'), enigo::Direction::Click);
            let _ = enigo.key(Key::Meta, enigo::Direction::Release);
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
/// 2. Put replacement text on clipboard
/// 3. Simulate Cmd+V to paste
/// 4. Restore original clipboard synchronously
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
    if let Err(e) = select_text_in_app(original) {
        warn!("Could not re-select original text via AX API: {}. Proceeding with current selection.", e);
        // Fall through — the current selection may still be correct
    }

    // 1. Save current clipboard
    let clipboard_backup = {
        let clip = app_handle.clipboard();
        clip.read_text().ok()
    };

    // 2. Put replacement text on clipboard
    {
        let clip = app_handle.clipboard();
        clip.write_text(replacement.to_string())
            .map_err(|e| format!("Failed to write to clipboard: {:?}", e))?;
    }

    // 3. Small delay for clipboard to update
    std::thread::sleep(std::time::Duration::from_millis(50));

    // 4. Simulate Cmd+V to paste
    let enigo_state = app_handle
        .try_state::<crate::input::EnigoState>()
        .ok_or("EnigoState not available")?;
    {
        let mut guard = enigo_state.0.lock().unwrap();
        if let Some(ref mut enigo) = *guard {
            crate::input::send_paste_ctrl_v(enigo)?;
        } else {
            return Err("Enigo not initialized".to_string());
        }
    }

    debug!("Paste command sent for text replacement");

    // 5. Restore clipboard synchronously after paste completes.
    //    This runs on a spawned thread (from fn_key_monitor accept handler),
    //    so blocking here is safe and eliminates the clipboard race window.
    std::thread::sleep(std::time::Duration::from_millis(300));
    if let Some(original_clipboard) = clipboard_backup {
        let clip = app_handle.clipboard();
        let _ = clip.write_text(original_clipboard);
        debug!("Clipboard restored after text replacement");
    }

    info!("Text replacement complete");
    Ok(())
}

/// Select text in the focused application by finding it in the element's value
/// and setting the AXSelectedTextRange attribute.
///
/// This allows the correction system to re-select the original text before
/// pasting the replacement, even if the user moved the cursor.
fn select_text_in_app(text_to_find: &str) -> Result<(), String> {
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

        // Find the text to select
        let Some(pos) = full_text.find(text_to_find) else {
            CFRelease(element);
            return Err(format!("Original text '{}' not found in element", text_to_find));
        };

        // Create the CFRange for the selection
        let range = CFRange {
            location: pos as i64,
            length: text_to_find.len() as i64,
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

        debug!(pos = pos, len = text_to_find.len(), "Re-selected text via AX API");
        Ok(())
    }
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
        // cursor at the space
        let result = expand_to_word_boundaries(text, 5);
        assert_eq!(result, None);
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
}
