//! Audio device information module for detecting device transport types.
//!
//! On macOS, this uses CoreAudio's `kAudioDevicePropertyTransportType` for reliable detection.
//! On other platforms, it falls back to name pattern matching.

use tracing::debug;

#[cfg(target_os = "macos")]
mod ffi {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    extern "C" {
        pub fn is_audio_device_bluetooth(device_name: *const c_char) -> c_int;

    }

    /// Check if a device is Bluetooth using CoreAudio on macOS.
    /// Returns Some(true) if Bluetooth, Some(false) if not, None if device not found.
    pub fn check_bluetooth(device_name: &str) -> Option<bool> {
        let c_name = CString::new(device_name).ok()?;
        let result = unsafe { is_audio_device_bluetooth(c_name.as_ptr()) };
        match result {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        }
    }


}

/// Common Bluetooth device name patterns for fallback detection.
/// These patterns are checked case-insensitively.
const BLUETOOTH_NAME_PATTERNS: &[&str] = &[
    "airpods",
    "beats",
    "bose",
    "sony wh-",
    "sony wf-",
    "jbl",
    "jabra",
    "galaxy buds",
    "samsung",
    "powerbeats",
    "bluetooth",
    "bt headset",
    "bt earphone",
    "wireless",
    "anker",
    "soundcore",
    "skullcandy",
    "sennheiser momentum",
    "audio-technica ath-m",
];

/// Check if a device name matches known Bluetooth patterns (fallback method).
fn is_bluetooth_by_name(device_name: &str) -> bool {
    let lower_name = device_name.to_lowercase();
    for pattern in BLUETOOTH_NAME_PATTERNS {
        if lower_name.contains(pattern) {
            return true;
        }
    }
    false
}

/// Check if an audio device is a Bluetooth device.
///
/// On macOS, this uses CoreAudio's transport type property for reliable detection.
/// On other platforms, it falls back to name pattern matching.
///
/// Returns `true` if the device is detected as Bluetooth, `false` otherwise.
pub fn is_device_bluetooth(device_name: &str) -> bool {
    debug!(
        device = device_name,
        "Checking if audio device is Bluetooth"
    );

    #[cfg(target_os = "macos")]
    {
        // Use CoreAudio for reliable detection
        match ffi::check_bluetooth(device_name) {
            Some(is_bt) => {
                debug!(
                    device = device_name,
                    is_bluetooth = is_bt,
                    method = "core_audio",
                    "CoreAudio Bluetooth check result"
                );
                return is_bt;
            }
            None => {
                debug!(
                    device = device_name,
                    "Device not found in CoreAudio, falling back to name pattern matching"
                );
                // Fall through to pattern matching
            }
        }
    }

    // Fallback: pattern matching by name
    let is_bt = is_bluetooth_by_name(device_name);
    debug!(
        device = device_name,
        is_bluetooth = is_bt,
        method = "name_pattern",
        "Bluetooth detection result (fallback)"
    );
    is_bt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bluetooth_name_patterns() {
        assert!(is_bluetooth_by_name("AirPods Pro"));
        assert!(is_bluetooth_by_name("Beats Studio Buds"));
        assert!(is_bluetooth_by_name("Sony WH-1000XM4"));
        assert!(is_bluetooth_by_name("Bose QuietComfort"));
        assert!(is_bluetooth_by_name("JBL Tune 500BT"));
        assert!(is_bluetooth_by_name("Galaxy Buds Pro"));
        assert!(is_bluetooth_by_name("Bluetooth Headset"));

        // Should NOT match
        assert!(!is_bluetooth_by_name("MacBook Pro Microphone"));
        assert!(!is_bluetooth_by_name("Built-in Microphone"));
        assert!(!is_bluetooth_by_name("USB Audio Device"));
        assert!(!is_bluetooth_by_name("Blue Yeti"));
    }
}
