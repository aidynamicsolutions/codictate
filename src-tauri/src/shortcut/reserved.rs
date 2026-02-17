
use std::collections::HashMap;
use std::sync::OnceLock;

/// Struct to hold information about a reserved shortcut
pub struct ReservedShortcut {
    pub translation_key: &'static str,
}

/// Static map of reserved shortcuts
/// Key: Lowercase shortcut string (e.g., "command+space")
/// Value: Reserved information including translation key
static RESERVED_SHORTCUTS: OnceLock<HashMap<&'static str, ReservedShortcut>> = OnceLock::new();

fn get_reserved_shortcuts() -> &'static HashMap<&'static str, ReservedShortcut> {
    RESERVED_SHORTCUTS.get_or_init(|| {
        let mut m = HashMap::new();
        
        // Helper to insert shortcuts easily
        fn insert(m: &mut HashMap<&'static str, ReservedShortcut>, keys: &[&'static str], translation_key: &'static str) {
            for key in keys {
                m.insert(key, ReservedShortcut { translation_key });
            }
        }

        // --- macOS Specific Shortcuts ---
        #[cfg(target_os = "macos")]
        {
            // Common Operations (System-wide app control)
            insert(&mut m, &[
                "command+q", "cmd+q",
                "command+h", "cmd+h",
                "command+option+h", "cmd+option+h", "command+alt+h", "cmd+alt+h",
                "command+m", "cmd+m",
                "command+option+m", "cmd+option+m", "command+alt+m", "cmd+alt+m",
                "command+w", "cmd+w",
                "command+option+w", "cmd+option+w", "command+alt+w", "cmd+alt+w",
                "command+comma", "cmd+comma", "command+,", "cmd+,",
            ], "shortcuts.reserved.app_control"); // "Reserved for Application Control (Quit, Hide, Minimize, Preferences)"

            // Navigation & Search (Spotlight - HIGH IMPACT)
            insert(&mut m, &[
                "command+space", "cmd+space",
            ], "shortcuts.reserved.spotlight"); // "Reserved for Spotlight Search"

            insert(&mut m, &[
                "command+option+space", "cmd+option+space", "command+alt+space", "cmd+alt+space",
            ], "shortcuts.reserved.spotlight_finder"); // "Reserved for Spotlight Finder Search"

            insert(&mut m, &[
                "command+tab", "cmd+tab",
                "command+shift+tab", "cmd+shift+tab", // Reverse cycle
                "command+`", "cmd+`", // Cycle windows
            ], "shortcuts.reserved.app_switching"); // "Reserved for App Switching"

            // Finder-Specific / System-Global
            insert(&mut m, &[
                "command+option+d", "cmd+option+d", "command+alt+d", "cmd+alt+d", // Dock
                "command+option+esc", "cmd+option+esc", "command+alt+esc", "cmd+alt+esc", // Force Quit
                "control+command+q", "ctrl+cmd+q", // Lock Screen
                "shift+command+q", "shift+cmd+q", // Log Out
                "option+shift+command+q", "opt+shift+cmd+q", "alt+shift+cmd+q", // Immediate Log Out
            ], "shortcuts.reserved.system_ui"); // "Reserved for System UI (Dock, Force Quit, Log Out)"

            // Input Source (Critical conflict)
            insert(&mut m, &[
                "control+space", "ctrl+space",
            ], "shortcuts.reserved.input_source_prev"); // "Reserved for Input Source Switching (Previous Source)"

            insert(&mut m, &[
                "control+option+space", "ctrl+option+space", "control+alt+space", "ctrl+alt+space",
            ], "shortcuts.reserved.input_source_next"); // "Reserved for Input Source Switching (Next Source)"

            // Mission Control & Spaces
            insert(&mut m, &[
                "control+up", "ctrl+up",
                "control+down", "ctrl+down",
                "control+left", "ctrl+left",
                "control+right", "ctrl+right",
                "control+1", "ctrl+1", "control+2", "ctrl+2", // Desktop switching (simplified list)
            ], "shortcuts.reserved.mission_control"); // "Reserved for Mission Control & Desktop Navigation"

            // System Navigation
            insert(&mut m, &[
                "control+f2", "ctrl+f2",
                "control+f3", "ctrl+f3",
                "control+f4", "ctrl+f4",
                "control+f5", "ctrl+f5",
                "control+f8", "ctrl+f8",
            ], "shortcuts.reserved.system_focus"); // "Reserved for System Focus Navigation (Check 'Keyboard Navigation' in Settings)"

            // Accessibility
            insert(&mut m, &[
                "option+command+8", "opt+cmd+8", "alt+cmd+8", // Zoom
                "command+f5", "cmd+f5", // VoiceOver
                "option+command+f5", "opt+cmd+f5", "alt+cmd+f5", // Accessibility Panel
                "control+option+command+8", "ctrl+opt+cmd+8", "ctrl+alt+cmd+8", // Invert Colors
            ], "shortcuts.reserved.accessibility"); // "Reserved for Accessibility Features"

            // Screen Capture
            insert(&mut m, &[
                "shift+command+3", "shift+cmd+3",
                "shift+command+4", "shift+cmd+4",
                "shift+command+5", "shift+cmd+5",
                "shift+command+6", "shift+cmd+6",
            ], "shortcuts.reserved.screenshots"); // "Reserved for Screenshots"
            
            // Common Editing - Critical for text input
            insert(&mut m, &[
                "command+c", "cmd+c",
                "command+v", "cmd+v",
                "command+x", "cmd+x",
                "command+z", "cmd+z",
                "shift+command+z", "shift+cmd+z",
                "command+a", "cmd+a",
            ], "shortcuts.reserved.common_editing"); // "Reserved for Common Editing (Copy, Paste, Cut, Undo, Select All)"

            // Common App/File Operations
            insert(&mut m, &[
                "command+s", "cmd+s",
                "command+n", "cmd+n",
                "command+o", "cmd+o",
                "command+p", "cmd+p",
            ], "shortcuts.reserved.common_file"); // "Reserved for Common File Operations (Save, New, Open, Print)"
            
            // Power & System Security
            insert(&mut m, &[
                "control+power", "ctrl+power",
                "control+shift+power", "ctrl+shift+power",
                "control+command+power", "ctrl+cmd+power",
            ], "shortcuts.reserved.power"); // "Reserved for Power & Security"
        }

        // --- Windows Specific Shortcuts ---
        #[cfg(target_os = "windows")]
        {
            insert(&mut m, &[
                "super+l", "win+l", // Lock screen
                "super+d", "win+d", // Show desktop
                "super+e", "win+e", // File Explorer
                "super+r", "win+r", // Run dialog
                "super+tab", "win+tab", // Task View
                "alt+tab", // App switcher
                "alt+f4", // Close app
                "ctrl+alt+delete", "ctrl+alt+del", // Security screen
            ], "shortcuts.reserved.windows_system"); // "Reserved for Windows System"

            // Common Editing
            insert(&mut m, &[
                "ctrl+c", "control+c",
                "ctrl+v", "control+v",
                "ctrl+x", "control+x",
                "ctrl+z", "control+z",
                "ctrl+y", "control+y",
                "ctrl+a", "control+a",
            ], "shortcuts.reserved.common_editing"); // "Reserved for Common Editing (Copy, Paste, Cut, Undo, Redo, Select All)"

             // Common File
            insert(&mut m, &[
                "ctrl+s", "control+s",
                "ctrl+n", "control+n",
                "ctrl+o", "control+o",
                "ctrl+p", "control+p",
                "ctrl+w", "control+w",
            ], "shortcuts.reserved.common_file"); // "Reserved for Common File Operations"
        }

        // --- Linux Specific Shortcuts ---
        #[cfg(target_os = "linux")]
        {
            insert(&mut m, &[
                "alt+tab", // App switcher
                "alt+f4", // Close app
                "super+l", // Lock screen (common in GNOME/KDE)
                "super+d", // Show desktop
            ], "shortcuts.reserved.linux_system"); // "Reserved for Linux System"

            // Common Editing
            insert(&mut m, &[
                "ctrl+c", "control+c",
                "ctrl+v", "control+v",
                "ctrl+x", "control+x",
                "ctrl+z", "control+z",
                "ctrl+y", "control+y",
                "ctrl+a", "control+a",
            ], "shortcuts.reserved.common_editing"); // "Reserved for Common Editing"
             
             // Common File
            insert(&mut m, &[
                "ctrl+s", "control+s",
                "ctrl+n", "control+n",
                "ctrl+o", "control+o",
                "ctrl+p", "control+p",
            ], "shortcuts.reserved.common_file"); // "Reserved for Common File Operations"
        }

        // --- Function (Fn) Shortcuts ---
        // These are blocked on all platforms if the user tries to register them naturally, 
        // though they are mostly relevant to macOS Fn key behavior.
        // On macOS "fn" key is special. On others it might not even register.
        // We include them generally to avoid confusion.
        insert(&mut m, &[
            "fn+a", "fn+c", "fn+d", "fn+e", "fn+f", "fn+h", "fn+m", "fn+n", "fn+q",
            "fn+left", "fn+right", "fn+up", "fn+down", "fn+delete",
        ], "shortcuts.reserved.fn_system"); // "Reserved Fn System Shortcut"

        m
    })
}

