use crate::audio_toolkit::{list_input_devices, vad::SmoothedVad, AudioRecorder, SileroVad};
use crate::helpers::clamshell;
use crate::overlay;
use crate::settings::{get_settings, AppSettings};
use crate::utils;
use tracing::{debug, error, info};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

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
) -> Result<AudioRecorder, anyhow::Error> {
    let silero = SileroVad::new(vad_path, 0.3)
        .map_err(|e| anyhow::anyhow!("Failed to create SileroVad: {}", e))?;
    let smoothed_vad = SmoothedVad::new(Box::new(silero), 15, 15, 2);

    // Recorder with VAD plus a spectrum-level callback that forwards updates to
    // the frontend.
    let recorder = AudioRecorder::new()
        .map_err(|e| anyhow::anyhow!("Failed to create AudioRecorder: {}", e))?
        .with_vad(Box::new(smoothed_vad))
        .with_level_callback({
            let app_handle = app_handle.clone();
            move |levels| {
                utils::emit_levels(&app_handle, &levels);
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
        };

        // Always-on?  Open immediately.
        if matches!(mode, MicrophoneMode::AlwaysOn) {
            manager.start_microphone_stream()?;
        }

        Ok(manager)
    }

    /* ---------- helper methods --------------------------------------------- */

    fn get_effective_microphone_device(&self, settings: &AppSettings) -> Option<cpal::Device> {
        // Check if we're in clamshell mode and have a clamshell microphone configured
        let use_clamshell_mic = if let Ok(is_clamshell) = clamshell::is_clamshell() {
            is_clamshell && settings.clamshell_microphone.is_some()
        } else {
            false
        };

        if use_clamshell_mic {
             let device_name = settings.clamshell_microphone.as_ref().unwrap();
             return list_input_devices().ok()?
                .into_iter()
                .find(|d| d.name == *device_name)
                .map(|d| d.device);
        }

        // Logic for handling standard selection vs Default
        let target_device_name = if let Some(name) = &settings.selected_microphone {
            // User explicitly selected a microphone -> Use it strictly
            Some(name.clone())
        } else {
            // "Default" is selected (None in settings)
            // Safety Check: If the system default is Bluetooth, try to fallback to an Internal Mic
            // to prevent low-quality audio.
            
            // 1. Get all devices first
            match list_input_devices() {
                Ok(devices) => {
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
                                    return Some(builtin.device.clone()); // Assuming cpal::Device is clonable or we need to handle it
                                } else {
                                    info!("No Built-in microphone found. Falling back to Bluetooth default.");
                                }
                         }
                    }
                    
                    // Standard default behavior if not Bluetooth or no fallback found
                    return devices.into_iter().find(|d| d.is_default).map(|d| d.device);
                }
                Err(e) => {
                    debug!("Failed to list devices for default resolution: {}", e);
                    return None;
                }
            }
        };


        // Standard lookup by name (for explicit selection)
        if let Some(name) = target_device_name {
            match list_input_devices() {
                Ok(devices) => devices
                    .into_iter()
                    .find(|d| d.name == name)
                    .map(|d| d.device),
                Err(e) => {
                    debug!("Failed to list devices: {}", e);
                    None
                }
            }
        } else {
             None
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

        // Fall back to default input device
        debug!("No microphone selected in settings, checking default device");
        match list_input_devices() {
            Ok(devices) => {
                let default_device = devices.into_iter().find(|d| d.is_default);
                if let Some(device) = default_device {
                    debug!(device = %device.name, "Using default input device for Bluetooth check");
                    return Some(device.name);
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
                    match create_audio_recorder(vad_path.to_str().unwrap(), &app_handle) {
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
            
            match create_audio_recorder(vad_path.to_str().unwrap(), &self.app_handle) {
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
        let mut open_flag = self.is_open.lock().unwrap();
        if *open_flag {
            debug!("Microphone stream already active");
            return Ok(());
        }

        let start_time = Instant::now();

        // Don't mute immediately - caller will handle muting after audio feedback
        let mut did_mute_guard = self.did_mute.lock().unwrap();
        *did_mute_guard = false;

        let vad_path = self
            .app_handle
            .path()
            .resolve(
                "resources/models/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| anyhow::anyhow!("Failed to resolve VAD path: {}", e))?;
        let mut recorder_opt = self.recorder.lock().unwrap();

        if recorder_opt.is_none() {
            *recorder_opt = Some(create_audio_recorder(
                vad_path.to_str().unwrap(),
                &self.app_handle,
            )?);
        }

        // Get the selected device from settings, considering clamshell mode
        let settings = get_settings(&self.app_handle);
        let selected_device = self.get_effective_microphone_device(&settings);

        if let Some(rec) = recorder_opt.as_mut() {
            rec.open(selected_device)
                .map_err(|e| anyhow::anyhow!("Failed to open recorder: {}", e))?;
        }

        *open_flag = true;
        info!(
            "Microphone stream initialized in {:?}",
            start_time.elapsed()
        );
        Ok(())
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
                
                // Check for 0 samples - this likely means the audio stream died (e.g. sleep/wake)
                // Force a restart of the stream for the next attempt
                if s_len == 0 {
                    tracing::warn!("Recording yielded 0 samples, forcing microphone stream restart to recover");
                    self.stop_microphone_stream();
                    // If in AlwaysOn mode, we should ideally restart it immediately, 
                    // but stop_microphone_stream() just closes it.
                    // The next try_start_recording call will re-open it because we set is_open=false.
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
                    let _ = rec.stop(); // Discard the result
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
