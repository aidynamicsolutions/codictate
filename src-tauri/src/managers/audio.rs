use crate::audio_toolkit::audio::{list_input_devices, AudioRecorder};
use crate::audio_toolkit::vad::SmoothedVad;
use crate::audio_toolkit::SileroVad;
use crate::helpers::clamshell;
use crate::overlay;
use crate::settings::{get_settings, AppSettings};
use crate::utils;
use anyhow::Result;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Listener, Manager};
use tracing::{debug, error, info, warn};
use tauri_plugin_notification::NotificationExt;

fn set_mute(mute: bool) {
    // Expected behavior:
    // - Windows: works on most systems using standard audio drivers.
    // - Linux: works on many systems (PipeWire, PulseAudio, ALSA),
    //   but some distros may lack the tools used.
    // - macOS: works on most standard setups via AppleScript.
    // If unsupported, fails silently.

    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows::Win32::{
                Media::Audio::{
                    eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
                    MMDeviceEnumerator,
                },
                System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
            };

            macro_rules! unwrap_or_return {
                ($expr:expr) => {
                    match $expr {
                        Ok(val) => val,
                        Err(_) => return,
                    }
                };
            }

            // Initialize the COM library for this thread.
            // If already initialized (e.g., by another library like Tauri), this does nothing.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let all_devices: IMMDeviceEnumerator =
                unwrap_or_return!(CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL));
            let default_device =
                unwrap_or_return!(all_devices.GetDefaultAudioEndpoint(eRender, eMultimedia));
            let volume_interface = unwrap_or_return!(
                default_device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            );

            let _ = volume_interface.SetMute(mute, std::ptr::null());
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let mute_val = if mute { "1" } else { "0" };
        let amixer_state = if mute { "mute" } else { "unmute" };

        // Try multiple backends to increase compatibility
        // 1. PipeWire (wpctl)
        if Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", mute_val])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return;
        }

        // 2. PulseAudio (pactl)
        if Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", mute_val])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return;
        }

        // 3. ALSA (amixer)
        let _ = Command::new("amixer")
            .args(["set", "Master", amixer_state])
            .output();
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let script = format!(
            "set volume output muted {}",
            if mute { "true" } else { "false" }
        );
        let _ = Command::new("osascript").args(["-e", &script]).output();
    }
}

const WHISPER_SAMPLE_RATE: usize = 16000;

/// Grace period after failover during which 0-sample detection is suppressed.
/// This prevents false positives when the new device is still initializing.
const FAILOVER_GRACE_PERIOD_SECS: u64 = 10;

/* ──────────────────────────────────────────────────────────────── */

#[derive(Clone, Debug)]
pub enum RecordingState {
    Idle,
    Preparing { binding_id: String },
    Recording { binding_id: String, session_id: String },
}

#[derive(Clone, Debug)]
pub enum MicrophoneMode {
    AlwaysOn,
    OnDemand,
}

/* ──────────────────────────────────────────────────────────────── */

fn create_audio_recorder(
    vad_path: &str,
    app_handle: &tauri::AppHandle,
    zero_level_count: Arc<Mutex<u32>>,
    failover_timestamp: Arc<Mutex<Option<Instant>>>,
    current_device_name: Arc<Mutex<Option<String>>>,
) -> Result<AudioRecorder, anyhow::Error> {
    let silero = SileroVad::new(vad_path, 0.3)
        .map_err(|e| anyhow::anyhow!("Failed to create SileroVad: {}", e))?;
    let smoothed_vad = SmoothedVad::new(Box::new(silero), 15, 15, 2);

    // Recorder with VAD plus a spectrum-level callback that forwards updates to
    // the frontend and monitors for dead devices.
    let recorder = AudioRecorder::new()
        .map_err(|e| anyhow::anyhow!("Failed to create AudioRecorder: {}", e))?
        .with_vad(Box::new(smoothed_vad))
        .with_level_callback({
            let app_handle = app_handle.clone();
            let current_device_name = current_device_name.clone();
            move |levels| {
                utils::emit_levels(&app_handle, &levels);
                
                // Dead device detection: check if all levels are zero
                // Levels at ~30ms intervals, 50 readings = ~1.5s of sustained zeros
                const ZERO_THRESHOLD: f32 = 0.001;
                const DEAD_DEVICE_READINGS: u32 = 50;
                
                let all_zero = levels.iter().all(|&l| l < ZERO_THRESHOLD);
                
                let mut count = zero_level_count.lock().unwrap();
                if all_zero {
                    *count += 1;
                    
                    if *count >= DEAD_DEVICE_READINGS {
                        // Check if we already triggered failover (timestamp is Some)
                        let mut triggered = failover_timestamp.lock().unwrap();
                        if triggered.is_none() {
                            *triggered = Some(Instant::now());
                            // Get the device name at the time of detection to include in event
                            let dead_device_name = current_device_name.lock().unwrap().clone()
                                .unwrap_or_else(|| "unknown".to_string());
                            warn!("Dead device detected: {} consecutive zero-level readings on '{}'. Triggering failover.", *count, dead_device_name);
                            let _ = app_handle.emit("audio-device-dead", serde_json::json!({
                                "reason": "sustained_zero_levels",
                                "count": *count,
                                "device": dead_device_name
                            }));
                        }
                    }
                } else {
                    // Reset counter if we get non-zero audio
                    if *count > 0 {
                        debug!("Audio signal detected, resetting zero-level counter");
                        *count = 0;
                        // NOTE: Do NOT reset failover_timestamp here!
                        // The timestamp expires naturally after FAILOVER_GRACE_PERIOD_SECS.
                        // Resetting it here was the bug - it removed protection before
                        // the 0-sample check in stop_recording() could use it.
                    }
                }
            }
        });

    Ok(recorder)
}

/* ──────────────────────────────────────────────────────────────── */

#[derive(Clone)]
pub struct AudioRecordingManager {
    state: Arc<Mutex<RecordingState>>,
    mode: Arc<Mutex<MicrophoneMode>>,
    app_handle: tauri::AppHandle,

    recorder: Arc<Mutex<Option<AudioRecorder>>>,
    is_open: Arc<Mutex<bool>>,
    is_recording: Arc<Mutex<bool>>,
    did_mute: Arc<Mutex<bool>>,
    /// Tracks if we've ever successfully started a recording (for first-trigger detection)
    has_recorded_before: Arc<Mutex<bool>>,
    
