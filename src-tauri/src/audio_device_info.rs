//! Audio device information module for detecting device transport types.
//!
//! On macOS, this uses CoreAudio's `kAudioDevicePropertyTransportType` for reliable detection.
//! On other platforms, it falls back to name pattern matching.

use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
#[cfg(target_os = "macos")]
use tracing::warn;

static INPUT_ROUTE_MONITOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static AUDIO_TOPOLOGY_EVENT_HANDLER: Lazy<Mutex<Option<Arc<dyn Fn(AudioTopologyEvent) + Send + Sync>>>> =
    Lazy::new(|| Mutex::new(None));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioTopologyEvent {
    RouteChange,
    Wake,
}

impl AudioTopologyEvent {
    #[cfg(test)]
    fn from_raw(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::RouteChange),
            2 => Some(Self::Wake),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::RouteChange => "audio_route_change",
            Self::Wake => "wake",
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MonitorRegistrationState {
    Started,
    AlreadyActive,
}

#[cfg(target_os = "macos")]
mod ffi {
    use super::MonitorRegistrationState;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    extern "C" {
        pub fn is_audio_device_bluetooth(device_name: *const c_char) -> c_int;
        pub fn is_audio_device_builtin(device_name: *const c_char) -> c_int;
        pub fn is_audio_device_virtual(device_name: *const c_char) -> c_int;
        pub fn is_audio_device_continuity_camera(device_name: *const c_char) -> c_int;
        pub fn start_input_route_change_monitor() -> c_int;
        pub fn get_input_route_change_generation() -> u64;
        pub fn start_audio_lifecycle_monitor() -> c_int;
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

    /// Check if a device is Built-in using CoreAudio on macOS.
    /// Returns Some(true) if Built-in, Some(false) if not, None if device not found.
    pub fn check_builtin(device_name: &str) -> Option<bool> {
        let c_name = CString::new(device_name).ok()?;
        let result = unsafe { is_audio_device_builtin(c_name.as_ptr()) };
        match result {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        }
    }

    /// Check if a device is Virtual using CoreAudio on macOS.
    /// Returns Some(true) if Virtual, Some(false) if not, None if device not found.
    pub fn check_virtual(device_name: &str) -> Option<bool> {
        let c_name = CString::new(device_name).ok()?;
        let result = unsafe { is_audio_device_virtual(c_name.as_ptr()) };
        match result {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        }
    }

    /// Check if a device is a Continuity Camera (iPhone mic) using CoreAudio on macOS.
    /// Returns Some(true) if Continuity Camera, Some(false) if not, None if device not found.
    pub fn check_continuity_camera(device_name: &str) -> Option<bool> {
        let c_name = CString::new(device_name).ok()?;
        let result = unsafe { is_audio_device_continuity_camera(c_name.as_ptr()) };
        match result {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        }
    }

    pub fn start_route_monitor() -> Result<MonitorRegistrationState, String> {
        let result = unsafe { start_input_route_change_monitor() };
        match result {
            0 => Ok(MonitorRegistrationState::Started),
            1 => Ok(MonitorRegistrationState::AlreadyActive),
            other => Err(format!(
                "start_input_route_change_monitor failed with status {other}"
            )),
        }
    }

    pub fn route_change_generation() -> u64 {
        unsafe { get_input_route_change_generation() }
    }

    pub fn start_lifecycle_monitor() -> Result<MonitorRegistrationState, String> {
        let result = unsafe { start_audio_lifecycle_monitor() };
        match result {
            0 => Ok(MonitorRegistrationState::Started),
            1 => Ok(MonitorRegistrationState::AlreadyActive),
            other => Err(format!(
                "start_audio_lifecycle_monitor failed with status {other}"
            )),
        }
    }
}

#[cfg(target_os = "macos")]
fn log_audio_topology_bridge_event(bridge_source: &str, outcome: &str, reason: Option<&str>) {
    info!(
        bridge_source = bridge_source,
        outcome = outcome,
        reason = reason.unwrap_or(""),
        event_code = "audio_topology_event_bridge",
        "Audio topology bridge state updated"
    );
}

fn dispatch_audio_topology_event(event: AudioTopologyEvent) {
    log_audio_topology_bridge_event(event.as_str(), "event_received", None);
    let handler = AUDIO_TOPOLOGY_EVENT_HANDLER
        .lock()
        .unwrap()
        .as_ref()
        .cloned();
    if let Some(handler) = handler {
        handler(event);
    } else {
        warn!(
            bridge_source = event.as_str(),
            outcome = "dropped_no_handler",
            reason = "handler_not_registered",
            event_code = "audio_topology_event_bridge",
            "Dropped audio topology event because no Rust handler is registered"
        );
    }
}

#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub extern "C" fn notify_audio_topology_route_change() {
    dispatch_audio_topology_event(AudioTopologyEvent::RouteChange);
}

#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub extern "C" fn notify_audio_topology_wake() {
    dispatch_audio_topology_event(AudioTopologyEvent::Wake);
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

pub fn start_input_route_change_monitor() {
    #[cfg(target_os = "macos")]
    {
        match ffi::start_route_monitor() {
            Ok(MonitorRegistrationState::Started) => {
                INPUT_ROUTE_MONITOR_ACTIVE.store(true, Ordering::SeqCst);
                log_audio_topology_bridge_event("route_monitor", "started", None);
            }
            Ok(MonitorRegistrationState::AlreadyActive) => {
                INPUT_ROUTE_MONITOR_ACTIVE.store(true, Ordering::SeqCst);
                log_audio_topology_bridge_event("route_monitor", "already_active", None);
            }
            Err(err) => {
                INPUT_ROUTE_MONITOR_ACTIVE.store(false, Ordering::SeqCst);
                warn!(
                    bridge_source = "route_monitor",
                    outcome = "registration_failed",
                    reason = err.as_str(),
                    event_code = "audio_topology_event_bridge",
                    error = err,
                    "Failed to start input route change monitor; default-route startup will fall back to fresh enumeration"
                );
            }
        }
    }
}

pub fn register_audio_topology_event_handler(
    handler: Arc<dyn Fn(AudioTopologyEvent) + Send + Sync>,
) {
    *AUDIO_TOPOLOGY_EVENT_HANDLER.lock().unwrap() = Some(handler);
    #[cfg(target_os = "macos")]
    log_audio_topology_bridge_event("rust_handler", "registered", None);
}

#[cfg(target_os = "macos")]
pub fn start_audio_lifecycle_monitor() -> Result<(), String> {
    match ffi::start_lifecycle_monitor() {
        Ok(MonitorRegistrationState::Started) => {
            log_audio_topology_bridge_event("lifecycle_monitor", "started", None);
            Ok(())
        }
        Ok(MonitorRegistrationState::AlreadyActive) => {
            log_audio_topology_bridge_event("lifecycle_monitor", "already_active", None);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub fn is_input_route_change_monitor_active() -> bool {
    #[cfg(target_os = "macos")]
    {
        return INPUT_ROUTE_MONITOR_ACTIVE.load(Ordering::SeqCst);
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn input_route_change_generation() -> u64 {
    #[cfg(target_os = "macos")]
    {
        if !is_input_route_change_monitor_active() {
            return 0;
        }
        return ffi::route_change_generation();
    }

    #[cfg(not(target_os = "macos"))]
    {
        0
    }
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

#[cfg(all(test, target_os = "macos"))]
mod macos_bridge_tests {
    use super::{
        dispatch_audio_topology_event, register_audio_topology_event_handler, AudioTopologyEvent,
    };
    use once_cell::sync::Lazy;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    static TEST_HANDLER_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn topology_event_from_raw_maps_known_codes() {
        let _guard = TEST_HANDLER_LOCK.lock().unwrap();
        assert_eq!(
            AudioTopologyEvent::from_raw(1),
            Some(AudioTopologyEvent::RouteChange)
        );
        assert_eq!(AudioTopologyEvent::from_raw(2), Some(AudioTopologyEvent::Wake));
        assert_eq!(AudioTopologyEvent::from_raw(99), None);
    }

    #[test]
    fn topology_event_dispatch_invokes_registered_handler() {
        let _guard = TEST_HANDLER_LOCK.lock().unwrap();
        let call_count = Arc::new(AtomicUsize::new(0));
        let received = Arc::new(Mutex::new(Vec::new()));
        register_audio_topology_event_handler(Arc::new({
            let call_count = Arc::clone(&call_count);
            let received = Arc::clone(&received);
            move |event| {
                call_count.fetch_add(1, Ordering::SeqCst);
                received.lock().unwrap().push(event);
            }
        }));

        dispatch_audio_topology_event(AudioTopologyEvent::Wake);

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert_eq!(*received.lock().unwrap(), vec![AudioTopologyEvent::Wake]);
    }

    #[test]
    fn topology_event_dispatch_without_handler_is_a_noop() {
        let _guard = TEST_HANDLER_LOCK.lock().unwrap();
        *super::AUDIO_TOPOLOGY_EVENT_HANDLER.lock().unwrap() = None;
        dispatch_audio_topology_event(AudioTopologyEvent::RouteChange);
        assert!(super::AUDIO_TOPOLOGY_EVENT_HANDLER
            .lock()
            .unwrap()
            .is_none());
    }
}

/// Check if an audio device is a Built-in device.
///
/// On macOS, this uses CoreAudio's transport type property for reliable detection.
/// On other platforms, this always returns false (or could use name matching if needed).
pub fn is_device_builtin(device_name: &str) -> bool {
    debug!(
        device = device_name,
        "Checking if audio device is Built-in"
    );

    #[cfg(target_os = "macos")]
    {
        // Use CoreAudio for reliable detection
        match ffi::check_builtin(device_name) {
            Some(is_builtin) => {
                debug!(
                    device = device_name,
                    is_builtin = is_builtin,
                    method = "core_audio",
                    "CoreAudio Built-in check result"
                );
                return is_builtin;
            }
            None => {
                debug!(
                    device = device_name,
                    "Device not found in CoreAudio"
                );
                return false;
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fallback for other OSs - simplistic check or always false
        // For now, assume false or maybe check for "Built-in" in name?
        // But the user specifically asked for macOS safety.
        false
    }
}

/// Check if an audio device is a Virtual/Phantom device.
///
/// On macOS, this uses CoreAudio's transport type property.
pub fn is_device_virtual(device_name: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        match ffi::check_virtual(device_name) {
            Some(is_virt) => return is_virt,
            None => return false,
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    false
}

/// Check if an audio device is a Continuity Camera (iPhone microphone).
///
/// On macOS, this uses CoreAudio's transport type property to detect
/// `kAudioDeviceTransportTypeContinuityCaptureWired` and `*Wireless`.
/// Continuity Camera microphones are unreliable for speech-to-text.
pub fn is_device_continuity_camera(device_name: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        match ffi::check_continuity_camera(device_name) {
            Some(is_cc) => {
                if is_cc {
                    debug!(
                        device = device_name,
                        "Device is a Continuity Camera (iPhone) microphone - excluding from list"
                    );
                }
                return is_cc;
            }
            None => return false,
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    false
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