/// Check if a given shortcut string is reserved by macOS.
/// Returns Ok(()) if not reserved, or Err(String) with the reason if it is.
/// 
/// The validation matches against normalized lowercase versions of the shortcut.
pub fn check_reserved_shortcut(shortcut: &str) -> Result<(), String> {
    // Normalize the shortcut string for comparison
    // 1. Lowercase
    // 2. Remove whitespace
    let normalized = shortcut.to_lowercase().replace(' ', "");
    
    // Check exact match first
    if let Some(reserved) = get_reserved_shortcuts().get(normalized.as_str()) {
        return Err(format!("RESERVED:{}", reserved.translation_key));
    }

    // Check strict component matching (order independent)
    // e.g. "space+ctrl" should match "ctrl+space"
    let parts: Vec<&str> = normalized.split('+').collect();
    if parts.len() > 1 {
        // We iterate through all reserved shortcuts and check if one has the exact same components
        for (key, reserved) in get_reserved_shortcuts() {
            let reserved_parts: Vec<&str> = key.split('+').collect();
            if parts.len() == reserved_parts.len() {
                // Check if all parts match (regardless of order)
                let all_match = parts.iter().all(|p| reserved_parts.contains(p));
                if all_match {
                    return Err(format!("RESERVED:{}", reserved.translation_key));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_reserved_shortcut;

    #[test]
    #[cfg(target_os = "macos")]
    fn undo_default_binding_is_not_reserved() {
        assert!(check_reserved_shortcut("control+command+z").is_ok());
    }

    #[test]
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn undo_default_binding_is_not_reserved() {
        assert!(check_reserved_shortcut("ctrl+alt+z").is_ok());
    }
}