    /// Recording start time for tracking elapsed time
    recording_start_time: Arc<Mutex<Option<Instant>>>,
    /// Channel to stop the time tracking timer
    timer_stop_tx: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    
    /// Counter for consecutive zero-level readings (dead device detection)
    zero_level_count: Arc<Mutex<u32>>,
    /// Timestamp of last failover (for grace period protection against false 0-sample detection)
    failover_timestamp: Arc<Mutex<Option<Instant>>>,
    /// Blocklist of dead devices (prevents switching back to them)
    blocked_devices: Arc<Mutex<std::collections::HashSet<String>>>,
    /// The name of the device currently opened in the stream (for accurate failover)
    current_device_name: Arc<Mutex<Option<String>>>,
}

impl AudioRecordingManager {
    /* ---------- construction ------------------------------------------------ */

    pub fn new(app: &tauri::AppHandle) -> Result<Self, anyhow::Error> {
        let settings = get_settings(app);
        let mode = if settings.always_on_microphone {
            MicrophoneMode::AlwaysOn
        } else {
            MicrophoneMode::OnDemand
        };

        let manager = Self {
            state: Arc::new(Mutex::new(RecordingState::Idle)),
            mode: Arc::new(Mutex::new(mode.clone())),
            app_handle: app.clone(),

            recorder: Arc::new(Mutex::new(None)),
            is_open: Arc::new(Mutex::new(false)),
            is_recording: Arc::new(Mutex::new(false)),
            did_mute: Arc::new(Mutex::new(false)),
            has_recorded_before: Arc::new(Mutex::new(false)),
            recording_start_time: Arc::new(Mutex::new(None)),
            timer_stop_tx: Arc::new(Mutex::new(None)),
            zero_level_count: Arc::new(Mutex::new(0)),
            failover_timestamp: Arc::new(Mutex::new(None)),
            blocked_devices: Arc::new(Mutex::new(std::collections::HashSet::new())),
            current_device_name: Arc::new(Mutex::new(None)),
        };

        // Always-on?  Open immediately.
        if matches!(mode, MicrophoneMode::AlwaysOn) {
            manager.start_microphone_stream()?;
        }

        // Setup dead device event listener
        manager.setup_dead_device_listener();

        Ok(manager)
    }
    
    /// Setup listener for dead device detection events.
    /// When 'audio-device-dead' is emitted, this triggers immediate failover.
    fn setup_dead_device_listener(&self) {
        let manager = self.clone();
        self.app_handle.listen("audio-device-dead", move |event| {
            // Extract device name from event payload to verify this event is for current device
            let event_device: Option<String> = serde_json::from_str::<serde_json::Value>(event.payload())
                .ok()
                .and_then(|v| v.get("device").and_then(|d| d.as_str().map(|s| s.to_string())));
            
            // Spawn a thread to handle failover (avoid blocking event loop)
            let manager = manager.clone();
            std::thread::spawn(move || {
                // Get the CURRENT device name to compare with event
                let current_device = manager.current_device_name.lock().unwrap().clone();
                
                // CRITICAL: Only process if event is for the current device.
                // This prevents stale events from old device from triggering failover on new device.
                if let Some(ref event_dev) = event_device {
                    if let Some(ref curr_dev) = current_device {
                        if event_dev != curr_dev {
                            debug!("Ignoring stale audio-device-dead event: event for '{}' but current device is '{}'", event_dev, curr_dev);
                            return;
                        }
                    }
                }
                
                let failed_device_name = current_device.unwrap_or_else(|| "Default".to_string());
                
                info!("Received audio-device-dead event for '{}', triggering failover", failed_device_name);
                
                // Add the failed device to the blocklist to prevent switching back
                {
                    let mut blocked = manager.blocked_devices.lock().unwrap();
                    if blocked.insert(failed_device_name.clone()) {
                        info!("Added '{}' to dead device blocklist", failed_device_name);
                    }
                }
                
                // Enumerate devices and find fallback
                if let Ok(devices) = list_input_devices() {
                    if let Some((fallback_name, _fallback_device)) = 
                        manager.find_fallback_device_from_list(&failed_device_name, devices) 
                    {
                        info!("Immediate failover: Switching from {} to {}", failed_device_name, fallback_name);
                        
                        // IMPORTANT: Remove the fallback device from blocklist if it was there
                        // (from a previous failover session). This ensures it can be used.
                        {
                            let mut blocked = manager.blocked_devices.lock().unwrap();
                            if blocked.remove(&fallback_name) {
                                info!("Removed '{}' from blocklist (now active as fallback)", fallback_name);
                            }
                        }
                        
                        // CRITICAL: Update current_device_name BEFORE stopping old stream
                        // This ensures any dead-device events that arrive during transition
                        // are correctly identified as stale (for old device) and filtered out.
                        *manager.current_device_name.lock().unwrap() = Some(fallback_name.clone());
                        
                        // Update settings with new device
                        let mut settings = get_settings(&manager.app_handle);
                        settings.selected_microphone = Some(fallback_name.clone());
                        crate::settings::write_settings(&manager.app_handle, settings);
                        
                        // Stop and restart mic stream with new device
                        manager.stop_microphone_stream();
                        
                        // CRITICAL: Set failover timestamp FIRST to give the 0-sample check a grace period,
                        // then reset the counter. This prevents false positives during device transition.
                        *manager.failover_timestamp.lock().unwrap() = Some(Instant::now());
                        manager.reset_zero_level_counter_only();
                        
                        if let Err(e) = manager.start_microphone_stream() {
                            error!("Failed to restart mic stream after failover: {}", e);
                        }
                        
                        // Notify frontend and system AFTER settings are written
                        info!("Emitting audio-device-auto-switched event: {} -> {}", failed_device_name, fallback_name);
                        let _ = manager.app_handle.emit("audio-device-auto-switched", serde_json::json!({
                            "previous": failed_device_name,
                            "current": fallback_name
                        }));
                        
                        let _ = manager.app_handle.notification().builder()
                            .title("Microphone Changed")
                            .body(&format!("Switched to {} because {} was unavailable.", fallback_name, failed_device_name))
                            .show();
                    } else {
                        warn!("No fallback device found during immediate failover");
                    }
                }
            });
        });
    }
    
    /* ---------- blocklist management --------------------------------------- */
    
    /// Get a copy of the blocked devices set.
    pub fn get_blocked_devices(&self) -> std::collections::HashSet<String> {
        self.blocked_devices.lock().unwrap().clone()
    }
    
    /// Replace the blocked devices set.
    pub fn set_blocked_devices(&self, devices: std::collections::HashSet<String>) {
        *self.blocked_devices.lock().unwrap() = devices;
    }
    
    /// Reset dead device detection counters (used when user explicitly re-selects a device).
    pub fn reset_dead_device_counters(&self) {
        *self.zero_level_count.lock().unwrap() = 0;
        *self.failover_timestamp.lock().unwrap() = None;
    }
    
    /// Reset ONLY the zero-level counter (used during failover).
    /// IMPORTANT: This does NOT reset failover_timestamp, allowing the 0-sample check
    /// to correctly identify that a failover just occurred and skip blocklist addition.
    pub fn reset_zero_level_counter_only(&self) {
        *self.zero_level_count.lock().unwrap() = 0;
        // Note: Do NOT reset failover_timestamp here - it acts as a grace period
        // to prevent false positives immediately after switching devices.
    }

    /* ---------- helper methods --------------------------------------------- */

    /// Get the effective microphone device from a pre-fetched device list.
    /// This avoids calling list_input_devices() which is slow during failover.
    fn get_effective_device_from_list(
        &self,
        settings: &AppSettings,
        devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>,
    ) -> Option<cpal::Device> {
        // Check if we're in clamshell mode and have a clamshell microphone configured
        let use_clamshell_mic = if let Ok(is_clamshell) = clamshell::is_clamshell() {
            is_clamshell && settings.clamshell_microphone.is_some()
        } else {
            false
        };

        if use_clamshell_mic {
            let device_name = settings.clamshell_microphone.as_ref().unwrap();
            return devices
                .into_iter()
                .find(|d| d.name == *device_name)
                .map(|d| d.device);
        }

        // Logic for handling standard selection vs Default
        if let Some(name) = &settings.selected_microphone {
            // Check if this device is in the blocklist (previously detected as dead)
            let blocked = self.blocked_devices.lock().unwrap();
            if blocked.contains(name) {
                info!("Selected microphone '{}' is in blocklist (dead device). Finding fallback...", name);
                drop(blocked);
                // Find a non-blocked fallback
                return self.find_fallback_device_from_list(name, devices).map(|(_, d)| d);
            }
            drop(blocked);
            
            // User explicitly selected a microphone -> Use it strictly
            return devices
                .into_iter()
                .find(|d| d.name == *name)
                .map(|d| d.device);
        }
        
        // "Default" is selected (None in settings)
        // Safety Check: If the system default is Bluetooth, try to fallback to an Internal Mic
        // to prevent low-quality audio.
        
        // Find the system default device
        if let Some(default_dev) = devices.iter().find(|d| d.is_default) {
            let is_bt = crate::audio_device_info::is_device_bluetooth(&default_dev.name);
            
            if is_bt {
                info!("System default microphone '{}' is Bluetooth. Searching for Built-in fallback...", default_dev.name);
                
                // Search for a verified built-in microphone
                let builtin_mic = devices.iter().find(|d| {
                    crate::audio_device_info::is_device_builtin(&d.name)
                });
                
                if let Some(builtin) = builtin_mic {
                    info!("Found Built-in fallback microphone: '{}'. Using it instead of Bluetooth default.", builtin.name);
                    return Some(builtin.device.clone());
                } else {
                    info!("No Built-in microphone found. Falling back to Bluetooth default.");
                }
            }
        }
        
        // Standard default behavior if not Bluetooth or no fallback found
        devices.into_iter().find(|d| d.is_default).map(|d| d.device)
    }
    
    /// Wrapper that fetches devices and calls get_effective_device_from_list.
    /// Only used when device list hasn't been pre-fetched.
    fn get_effective_microphone_device(&self, settings: &AppSettings) -> Option<cpal::Device> {
        match list_input_devices() {
            Ok(devices) => self.get_effective_device_from_list(settings, devices),
            Err(e) => {
                debug!("Failed to list devices: {}", e);
                None
            }
        }
    }

    /// Get the name of the currently effective microphone device.
    /// This considers clamshell mode overrides and falls back to the default device.
    pub fn get_effective_device_name(&self) -> Option<String> {
        let settings = get_settings(&self.app_handle);
        
        // Check if we're in clamshell mode and have a clamshell microphone configured
        let use_clamshell_mic = if let Ok(is_clamshell) = clamshell::is_clamshell() {
            is_clamshell && settings.clamshell_microphone.is_some()
        } else {
            false
        };

        // First try explicitly selected devices
        if use_clamshell_mic {
            if let Some(name) = settings.clamshell_microphone.clone() {
                return Some(name);
            }
        }
        
        if let Some(name) = settings.selected_microphone.clone() {
            return Some(name);
        }

        // No mic explicitly selected - prioritize built-in over Bluetooth default
        debug!("No microphone selected in settings, checking available devices");
        match list_input_devices() {
            Ok(devices) => {
                // First, try to find a built-in microphone
                let builtin_device = devices.iter().find(|d| {
                    crate::audio_device_info::is_device_builtin(&d.name) &&
                    !crate::audio_device_info::is_device_virtual(&d.name)
                });
                
                if let Some(device) = builtin_device {
                    info!(device = %device.name, "Preferring built-in microphone over system default");
                    return Some(device.name.clone());
                }
                
                // If no built-in found, use system default (but not if Bluetooth)
                let default_device = devices.iter().find(|d| d.is_default);
                if let Some(device) = default_device {
                    // If default is Bluetooth, try to find any non-Bluetooth alternative
                    if crate::audio_device_info::is_device_bluetooth(&device.name) {
                        let non_bt_device = devices.iter().find(|d| {
                            !crate::audio_device_info::is_device_bluetooth(&d.name) &&
                            !crate::audio_device_info::is_device_virtual(&d.name)
                        });
                        
                        if let Some(alt_device) = non_bt_device {
                            info!(
                                default = %device.name, 
                                alternative = %alt_device.name, 
                                "Default is Bluetooth, using non-Bluetooth alternative"
                            );
                            return Some(alt_device.name.clone());
                        }
                    }
                    
                    debug!(device = %device.name, "Using default input device");
                    return Some(device.name.clone());
                }
            }
            Err(e) => {
                debug!("Failed to list devices for default: {}", e);
            }
        }
        
        None
    }
    
    /// Get the device name from settings only (no device enumeration).
    /// This is faster but won't return the default device if nothing is selected.
    fn get_selected_device_name_fast(&self) -> Option<String> {
        let settings = get_settings(&self.app_handle);
        
        // Check clamshell mode first
        let use_clamshell_mic = if let Ok(is_clamshell) = clamshell::is_clamshell() {
            is_clamshell && settings.clamshell_microphone.is_some()
        } else {
            false
        };

        if use_clamshell_mic {
            return settings.clamshell_microphone.clone();
        }
        
        settings.selected_microphone.clone()
    }

    /// Check if the currently selected microphone is a Bluetooth device.
    /// Uses a fast path that avoids device enumeration when possible.
    pub fn is_current_device_bluetooth(&self) -> bool {
        // Fast path: check settings first (no CPAL calls)
        if let Some(device_name) = self.get_selected_device_name_fast() {
            let is_bt = crate::audio_device_info::is_device_bluetooth(&device_name);
            info!(
                device = device_name,
                is_bluetooth = is_bt,
                method = "settings",
                "Bluetooth device check"
            );
            return is_bt;
        }
        
        // Slow path: need to enumerate devices to find the default
        // Only do this if no device is explicitly selected in settings
        debug!("No device in settings, checking default device (slow path)");
        match self.get_effective_device_name() {
            Some(device_name) => {
                let is_bt = crate::audio_device_info::is_device_bluetooth(&device_name);
                info!(
                    device = device_name,
                    is_bluetooth = is_bt,
                    method = "default_device",
                    "Bluetooth device check"
                );
                is_bt
            }
            None => {
                debug!("No device name available, assuming not Bluetooth");
                false
            }
        }
    }
    
    /// Check if this is the first recording trigger since app start.
    /// Used to determine longer warmup times on first use.
    pub fn is_first_trigger(&self) -> bool {
        let has_recorded = *self.has_recorded_before.lock().unwrap();
        let is_first = !has_recorded;
        info!(
            has_recorded_before = has_recorded,
            is_first_trigger = is_first,
            "First trigger detection check"
        );
        is_first
    }
    
    /// Mark that a recording has been successfully started.
    /// Called after the first successful recording to disable first-trigger behavior.
    fn mark_recording_started(&self) {
        let mut has_recorded = self.has_recorded_before.lock().unwrap();
        if !*has_recorded {
            info!("Marking first recording as completed - future recordings will skip first-trigger warmup");
            *has_recorded = true;
        }
    }
    
    /// Pre-warm a Bluetooth microphone by briefly opening the audio stream.
    /// This triggers the Bluetooth A2DP→HFP profile switch in the background,
    /// so when the user presses fn, the mic is already in the correct mode.
    /// This significantly reduces perceived latency for Bluetooth mics.
    pub fn prewarm_bluetooth_mic(&self) {
        // Only pre-warm Bluetooth devices
        if !self.is_current_device_bluetooth() {
            debug!("Skipping pre-warm: not a Bluetooth device");
            return;
        }
        
        // Don't pre-warm if already open (e.g., always-on mode)
        if *self.is_open.lock().unwrap() {
            debug!("Skipping pre-warm: microphone stream already open");
            return;
        }
        
        info!("Pre-warming Bluetooth microphone in background");
        
        // Clone what we need for the background thread
        let is_open = Arc::clone(&self.is_open);
        let is_recording = Arc::clone(&self.is_recording);
        let recorder = Arc::clone(&self.recorder);
        let app_handle = self.app_handle.clone();
        let did_mute = Arc::clone(&self.did_mute);
        let zero_level_count = Arc::clone(&self.zero_level_count);
        let failover_timestamp = Arc::clone(&self.failover_timestamp);
        let current_device_name = Arc::clone(&self.current_device_name);
        
        std::thread::spawn(move || {
            // Open the microphone stream to trigger Bluetooth profile switch
            let start_time = Instant::now();
            
            // Get VAD path for recorder initialization
            let vad_path = match app_handle.path().resolve(
                "resources/models/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            ) {
                Ok(path) => path,
                Err(e) => {
                    debug!("Pre-warm failed to resolve VAD path: {}", e);
                    return;
                }
            };
            
            // Initialize recorder if needed
            {
                let mut recorder_guard = recorder.lock().unwrap();
                if recorder_guard.is_none() {
                    match create_audio_recorder(vad_path.to_str().unwrap(), &app_handle, zero_level_count.clone(), failover_timestamp.clone(), current_device_name.clone()) {
                        Ok(rec) => *recorder_guard = Some(rec),
                        Err(e) => {
                            debug!("Pre-warm failed to create recorder: {}", e);
                            return;
                        }
                    }
                }
            }
            
            // Open the stream (this triggers Bluetooth profile switch)
            {
                let settings = get_settings(&app_handle);
                let mut recorder_guard = recorder.lock().unwrap();
                if let Some(rec) = recorder_guard.as_mut() {
                    // Get the device to use by name lookup
                    let selected_device = {
                        // Get effective device considering clamshell mode
                        let use_clamshell = if let Ok(is_clamshell) = clamshell::is_clamshell() {
                            is_clamshell && settings.clamshell_microphone.is_some()
                        } else {
                            false
                        };
                        
                        let device_name = if use_clamshell {
                            settings.clamshell_microphone.as_ref()
                        } else {
                            settings.selected_microphone.as_ref()
                        };
                        
                        // Find device by name from available devices
                        device_name.and_then(|name| {
                            match list_input_devices() {
                                Ok(devices) => devices.into_iter()
                                    .find(|d| &d.name == name)
                                    .map(|d| d.device),
                                Err(_) => None
                            }
                        })
                    };
                    
                    if let Err(e) = rec.open(selected_device) {
                        debug!("Pre-warm failed to open stream: {}", e);
                        return;
                    }
                }
            }
            
            *is_open.lock().unwrap() = true;
            info!("Pre-warm: Bluetooth profile switch triggered in {:?}", start_time.elapsed());
            
            // Keep the stream open briefly to ensure profile switch completes
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Close the stream (unless user started recording in the meantime)
            if !*is_recording.lock().unwrap() {
                // Safely close the stream
                if let Some(rec) = recorder.lock().unwrap().as_mut() {
                    let _ = rec.close();
                }
                *is_open.lock().unwrap() = false;
                
                // Reset mute flag
                *did_mute.lock().unwrap() = false;
                
                info!("Pre-warm complete: Bluetooth microphone ready");
            } else {
                debug!("Pre-warm: User started recording, keeping stream open");
            }
        });
    }

    pub fn warmup_recorder(&self) {
        // Just create the recorder if it doesn't exist. 
        // This loads the VAD model (Silero ONNX) which is the expensive part.
        // We do NOT call open() so the microphone light stays off.
        let mut recorder_opt = self.recorder.lock().unwrap();
        if recorder_opt.is_none() {
             use tracing::debug;
             debug!("Warming up audio recorder (loading VAD model)...");
             let vad_path = match self.app_handle.path().resolve(
                "resources/models/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            ) {
                Ok(path) => path,
                Err(e) => {
                    tracing::warn!("Warmup failed to resolve VAD path: {}", e);
                    return;
                }
            };
            
            match create_audio_recorder(vad_path.to_str().unwrap(), &self.app_handle, self.zero_level_count.clone(), self.failover_timestamp.clone(), self.current_device_name.clone()) {
                Ok(rec) => {
                    *recorder_opt = Some(rec);
                    debug!("Audio recorder warmed up successfully");
                }
                Err(e) => {
                    tracing::warn!("Warmup failed to create recorder: {}", e);
                }
            }
        }
    }

    /* ---------- microphone life-cycle -------------------------------------- */

    /// Applies mute if mute_while_recording is enabled and stream is open
    pub fn apply_mute(&self) {
        let settings = get_settings(&self.app_handle);
        let mut did_mute_guard = self.did_mute.lock().unwrap();

        if settings.mute_while_recording && *self.is_open.lock().unwrap() {
            set_mute(true);
            *did_mute_guard = true;
            debug!("Mute applied");
        }
    }

    /// Removes mute if it was applied
    pub fn remove_mute(&self) {
        let mut did_mute_guard = self.did_mute.lock().unwrap();
        if *did_mute_guard {
            set_mute(false);
            *did_mute_guard = false;
            debug!("Mute removed");
        }
    }

    pub fn start_microphone_stream(&self) -> Result<(), anyhow::Error> {
        // ══════════════════════════════════════════════════════════════════════
        // PHASE 1: Quick check if already open (minimal lock, fast path)
        // ══════════════════════════════════════════════════════════════════════
        {
            let open_flag = self.is_open.lock().unwrap();
            if *open_flag {
                debug!("Microphone stream already active");
                return Ok(());
            }
        } // Lock released immediately

        let start_time = Instant::now();
        info!("[TIMING] start_microphone_stream starting...");

        // Reset zero-level counter (but NOT failover_timestamp) to prevent false positives
        // from stale readings during device transitions
        self.reset_zero_level_counter_only();

        // ══════════════════════════════════════════════════════════════════════
        // PHASE 2: Device enumeration OUTSIDE of locks (this is the slow part)
        // ══════════════════════════════════════════════════════════════════════
        let enum_start = Instant::now();
        let devices = list_input_devices()
            .map_err(|e| anyhow::anyhow!("Failed to enumerate devices: {}", e))?;
        info!("[TIMING] Device enumeration completed in {:?} ({} devices)", enum_start.elapsed(), devices.len());

        // Get settings and find the target device (still outside locks)
        let settings = get_settings(&self.app_handle);
        let selected_device = self.get_effective_device_from_list(&settings, devices.clone());
        
        // Determine failed device name for potential fallback logging
        let target_device_name = settings.selected_microphone.clone().unwrap_or_else(|| "Default".to_string());
        debug!("Target device: {}", target_device_name);

        // ══════════════════════════════════════════════════════════════════════
        // PHASE 3: Prepare recorder (VAD path resolution - fast)
        // ══════════════════════════════════════════════════════════════════════
        let vad_path = self
            .app_handle
            .path()
            .resolve(
                "resources/models/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| anyhow::anyhow!("Failed to resolve VAD path: {}", e))?;

        // ══════════════════════════════════════════════════════════════════════
        // PHASE 4: Acquire locks and perform quick state changes
        // ══════════════════════════════════════════════════════════════════════
        let lock_start = Instant::now();
        let mut open_flag = self.is_open.lock().unwrap();
        
        // Double-check another thread didn't open while we were enumerating
        if *open_flag {
            info!("[TIMING] Lock check: stream opened by another thread during enumeration");
            return Ok(());
        }
        
        // Reset mute flag
        let mut did_mute_guard = self.did_mute.lock().unwrap();
        *did_mute_guard = false;
        drop(did_mute_guard);

        let mut recorder_opt = self.recorder.lock().unwrap();

        if recorder_opt.is_none() {
            *recorder_opt = Some(create_audio_recorder(
                vad_path.to_str().unwrap(),
                &self.app_handle,
                self.zero_level_count.clone(),
                self.failover_timestamp.clone(),
                self.current_device_name.clone(),
            )?);
        }

        if let Some(rec) = recorder_opt.as_mut() {
            // First attempt to open with the selected device
            if let Err(e) = rec.open(selected_device.clone()) {
                error!("Failed to open recorder (attempt 1): {}", e);
                
                // ══════════════════════════════════════════════════════════════
                // FAILOVER: Use pre-fetched device list to find fallback
                // (No new list_input_devices call - use the devices we already have)
                // ══════════════════════════════════════════════════════════════
                if let Some((fallback_name, fallback_device)) = 
                    self.find_fallback_device_from_list(&target_device_name, devices) 
                {
                    info!("Retrying with fallback device: {}", fallback_name);
                    
                    // IMPORTANT: Remove the fallback device from blocklist if it was there
                    // (from a previous failover session). This ensures it can be used.
                    {
                        let mut blocked = self.blocked_devices.lock().unwrap();
                        if blocked.remove(&fallback_name) {
                            info!("Removed '{}' from blocklist (now active as fallback)", fallback_name);
                        }
                    }
                    
                    // Update settings to persist the fallback choice
                    let mut settings = get_settings(&self.app_handle);
                    settings.selected_microphone = Some(fallback_name.clone());
                    crate::settings::write_settings(&self.app_handle, settings);
                    
                    // Notify frontend and system AFTER settings are written
                    info!("Emitting audio-device-auto-switched event: {} -> {}", target_device_name, fallback_name);
                    let _ = self.app_handle.emit("audio-device-auto-switched", serde_json::json!({
                        "previous": target_device_name,
                        "current": fallback_name
                    }));
                    
                    let _ = self.app_handle.notification().builder()
                        .title("Microphone Changed")
                        .body(&format!("Switched to {} due to connection error.", fallback_name))
                        .show();
                    
                    // Retry open with fallback
                    rec.reset_cache();
                    rec.open(Some(fallback_device))
                        .map_err(|e| anyhow::anyhow!("Failed to open fallback recorder: {}", e))?;
                    
                    // CRITICAL: Reset dead device counters after successful fallback
                    // to prevent false positives from stale zero-level readings
                    *self.zero_level_count.lock().unwrap() = 0;
                    *self.failover_timestamp.lock().unwrap() = None;
                    
                    // Track the fallback device as current
                    *self.current_device_name.lock().unwrap() = Some(fallback_name);
                } else {
                    // No fallback found, propagate original error
                    return Err(anyhow::anyhow!("Failed to open recorder and no fallback found: {}", e));
                }
            } else {
                // Successfully opened with target device - track it
                *self.current_device_name.lock().unwrap() = Some(target_device_name.clone());
            }
        }

        *open_flag = true;
        info!(
            "[TIMING] Lock held for {:?}, total init: {:?} (active: {})",
            lock_start.elapsed(),
            start_time.elapsed(),
            self.current_device_name.lock().unwrap().clone().unwrap_or("Default".to_string())
        );
        Ok(())
    }
    
    /// Find a fallback device from a pre-fetched device list.
    /// Returns (device_name, device) tuple if found.
    fn find_fallback_device_from_list(
        &self,
        failed_device_name: &str,
        devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>,
    ) -> Option<(String, cpal::Device)> {
        info!("[TIMING] Finding fallback device (using pre-fetched list, no new enumeration)");
        
        // Get blocked devices for filtering
        let blocked = self.blocked_devices.lock().unwrap();
        
        // Filter candidates
        let mut candidates: Vec<_> = devices
            .into_iter()
            .filter(|d| {
                // Exclude the failed device
                if d.name == failed_device_name {
                    return false;
                }
                
                // Exclude blocked devices (previously detected as dead)
                if blocked.contains(&d.name) {
                    debug!("Excluding '{}' from fallback candidates (in blocklist)", d.name);
                    return false;
                }
                
                // Exclude virtual/phantom devices
                if crate::audio_device_info::is_device_virtual(&d.name) {
                    return false;
                }
                
                true
            })
            .collect();
        
        drop(blocked);

        // Sort by priority: Built-in first, then Wired, then Bluetooth
        candidates.sort_by(|a, b| {
            let a_builtin = crate::audio_device_info::is_device_builtin(&a.name);
            let b_builtin = crate::audio_device_info::is_device_builtin(&b.name);
            
            if a_builtin && !b_builtin {
                return std::cmp::Ordering::Less;
            }
            if !a_builtin && b_builtin {
                return std::cmp::Ordering::Greater;
            }
            
            let a_bt = crate::audio_device_info::is_device_bluetooth(&a.name);
            let b_bt = crate::audio_device_info::is_device_bluetooth(&b.name);
            
            if !a_bt && b_bt {
                return std::cmp::Ordering::Less;
            }
            if a_bt && !b_bt {
                return std::cmp::Ordering::Greater;
            }
            
            std::cmp::Ordering::Equal
        });

        if let Some(best) = candidates.into_iter().next() {
            info!("Found fallback device: {}", best.name);
            return Some((best.name, best.device));
        }

        warn!("No suitable fallback microphone found");
        None
    }

    pub fn stop_microphone_stream(&self) {
        let mut open_flag = self.is_open.lock().unwrap();
        if !*open_flag {
            return;
        }

        let mut did_mute_guard = self.did_mute.lock().unwrap();
        if *did_mute_guard {
            set_mute(false);
        }
        *did_mute_guard = false;

        if let Some(rec) = self.recorder.lock().unwrap().as_mut() {
            // If still recording, stop first.
            if *self.is_recording.lock().unwrap() {
                let _ = rec.stop();
                *self.is_recording.lock().unwrap() = false;
            }
            let _ = rec.close();
        }

        *open_flag = false;
        debug!("Microphone stream stopped");
    }

    /* ---------- mode switching --------------------------------------------- */

    pub fn update_mode(&self, new_mode: MicrophoneMode) -> Result<(), anyhow::Error> {
        let mode_guard = self.mode.lock().unwrap();
        let cur_mode = mode_guard.clone();

        match (cur_mode, &new_mode) {
            (MicrophoneMode::AlwaysOn, MicrophoneMode::OnDemand) => {
                if matches!(*self.state.lock().unwrap(), RecordingState::Idle) {
                    drop(mode_guard);
                    self.stop_microphone_stream();
                }
            }
            (MicrophoneMode::OnDemand, MicrophoneMode::AlwaysOn) => {
                drop(mode_guard);
                self.start_microphone_stream()?;
            }
            _ => {}
        }

        *self.mode.lock().unwrap() = new_mode;
        Ok(())
    }

    /* ---------- recording --------------------------------------------------- */

    pub fn prepare_recording(&self, binding_id: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        if let RecordingState::Idle = *state {
            *state = RecordingState::Preparing {
                binding_id: binding_id.to_string(),
            };
            debug!("Prepared recording for binding {}", binding_id);
            true
        } else {
            debug!("Cannot prepare recording: state is not Idle (current: {:?})", *state);
            false
        }
    }

    pub fn try_start_recording(&self, binding_id: &str, session_id: &str) -> bool {
        // Note: Microphone permission is checked in TranscribeAction::start() before
        // the overlay is shown. This allows for better UX - we can show a modal dialog
        // instead of the overlay when permission is denied.
        
        let mut state = self.state.lock().unwrap();

        // Validate that we are in the expected Preparing state for this binding
        match *state {
            RecordingState::Preparing { binding_id: ref active_binding, .. } if active_binding == binding_id => {
                // Good to go - state is Preparing for the same binding we're trying to start
            },
            RecordingState::Preparing { binding_id: ref active_binding, .. } => {
                // Preparing for a DIFFERENT binding - abort this stale request
                debug!("try_start_recording aborted: preparing for different binding '{}', not '{}'", 
                       active_binding, binding_id);
                return false;
            },
            // If state is Idle, user called stop before start completed (quick release).
            // Abort the recording - don't start a new orphaned recording session.
            RecordingState::Idle => {
                debug!("try_start_recording aborted: state is Idle (stop was called during prepare)");
                return false;
            },
            // If state implies we are already recording, abort
            ref other => {
                error!("try_start_recording aborted: state changed to {:?}", other);
                return false;
            }
        }

        // Ensure microphone is open in on-demand mode
        if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
            if let Err(e) = self.start_microphone_stream() {
                error!("Failed to open microphone stream: {e}");
                // If we failed, reset state to Idle so next attempt can work
                *state = RecordingState::Idle;
                return false;
            }
        }

        if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
            if rec.start().is_ok() {
                *self.is_recording.lock().unwrap() = true;
                *state = RecordingState::Recording {
                    binding_id: binding_id.to_string(),
                    session_id: session_id.to_string(),
                };
                
                // Start recording timer
                let start_time = Instant::now();
                *self.recording_start_time.lock().unwrap() = Some(start_time);
                self.start_recording_timer(binding_id.to_string());
                
                // Mark that we've successfully recorded (for first-trigger detection)
                self.mark_recording_started();
                
                debug!(session = session_id, "Recording started for binding {binding_id}");
                return true;
            }
        }
        // If we got here, something failed. Reset state.
        error!("Recorder not available or start failed");
        *state = RecordingState::Idle;
        false
    }
    
    /// Start a timer thread that emits recording time updates every second
    fn start_recording_timer(&self, binding_id: String) {
        use crate::actions::ACTION_MAP;
        use crate::i18n;
        
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        *self.timer_stop_tx.lock().unwrap() = Some(stop_tx);
        
        let app_handle = self.app_handle.clone();
        let max_secs = utils::get_recording_limit_seconds();
        let is_recording = self.is_recording.clone();
        let recording_start_time = self.recording_start_time.clone();
        let timer_stop_tx = self.timer_stop_tx.clone();
        
        // Get the localized warning message before spawning the thread
        let warning_message = i18n::t(&self.app_handle, "recording.limitWarning");
        
        std::thread::spawn(move || {
            let mut warned_at_30s = false;
            let mut next_tick: u32 = 1; // Next second to emit (1-indexed since we emit after first second)
            
            // Get the start instant for drift-free timing
            let start_instant = match *recording_start_time.lock().unwrap() {
                Some(t) => t,
                None => return,
            };
            
            loop {
                // Check if we should stop
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                
                // Check if still recording
                if !*is_recording.lock().unwrap() {
                    break;
                }
                
                // Calculate elapsed time
                let elapsed = start_instant.elapsed().as_secs() as u32;
                
                // If we've passed the next tick point, emit update
                if elapsed >= next_tick {
                    // Emit time update to overlay
                    overlay::emit_recording_time(&app_handle, elapsed, max_secs);
                    
                    // Check for 30s warning - use native notification
                    let remaining = max_secs.saturating_sub(elapsed);
                    if remaining <= 30 && remaining > 0 && !warned_at_30s {
                        // Use centralized notification module
                        crate::notification::show_info_with_text(&app_handle, &warning_message);
                        
                        warned_at_30s = true;
                        info!("Recording limit warning: {}s remaining", remaining);
                    }
                    
                    // Check for auto-stop at limit
                    if elapsed >= max_secs {
                        info!("Recording limit reached ({}s), auto-stopping", max_secs);
                        
                        // Clean up timer state before triggering stop to avoid double-call
                        // The action.stop() will eventually call stop_recording_timer(),
                        // but we've already cleared the sender so it's a no-op
                        *timer_stop_tx.lock().unwrap() = None;
                        
                        // Trigger the transcribe action's stop handler to properly process recording
                        // This matches how SIGUSR2 and keyboard shortcuts trigger transcription
                        if let Some(action) = ACTION_MAP.get("transcribe") {
                            info!("Triggering transcribe action stop for binding: {}", binding_id);
                            action.stop(&app_handle, &binding_id, "recording_limit");
                        } else {
                            error!("Failed to find transcribe action in ACTION_MAP");
                        }
                        
                        break;
                    }
                    
                    next_tick = elapsed + 1;
                }
                
                // Calculate sleep duration until next tick to avoid drift
                let target_time = start_instant + Duration::from_secs(next_tick as u64);
                let now = Instant::now();
                let sleep_duration = if target_time > now {
                    target_time - now
                } else {
                    // We're behind, catch up immediately
                    Duration::from_millis(10)
                };
                
                std::thread::sleep(sleep_duration);
            }
        });
    }
    
    /// Stop the recording timer if running
    fn stop_recording_timer(&self) {
        if let Some(tx) = self.timer_stop_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        *self.recording_start_time.lock().unwrap() = None;
    }

    pub fn update_selected_device(&self) -> Result<(), anyhow::Error> {
        // Reset cache to ensure we fetch fresh config for the new device
        if let Some(rec) = self.recorder.lock().unwrap().as_mut() {
            rec.reset_cache();
        }
        
        // Reset first-trigger status so the new device gets a proper warmup
        *self.has_recorded_before.lock().unwrap() = false;

        // If currently open, restart the microphone stream to use the new device
        if *self.is_open.lock().unwrap() {
            self.stop_microphone_stream();
            self.start_microphone_stream()?;
        }
        Ok(())
    }

    pub fn stop_recording(&self, binding_id: &str) -> Option<Vec<f32>> {
        let mut state = self.state.lock().unwrap();

        match *state {
            RecordingState::Preparing {
                binding_id: ref active,
                ..
            } if active == binding_id => {
                // Race condition handled: User stopped before start completed!
                debug!("stop_recording called while Preparing. Cancelling start.");
                *state = RecordingState::Idle;
                return None;
            }
            RecordingState::Recording {
                binding_id: ref active,
                session_id: _,
            } if active == binding_id => {
                *state = RecordingState::Idle;
                drop(state);
                
                // Stop the recording timer
                self.stop_recording_timer();

                let samples = if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                    match rec.stop() {
                        Ok(buf) => buf,
                        Err(e) => {
                            error!("stop() failed: {e}");
                            Vec::new()
                        }
                    }
                } else {
                    error!("Recorder not available");
                    Vec::new()
                };

                *self.is_recording.lock().unwrap() = false;

                // In on-demand mode turn the mic off again
                if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
                    self.stop_microphone_stream();
                }

                // Pad if very short
                let s_len = samples.len();
                // debug!("Got {} samples", s_len);
                
                // Check for 0 samples - this likely means the audio stream died (e.g. device disconnected)
                // Trigger failover to a working device
                if s_len == 0 {
                    warn!("Recording yielded 0 samples - device may have disconnected. Triggering failover...");
                    
                    // Check if a failover just happened - if so, this is likely a false positive
                    // (the stream returned 0 samples during the mic switch transition)
                    let in_grace_period = self.failover_timestamp.lock().unwrap()
                        .map(|t| t.elapsed().as_secs() < FAILOVER_GRACE_PERIOD_SECS)
                        .unwrap_or(false);
                    if in_grace_period {
                        warn!("Skipping blocklist addition - within {}s grace period after failover", FAILOVER_GRACE_PERIOD_SECS);
                        // Don't add to blocklist, just force restart
                        self.stop_microphone_stream();
                    } else {
                        // Get current device name to exclude from fallback search
                        let settings = get_settings(&self.app_handle);
                        let failed_device_name = settings.selected_microphone.clone()
                            .unwrap_or_else(|| "Default".to_string());
                        
                        // Add to blocklist to prevent switching back
                        {
                            let mut blocked = self.blocked_devices.lock().unwrap();
                            if blocked.insert(failed_device_name.clone()) {
                                info!("Added '{}' to dead device blocklist", failed_device_name);
                            }
                        }
                        
                        // Enumerate devices and find fallback (outside any locks at this point)
                        if let Ok(devices) = list_input_devices() {
                            if let Some((fallback_name, _fallback_device)) = 
                                self.find_fallback_device_from_list(&failed_device_name, devices) 
                            {
                                info!("Switching to fallback device after 0-sample recording: {}", fallback_name);
                                
                                // IMPORTANT: Remove the fallback device from blocklist if it was there
                                {
                                    let mut blocked = self.blocked_devices.lock().unwrap();
                                    if blocked.remove(&fallback_name) {
                                        info!("Removed '{}' from blocklist (now active as fallback)", fallback_name);
                                    }
                                }
                                
                                // Update settings to use the fallback device
                                let mut settings = get_settings(&self.app_handle);
                                settings.selected_microphone = Some(fallback_name.clone());
                                crate::settings::write_settings(&self.app_handle, settings);
                                
                                // Notify frontend and system AFTER settings are written
                                info!("Emitting audio-device-auto-switched event: {} -> {}", failed_device_name, fallback_name);
                                let _ = self.app_handle.emit("audio-device-auto-switched", serde_json::json!({
                                    "previous": failed_device_name,
                                    "current": fallback_name
                                }));
                                
                                let _ = self.app_handle.notification().builder()
                                    .title("Microphone Changed")
                                    .body(&format!("Switched to {} because {} was unavailable.", fallback_name, failed_device_name))
                                    .show();
                            } else {
                                warn!("No fallback device found, will retry with same device");
                            }
                        }
                        
                        // CRITICAL: Reset dead device counters after successful fallback
                        // to prevent false positives from stale zero-level readings on next recording
                        self.reset_dead_device_counters();
                        
                        // Force restart - next recording will use the new device
                        self.stop_microphone_stream();
                    }
                }

                if s_len < WHISPER_SAMPLE_RATE && s_len > 0 {
                    let mut padded = samples;
                    padded.resize(WHISPER_SAMPLE_RATE * 5 / 4, 0.0);
                    Some(padded)
                } else {
                    Some(samples)
                }
            }
            _ => {
                // Idle or other binding active
                None
            }
        }
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }
    
    pub fn get_active_binding_id(&self) -> Option<String> {
        match &*self.state.lock().unwrap() {
            RecordingState::Recording { binding_id, .. } => Some(binding_id.clone()),
            RecordingState::Preparing { binding_id, .. } => Some(binding_id.clone()),
            RecordingState::Idle => None,
        }
    }

    /* ---------- failover logic --------------------------------------------- */

    /// Attempts to switch to a fallback microphone, excluding the current failed device.
    /// Returns the new device name if successful.
    fn switch_to_fallback_mic(&self, failed_device_name: &str) -> Option<String> {
        info!("Attempting to switch to fallback microphone (failed: {})", failed_device_name);

        let devices = match crate::audio_toolkit::audio::list_input_devices() {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to list devices for fallback: {}", e);
                return None;
            }
        };

        // Filter candidates
        let mut candidates: Vec<_> = devices
            .into_iter()
            .filter(|d| {
                // Exclude the failed device
                if d.name == failed_device_name {
                    return false;
                }
                
                // Exclude virtual/phantom devices (unless we are desperate, but user said never virtual)
                if crate::audio_device_info::is_device_virtual(&d.name) {
                    return false;
                }
                
                true
            })
            .collect();

        // Sort by priority:
        // 1. Built-in
        // 2. Wired (not bluetooth)
        // 3. Bluetooth (last resort)
        candidates.sort_by(|a, b| {
            let a_builtin = crate::audio_device_info::is_device_builtin(&a.name);
            let b_builtin = crate::audio_device_info::is_device_builtin(&b.name);
            
            if a_builtin && !b_builtin {
                return std::cmp::Ordering::Less; // a comes first
            }
            if !a_builtin && b_builtin {
                return std::cmp::Ordering::Greater;
            }
            
            let a_bt = crate::audio_device_info::is_device_bluetooth(&a.name);
            let b_bt = crate::audio_device_info::is_device_bluetooth(&b.name);
            
            if !a_bt && b_bt {
                return std::cmp::Ordering::Less; // non-bt comes first
            }
            if a_bt && !b_bt {
                return std::cmp::Ordering::Greater;
            }
            
            std::cmp::Ordering::Equal
        });

        if let Some(best) = candidates.first() {
            info!("Found fallback device: {}", best.name);
            
            // Update settings
            let mut settings = get_settings(&self.app_handle);
            settings.selected_microphone = Some(best.name.clone());
            crate::settings::write_settings(&self.app_handle, settings);
            
            // Notify frontend and system
            let _ = self.app_handle.emit("audio-device-auto-switched", serde_json::json!({
                "previous": failed_device_name,
                "current": best.name
            }));
            
            let _ = self.app_handle.notification().builder()
                .title("Microphone Changed")
                .body(&format!("Switched to {} due to connection error.", best.name))
                .show();

            return Some(best.name.clone());
        }

        warn!("No suitable fallback microphone found");
        None
    }

    /// Get the current session ID if recording is active
    pub fn get_current_session_id(&self) -> Option<String> {
        match &*self.state.lock().unwrap() {
            RecordingState::Recording { session_id, .. } => Some(session_id.clone()),
            RecordingState::Idle | RecordingState::Preparing { .. } => None,
        }
    }




    /// Cancel any ongoing recording without returning audio samples
    pub fn cancel_recording(&self) {
        let mut state = self.state.lock().unwrap();

        match *state {
            RecordingState::Preparing { .. } => {
                *state = RecordingState::Idle;
                debug!("Cancelled recording (while preparing)");
            }
            RecordingState::Recording { .. } => {
                *state = RecordingState::Idle;
                drop(state);
                
                // Stop the recording timer
                self.stop_recording_timer();

                if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                    let _: Result<Vec<f32>, _> = rec.stop(); // Discard the result, fixing type inference
                }

                *self.is_recording.lock().unwrap() = false;

                // In on-demand mode turn the mic off again
                if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
                    self.stop_microphone_stream();
                }
            }
            RecordingState::Idle => {}
        }
    }
}
