use crate::audio_toolkit::audio::{
    list_input_devices, AudioRecorder, RecorderStartError, RecorderStartWait,
};
use crate::audio_toolkit::vad::SmoothedVad;
use crate::audio_toolkit::SileroVad;
use crate::helpers::clamshell;
use crate::overlay;
use crate::settings::{get_settings, AppSettings};
use crate::utils;
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tracing::{debug, error, info, warn};

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
pub struct StoppedRecording {
    pub samples_for_transcription: Vec<f32>,
    pub speech_duration_ms: i64,
    pub recording_duration_ms: i64,
}

fn build_stopped_recording(
    speech_samples: Vec<f32>,
    mut recording_duration_ms: i64,
) -> StoppedRecording {
    let speech_sample_count = speech_samples.len();
    let mut samples_for_transcription = speech_samples;
    if speech_sample_count < WHISPER_SAMPLE_RATE && speech_sample_count > 0 {
        samples_for_transcription.resize(WHISPER_SAMPLE_RATE * 5 / 4, 0.0);
    }

    let mut speech_duration_ms = ((speech_sample_count as i64) * 1000) / WHISPER_SAMPLE_RATE as i64;

    if recording_duration_ms <= 0 && speech_duration_ms > 0 {
        recording_duration_ms = speech_duration_ms;
    }

    if recording_duration_ms > 0 {
        speech_duration_ms = speech_duration_ms.min(recording_duration_ms);
    }

    StoppedRecording {
        samples_for_transcription,
        speech_duration_ms,
        recording_duration_ms,
    }
}

/* ──────────────────────────────────────────────────────────────── */

#[derive(Clone, Debug)]
pub enum RecordingState {
    Idle,
    Preparing {
        binding_id: String,
        prepare_token: PrepareToken,
    },
    CancellingPrepare {
        binding_id: String,
        prepare_token: PrepareToken,
    },
    Recording { binding_id: String, session_id: String },
}

#[derive(Clone, Debug)]
pub enum MicrophoneMode {
    AlwaysOn,
    OnDemand,
}

#[derive(Clone, Debug)]
pub struct RecordingStartSuccess {
    pub capture_ready_latency: Duration,
    pub active_device_name: Option<String>,
}

pub type PrepareToken = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordingStartOwner {
    SelfOwned,
    OtherOwned,
}

impl RecordingStartOwner {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordingStartOwner::SelfOwned => "self",
            RecordingStartOwner::OtherOwned => "other",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateMismatchKind {
    PreparingForDifferentBinding { active_binding: String },
    SupersededByNewPrepareToken {
        active_prepare_token: PrepareToken,
        requested_prepare_token: PrepareToken,
    },
    IdleDuringPrepare,
    CancellingPrepare {
        active_binding: String,
        active_prepare_token: PrepareToken,
    },
    UnexpectedState { state: String },
    IdleBeforeCommit,
    StateChangedBeforeCommit { state: String },
}

impl StateMismatchKind {
    fn details(&self) -> String {
        match self {
            StateMismatchKind::PreparingForDifferentBinding { active_binding } => {
                format!("preparing_for_different_binding:{active_binding}")
            }
            StateMismatchKind::SupersededByNewPrepareToken {
                active_prepare_token,
                requested_prepare_token,
            } => format!(
                "prepare_token_superseded:active={active_prepare_token},requested={requested_prepare_token}"
            ),
            StateMismatchKind::IdleDuringPrepare => "state_idle_during_prepare".to_string(),
            StateMismatchKind::CancellingPrepare {
                active_binding,
                active_prepare_token,
            } => format!(
                "state_cancelling_prepare:binding={active_binding},prepare_token={active_prepare_token}"
            ),
            StateMismatchKind::UnexpectedState { state } => format!("unexpected_state:{state}"),
            StateMismatchKind::IdleBeforeCommit => "state_idle_before_recording_commit".to_string(),
            StateMismatchKind::StateChangedBeforeCommit { state } => {
                format!("state_changed_before_recording_commit:{state}")
            }
        }
    }

    fn owner(&self) -> RecordingStartOwner {
        match self {
            StateMismatchKind::IdleDuringPrepare
            | StateMismatchKind::CancellingPrepare { .. }
            | StateMismatchKind::IdleBeforeCommit => RecordingStartOwner::SelfOwned,
            StateMismatchKind::PreparingForDifferentBinding { .. }
            | StateMismatchKind::SupersededByNewPrepareToken { .. }
            | StateMismatchKind::UnexpectedState { .. }
            | StateMismatchKind::StateChangedBeforeCommit { .. } => RecordingStartOwner::OtherOwned,
        }
    }

    fn should_cleanup_ui(&self) -> bool {
        matches!(
            self,
            StateMismatchKind::IdleDuringPrepare
                | StateMismatchKind::CancellingPrepare { .. }
                | StateMismatchKind::IdleBeforeCommit
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecordingStartFailure {
    StateMismatch(StateMismatchKind),
    StartAbandonedDueToSupersededState(String),
    MaintenanceMode,
    StreamOpenFailed(String),
    RecorderUnavailable,
    CaptureReadyTimeout(Duration),
    StartCommandFailed(String),
}

impl RecordingStartFailure {
    pub fn label(&self) -> &'static str {
        match self {
            RecordingStartFailure::StateMismatch(_) => "state_mismatch",
            RecordingStartFailure::StartAbandonedDueToSupersededState(_) => {
                "start_abandoned_superseded_state"
            }
            RecordingStartFailure::MaintenanceMode => "maintenance_mode",
            RecordingStartFailure::StreamOpenFailed(_) => "stream_open_failed",
            RecordingStartFailure::RecorderUnavailable => "recorder_unavailable",
            RecordingStartFailure::CaptureReadyTimeout(_) => "capture_ready_timeout",
            RecordingStartFailure::StartCommandFailed(_) => "start_command_failed",
        }
    }

    pub fn details(&self) -> String {
        match self {
            RecordingStartFailure::StateMismatch(kind) => kind.details(),
            RecordingStartFailure::StartAbandonedDueToSupersededState(detail) => detail.clone(),
            RecordingStartFailure::MaintenanceMode => "blocked_by_maintenance_mode".to_string(),
            RecordingStartFailure::StreamOpenFailed(detail) => detail.clone(),
            RecordingStartFailure::RecorderUnavailable => "recorder_not_initialized".to_string(),
            RecordingStartFailure::CaptureReadyTimeout(timeout) => {
                format!("timeout_ms={}", timeout.as_millis())
            }
            RecordingStartFailure::StartCommandFailed(detail) => detail.clone(),
        }
    }

    pub fn owner(&self) -> RecordingStartOwner {
        match self {
            RecordingStartFailure::StateMismatch(kind) => kind.owner(),
            RecordingStartFailure::StartAbandonedDueToSupersededState(_) => {
                RecordingStartOwner::OtherOwned
            }
            RecordingStartFailure::MaintenanceMode
            | RecordingStartFailure::StreamOpenFailed(_)
            | RecordingStartFailure::RecorderUnavailable
            | RecordingStartFailure::CaptureReadyTimeout(_)
            | RecordingStartFailure::StartCommandFailed(_) => RecordingStartOwner::SelfOwned,
        }
    }

    pub fn should_cleanup_ui(&self) -> bool {
        match self {
            RecordingStartFailure::StateMismatch(kind) => kind.should_cleanup_ui(),
            RecordingStartFailure::StartAbandonedDueToSupersededState(_) => false,
            RecordingStartFailure::MaintenanceMode
            | RecordingStartFailure::StreamOpenFailed(_)
            | RecordingStartFailure::RecorderUnavailable
            | RecordingStartFailure::CaptureReadyTimeout(_)
            | RecordingStartFailure::StartCommandFailed(_) => true,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RecordingStartOutcome {
    Started(RecordingStartSuccess),
    Failed(RecordingStartFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartFailureCleanupAction {
    ClearPreparingAndCloseStream,
    CloseStream,
    Noop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartCommitMismatchOwner {
    SelfCancelled,
    SupersededByOther,
}

impl StartCommitMismatchOwner {
    fn as_str(self) -> &'static str {
        match self {
            StartCommitMismatchOwner::SelfCancelled => "self",
            StartCommitMismatchOwner::SupersededByOther => "other",
        }
    }
}

fn classify_start_failure_cleanup(
    state: &RecordingState,
    binding_id: &str,
    prepare_token: PrepareToken,
) -> StartFailureCleanupAction {
    match state {
        RecordingState::Preparing {
            binding_id: active_binding,
            prepare_token: active_prepare_token,
        } if active_binding == binding_id && *active_prepare_token == prepare_token => {
            StartFailureCleanupAction::ClearPreparingAndCloseStream
        }
        RecordingState::Idle => StartFailureCleanupAction::CloseStream,
        RecordingState::Preparing { .. }
        | RecordingState::CancellingPrepare { .. }
        | RecordingState::Recording { .. } => {
            StartFailureCleanupAction::Noop
        }
    }
}

fn should_stop_stream_for_start_cleanup(
    mode: &MicrophoneMode,
    cleanup_action: StartFailureCleanupAction,
) -> bool {
    matches!(mode, MicrophoneMode::OnDemand)
        && matches!(
            cleanup_action,
            StartFailureCleanupAction::ClearPreparingAndCloseStream
                | StartFailureCleanupAction::CloseStream
        )
}

fn map_recorder_start_error(
    err: RecorderStartError,
    cleanup_action: StartFailureCleanupAction,
) -> RecordingStartFailure {
    match err {
        RecorderStartError::CaptureReadyTimeout(timeout) => {
            RecordingStartFailure::CaptureReadyTimeout(timeout)
        }
        RecorderStartError::SupersededByNewStart => {
            RecordingStartFailure::StartAbandonedDueToSupersededState(
                "recorder_start_superseded_by_new_start".to_string(),
            )
        }
        RecorderStartError::CommandChannelUnavailable => RecordingStartFailure::RecorderUnavailable,
        RecorderStartError::CommandSendFailed | RecorderStartError::WorkerDisconnected => {
            if cleanup_action == StartFailureCleanupAction::Noop {
                RecordingStartFailure::StartAbandonedDueToSupersededState(err.to_string())
            } else {
                RecordingStartFailure::StartCommandFailed(err.to_string())
            }
        }
        RecorderStartError::UnexpectedAcknowledgement { .. } => {
            RecordingStartFailure::StartCommandFailed(err.to_string())
        }
    }
}

fn classify_start_commit_mismatch(
    state: &RecordingState,
    binding_id: &str,
    prepare_token: PrepareToken,
) -> Option<(StateMismatchKind, StartCommitMismatchOwner)> {
    match state {
        RecordingState::Preparing {
            binding_id: active_binding,
            prepare_token: active_prepare_token,
        } if active_binding == binding_id && *active_prepare_token == prepare_token => None,
        RecordingState::Preparing {
            binding_id: active_binding,
            prepare_token: active_prepare_token,
        } if active_binding == binding_id => Some((
            StateMismatchKind::SupersededByNewPrepareToken {
                active_prepare_token: *active_prepare_token,
                requested_prepare_token: prepare_token,
            },
            StartCommitMismatchOwner::SupersededByOther,
        )),
        RecordingState::CancellingPrepare {
            binding_id: active_binding,
            prepare_token: active_prepare_token,
        } => Some((
            StateMismatchKind::CancellingPrepare {
                active_binding: active_binding.clone(),
                active_prepare_token: *active_prepare_token,
            },
            StartCommitMismatchOwner::SelfCancelled,
        )),
        RecordingState::Idle => Some((
            StateMismatchKind::IdleBeforeCommit,
            StartCommitMismatchOwner::SelfCancelled,
        )),
        other => Some((
            StateMismatchKind::StateChangedBeforeCommit {
                state: format!("{other:?}"),
            },
            StartCommitMismatchOwner::SupersededByOther,
        )),
    }
}

fn begin_prepare_cancellation(
    state: &mut RecordingState,
    binding_filter: Option<&str>,
) -> Option<(String, PrepareToken)> {
    match state {
        RecordingState::Preparing {
            binding_id,
            prepare_token,
        } => {
            if binding_filter.is_some_and(|requested| requested != binding_id) {
                return None;
            }
            let cancelled_binding = binding_id.clone();
            let cancelled_token = *prepare_token;
            *state = RecordingState::CancellingPrepare {
                binding_id: cancelled_binding.clone(),
                prepare_token: cancelled_token,
            };
            Some((cancelled_binding, cancelled_token))
        }
        _ => None,
    }
}

fn finalize_prepare_cancellation(
    state: &mut RecordingState,
    binding_id: &str,
    prepare_token: PrepareToken,
) -> bool {
    match state {
        RecordingState::CancellingPrepare {
            binding_id: active_binding,
            prepare_token: active_token,
        } if active_binding == binding_id && *active_token == prepare_token => {
            *state = RecordingState::Idle;
            true
        }
        _ => false,
    }
}

/* ──────────────────────────────────────────────────────────────── */

fn create_audio_recorder(
    vad_path: &str,
    app_handle: &tauri::AppHandle,
) -> Result<AudioRecorder, anyhow::Error> {
    let silero = SileroVad::new(vad_path, 0.3)
        .map_err(|e| anyhow::anyhow!("Failed to create SileroVad: {}", e))?;
    let smoothed_vad = SmoothedVad::new(Box::new(silero), 15, 15, 2);

    // Recorder with VAD plus a spectrum-level callback that forwards updates to the frontend.
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

    // Locking invariant: manager mutexes should be acquired one-at-a-time.
    // Do not nest these locks; release before blocking or slow operations.
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
    
    /// The name of the device currently opened in the stream (for logging)
    current_device_name: Arc<Mutex<Option<String>>>,
    /// Short-lived cache for input device enumeration to speed first trigger after startup.
    device_cache:
        Arc<Mutex<Option<(Instant, Vec<crate::audio_toolkit::audio::CpalDeviceInfo>)>>>,
    next_prepare_token: Arc<AtomicU64>,
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
            current_device_name: Arc::new(Mutex::new(None)),
            device_cache: Arc::new(Mutex::new(None)),
            next_prepare_token: Arc::new(AtomicU64::new(1)),
        };

        // Always-on?  Open immediately.
        if matches!(mode, MicrophoneMode::AlwaysOn) {
            manager.start_microphone_stream()?;
        }

        Ok(manager)
    }

    /* ---------- helper methods --------------------------------------------- */

    const DEVICE_CACHE_TTL: Duration = Duration::from_secs(5);

    fn update_device_cache(&self, devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>) {
        *self.device_cache.lock().unwrap() = Some((Instant::now(), devices));
    }

    fn get_fresh_device_cache(
        &self,
    ) -> Option<(Duration, Vec<crate::audio_toolkit::audio::CpalDeviceInfo>)> {
        let cache = self.device_cache.lock().unwrap();
        let (cached_at, devices) = cache.as_ref()?;
        let age = cached_at.elapsed();
        if age > Self::DEVICE_CACHE_TTL {
            return None;
        }
        Some((age, devices.clone()))
    }

    pub fn prime_input_device_cache(&self) {
        let manager = self.clone();
        std::thread::spawn(move || {
            let start_time = Instant::now();
            match list_input_devices() {
                Ok(devices) => {
                    let count = devices.len();
                    manager.update_device_cache(devices);
                    info!(
                        "[TIMING] Primed input device cache in {:?} ({} devices)",
                        start_time.elapsed(),
                        count
                    );
                }
                Err(e) => {
                    debug!("Input device cache prime skipped: {}", e);
                }
            }
        });
    }

    /// Get the effective microphone device from a pre-fetched device list.
    /// This avoids calling list_input_devices() which is slow during failover.
    fn get_effective_device_from_list(
        &self,
        settings: &AppSettings,
        devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>,
    ) -> Option<(cpal::Device, String)> {
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
                .map(|d| (d.device, d.name));
        }

        // Logic for handling standard selection vs Default
        if let Some(name) = &settings.selected_microphone {
            // "default" / "Default" means use system default (same as None)
            if !name.eq_ignore_ascii_case("default") {
                // User explicitly selected a microphone -> Use it strictly
                return devices
                    .into_iter()
                    .find(|d| d.name == *name)
                    .map(|d| (d.device, d.name));
            }
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
                    return Some((builtin.device.clone(), builtin.name.clone()));
                }
                
                // No built-in found — try any non-Bluetooth, non-virtual device
                let non_bt_mic = devices.iter().find(|d| {
                    !crate::audio_device_info::is_device_bluetooth(&d.name)
                    && !crate::audio_device_info::is_device_virtual(&d.name)
                    && !crate::audio_device_info::is_device_continuity_camera(&d.name)
                });
                
                if let Some(alt) = non_bt_mic {
                    info!("No Built-in mic found, using non-Bluetooth alternative: '{}'", alt.name);
                    return Some((alt.device.clone(), alt.name.clone()));
                }
                
                info!("No non-Bluetooth microphone found. Falling back to Bluetooth default.");
            }
        }
        
        // Standard default behavior if not Bluetooth or no fallback found
        devices
            .into_iter()
            .find(|d| d.is_default)
            .map(|d| (d.device, d.name))
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

    /// Fast pre-start hint for showing the "connecting microphone" overlay.
    /// Uses only explicit settings (no CPAL device enumeration).
    pub fn should_show_connecting_overlay_pre_start(&self) -> bool {
        let Some(device_name) = self.get_selected_device_name_fast() else {
            return false;
        };

        // "Default" means no explicit device selection; avoid expensive detection here.
        if device_name.eq_ignore_ascii_case("default") {
            return false;
        }

        let is_bt = crate::audio_device_info::is_device_bluetooth(&device_name);
        info!(
            device = device_name,
            is_bluetooth = is_bt,
            method = "settings_fast_prestart",
            "Bluetooth device check"
        );
        is_bt
    }

    /// Determine Bluetooth status from the currently opened stream device.
    /// This reflects the actual device used for recording without extra enumeration.
    pub fn is_active_stream_bluetooth(&self) -> bool {
        // Only consider active-stream Bluetooth checks when the stream is open.
        if !*self.is_open.lock().unwrap() {
            return false;
        }

        if let Some(device_name) = self.current_device_name.lock().unwrap().clone() {
            let is_bt = crate::audio_device_info::is_device_bluetooth(&device_name);
            info!(
                device = device_name,
                is_bluetooth = is_bt,
                method = "active_stream",
                "Bluetooth device check"
            );
            return is_bt;
        }

        // Race fallback: stream is open but active device identity is not populated yet
        // (e.g., narrow prewarm/start overlap). Use explicit settings as a conservative hint.
        let Some(device_name) = self.get_selected_device_name_fast() else {
            return false;
        };
        if device_name.eq_ignore_ascii_case("default") {
            return false;
        }

        let is_bt = crate::audio_device_info::is_device_bluetooth(&device_name);
        info!(
            device = device_name,
            is_bluetooth = is_bt,
            method = "active_stream_settings_fallback",
            "Bluetooth device check"
        );
        is_bt
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
        // Only pre-warm if user has explicitly selected a Bluetooth device.
        // When using "Default", we prefer built-in mics, so no BT pre-warm needed.
        let explicitly_selected_bt = self.get_selected_device_name_fast()
            .map(|name| crate::audio_device_info::is_device_bluetooth(&name))
            .unwrap_or(false);
        
        if !explicitly_selected_bt {
            debug!("Skipping pre-warm: not an explicitly selected Bluetooth device");
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
                    let selected_target: Option<(cpal::Device, String)> = {
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
                                    .map(|d| (d.device, d.name)),
                                Err(_) => None
                            }
                        })
                    };

                    let selected_device = selected_target.as_ref().map(|(device, _)| device.clone());

                    if let Err(e) = rec.open(selected_device) {
                        debug!("Pre-warm failed to open stream: {}", e);
                        return;
                    }

                    // Capture active stream identity for downstream Bluetooth warmup decisions.
                    let active_name = selected_target
                        .as_ref()
                        .map(|(_, name)| name.clone())
                        .unwrap_or_else(|| "Default".to_string());
                    *current_device_name.lock().unwrap() = Some(active_name);
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
                *current_device_name.lock().unwrap() = None;
                
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

    fn persist_auto_switched_microphone(&self, previous: &str, current: &str) {
        // Persist only after a fresh device enumeration pass confirms the fallback.
        let mut settings = get_settings(&self.app_handle);
        settings.selected_microphone = Some(current.to_string());
        crate::settings::write_settings(&self.app_handle, settings);

        info!(
            "Emitting audio-device-auto-switched event: {} -> {}",
            previous, current
        );
        let _ = self.app_handle.emit(
            "audio-device-auto-switched",
            serde_json::json!({
                "previous": previous,
                "current": current
            }),
        );

        let _ = self
            .app_handle
            .notification()
            .builder()
            .title("Microphone Changed")
            .body(&format!(
                "Switched to {} - {} is disconnected.",
                current, previous
            ))
            .show();
    }

    fn resolve_device_open_target(
        &self,
        settings: &AppSettings,
        target_device_name: &str,
        devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>,
        allow_persist_fallback: bool,
    ) -> (Option<cpal::Device>, String) {
        let selected_device = self.get_effective_device_from_list(settings, devices.clone());
        let has_explicit_device = settings
            .selected_microphone
            .as_ref()
            .is_some_and(|name| !name.eq_ignore_ascii_case("default"));

        if selected_device.is_none() && has_explicit_device {
            // Selected device not found in device list - it's disconnected
            // Trigger fallback BEFORE attempting to open
            warn!(
                "Selected device '{}' not found in available devices - triggering fallback",
                target_device_name
            );

            if let Some((fallback_name, fallback_device)) =
                self.find_fallback_device_from_list(target_device_name, devices.clone())
            {
                info!("Switching to fallback device: {}", fallback_name);

                if allow_persist_fallback {
                    self.persist_auto_switched_microphone(target_device_name, &fallback_name);
                } else {
                    info!(
                        "Deferred fallback persistence for '{}' -> '{}' until a fresh re-enumeration confirms topology",
                        target_device_name, fallback_name
                    );
                }

                (Some(fallback_device), fallback_name)
            } else {
                // No fallback found - will try default device
                warn!("No fallback device found, attempting to use system default");
                (None, "Default".to_string())
            }
        } else if let Some((selected_dev, selected_name)) = selected_device {
            // Device found - proceed normally
            (Some(selected_dev), selected_name)
        } else {
            // No specific device resolved — pick best available to avoid cpal
            // defaulting to a Bluetooth device the OS just switched to
            let best = devices
                .iter()
                .find(|d| crate::audio_device_info::is_device_builtin(&d.name))
                .or_else(|| {
                    devices.iter().find(|d| {
                        !crate::audio_device_info::is_device_bluetooth(&d.name)
                            && !crate::audio_device_info::is_device_virtual(&d.name)
                            && !crate::audio_device_info::is_device_continuity_camera(&d.name)
                    })
                })
                .or_else(|| devices.iter().find(|d| d.is_default));

            match best {
                Some(dev) => {
                    info!("No device resolved, using best available: '{}'", dev.name);
                    (Some(dev.device.clone()), dev.name.clone())
                }
                None => (None, "Default".to_string()),
            }
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

        // ══════════════════════════════════════════════════════════════════════
        // PHASE 2: Device enumeration OUTSIDE of locks (this is the slow part)
        // ══════════════════════════════════════════════════════════════════════
        let enum_start = Instant::now();
        let (mut devices, mut used_cached_devices) =
            if let Some((cache_age, cached_devices)) = self.get_fresh_device_cache() {
                info!(
                    "[TIMING] Device enumeration skipped via cache (age: {:?}, {} devices)",
                    cache_age,
                    cached_devices.len()
                );
                (cached_devices, true)
            } else {
                let enumerated = list_input_devices()
                    .map_err(|e| anyhow::anyhow!("Failed to enumerate devices: {}", e))?;
                info!(
                    "[TIMING] Device enumeration completed in {:?} ({} devices)",
                    enum_start.elapsed(),
                    enumerated.len()
                );
                self.update_device_cache(enumerated.clone());
                (enumerated, false)
            };

        // Get settings and find the target device (still outside locks)
        let settings = get_settings(&self.app_handle);
        // Determine failed device name for potential fallback logging
        let target_device_name = settings
            .selected_microphone
            .clone()
            .unwrap_or_else(|| "Default".to_string());
        debug!("Target device: {}", target_device_name);
        let has_explicit_device = settings
            .selected_microphone
            .as_ref()
            .is_some_and(|name| !name.eq_ignore_ascii_case("default"));

        // Never trust cached topology when an explicit device appears missing.
        // Refresh before making fallback/persistence decisions.
        if used_cached_devices
            && has_explicit_device
            && self
                .get_effective_device_from_list(&settings, devices.clone())
                .is_none()
        {
            let refresh_start = Instant::now();
            info!(
                "Selected device '{}' missing in cached snapshot, refreshing topology before fallback",
                target_device_name
            );
            let refreshed = list_input_devices()
                .map_err(|e| anyhow::anyhow!("Failed to refresh devices: {}", e))?;
            info!(
                "[TIMING] Device enumeration refreshed in {:?} ({} devices) after cached selected-device miss",
                refresh_start.elapsed(),
                refreshed.len()
            );
            self.update_device_cache(refreshed.clone());
            devices = refreshed;
            used_cached_devices = false;
        }

        let (device_to_open, mut active_device_name) = self.resolve_device_open_target(
            &settings,
            &target_device_name,
            devices.clone(),
            !used_cached_devices,
        );

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
            )?);
        }
        if let Some(rec) = recorder_opt.as_mut() {
            // First attempt to open with the target device (already resolved via fallback if needed)
            if let Err(e) = rec.open(device_to_open.clone()) {
                if used_cached_devices {
                    warn!(
                        "Failed to open recorder from cached topology: {}. Retrying once with fresh enumeration",
                        e
                    );
                    let refresh_start = Instant::now();
                    let refreshed = list_input_devices().map_err(|refresh_err| {
                        anyhow::anyhow!(
                            "Failed to refresh devices after cached open failure (initial error: {}, refresh error: {})",
                            e,
                            refresh_err
                        )
                    })?;
                    info!(
                        "[TIMING] Device enumeration refreshed in {:?} ({} devices) after open failure",
                        refresh_start.elapsed(),
                        refreshed.len()
                    );
                    self.update_device_cache(refreshed.clone());

                    let (retry_device_to_open, retry_active_device_name) =
                        self.resolve_device_open_target(
                            &settings,
                            &target_device_name,
                            refreshed,
                            true,
                        );

                    if let Err(retry_e) = rec.open(retry_device_to_open) {
                        error!(
                            "Failed to open recorder after fresh retry (initial error: {}, retry error: {})",
                            e, retry_e
                        );
                        return Err(anyhow::anyhow!("Failed to open microphone: {}", retry_e));
                    }
                    active_device_name = retry_active_device_name;
                } else {
                    error!("Failed to open recorder: {}", e);
                    return Err(anyhow::anyhow!("Failed to open microphone: {}", e));
                }
            }

            // Successfully opened - track the active device
            *self.current_device_name.lock().unwrap() = Some(active_device_name.clone());
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
        
        // Filter candidates
        let mut candidates: Vec<_> = devices
            .into_iter()
            .filter(|d| {
                // Exclude the failed device
                if d.name == failed_device_name {
                    return false;
                }
                
                // Exclude virtual/phantom devices
                if crate::audio_device_info::is_device_virtual(&d.name) {
                    return false;
                }
                
                // Exclude Continuity Camera (iPhone) microphones - unreliable
                if crate::audio_device_info::is_device_continuity_camera(&d.name) {
                    debug!("Excluding '{}' from fallback candidates (Continuity Camera)", d.name);
                    return false;
                }
                
                true
            })
            .collect();

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
        *self.current_device_name.lock().unwrap() = None;
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

    pub fn prepare_recording(&self, binding_id: &str) -> Option<PrepareToken> {
        let mut state = self.state.lock().unwrap();
        if let RecordingState::Idle = *state {
            let prepare_token = self.next_prepare_token.fetch_add(1, Ordering::Relaxed);
            *state = RecordingState::Preparing {
                binding_id: binding_id.to_string(),
                prepare_token,
            };
            debug!(
                "Prepared recording for binding {} (prepare_token={})",
                binding_id, prepare_token
            );
            Some(prepare_token)
        } else {
            debug!("Cannot prepare recording: state is not Idle (current: {:?})", *state);
            None
        }
    }

    fn cancel_preparing_recording(&self, binding_filter: Option<&str>) -> bool {
        let cancellation = {
            let mut state = self.state.lock().unwrap();
            begin_prepare_cancellation(&mut state, binding_filter)
        };

        let Some((binding_id, prepare_token)) = cancellation else {
            return false;
        };

        // Cleanup runs after publishing a non-Idle transient state so new prepares
        // cannot race between cancellation intent and stream teardown.
        if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
            self.stop_microphone_stream();
        }

        let finalized = {
            let mut state = self.state.lock().unwrap();
            finalize_prepare_cancellation(&mut state, &binding_id, prepare_token)
        };

        if !finalized {
            warn!(
                "Prepare cancellation finalization skipped due to state ownership change (binding='{}', token={})",
                binding_id, prepare_token
            );
        }

        true
    }

    pub fn try_start_recording(
        &self,
        binding_id: &str,
        session_id: &str,
        prepare_token: PrepareToken,
    ) -> RecordingStartOutcome {
        // Note: Microphone permission is checked in TranscribeAction::start() before
        // the overlay is shown. This allows for better UX - we can show a modal dialog
        // instead of the overlay when permission is denied.

        // Phase A: validate precondition quickly under lock, then release.
        {
            let state = self.state.lock().unwrap();
            match &*state {
                RecordingState::Preparing {
                    binding_id: active_binding,
                    prepare_token: active_prepare_token,
                } if active_binding == binding_id && *active_prepare_token == prepare_token => {
                    // Expected state.
                }
                RecordingState::Preparing {
                    binding_id: active_binding,
                    prepare_token: active_prepare_token,
                } if active_binding == binding_id => {
                    debug!(
                        "try_start_recording aborted: prepare token superseded (active={}, requested={})",
                        active_prepare_token, prepare_token
                    );
                    return RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(
                        StateMismatchKind::SupersededByNewPrepareToken {
                            active_prepare_token: *active_prepare_token,
                            requested_prepare_token: prepare_token,
                        },
                    ));
                }
                RecordingState::Preparing {
                    binding_id: active_binding,
                    ..
                } => {
                    debug!(
                        "try_start_recording aborted: preparing for different binding '{}', not '{}'",
                        active_binding, binding_id
                    );
                    return RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(
                        StateMismatchKind::PreparingForDifferentBinding {
                            active_binding: active_binding.clone(),
                        },
                    ));
                }
                RecordingState::Idle => {
                    debug!(
                        "try_start_recording aborted: state is Idle (stop was called during prepare)"
                    );
                    return RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(
                        StateMismatchKind::IdleDuringPrepare,
                    ));
                }
                RecordingState::CancellingPrepare {
                    binding_id: active_binding,
                    prepare_token: active_prepare_token,
                } => {
                    debug!(
                        "try_start_recording aborted: prepare cancellation in progress for binding '{}' (token={})",
                        active_binding, active_prepare_token
                    );
                    return RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(
                        StateMismatchKind::CancellingPrepare {
                            active_binding: active_binding.clone(),
                            active_prepare_token: *active_prepare_token,
                        },
                    ));
                }
                other => {
                    error!("try_start_recording aborted: state changed to {:?}", other);
                    return RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(
                        StateMismatchKind::UnexpectedState {
                            state: format!("{other:?}"),
                        },
                    ));
                }
            }
        }

        let cleanup_failed_start = || -> StartFailureCleanupAction {
            let cleanup_action = {
                let mut state = self.state.lock().unwrap();
                let action = classify_start_failure_cleanup(&state, binding_id, prepare_token);
                if matches!(
                    action,
                    StartFailureCleanupAction::ClearPreparingAndCloseStream
                ) {
                    *state = RecordingState::Idle;
                }
                action
            };

            let should_close_stream = {
                let mode = self.mode.lock().unwrap();
                should_stop_stream_for_start_cleanup(&mode, cleanup_action)
            };
            if should_close_stream {
                self.stop_microphone_stream();
            }
            cleanup_action
        };

        // Phase B: blocking/slow operations outside the state lock.
        if !crate::backup_restore::ensure_transcription_start_allowed(&self.app_handle) {
            warn!(
                "Blocking recording start for '{}' because backup/restore maintenance mode is active",
                binding_id
            );
            let _ = cleanup_failed_start();
            return RecordingStartOutcome::Failed(RecordingStartFailure::MaintenanceMode);
        }

        if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
            if let Err(e) = self.start_microphone_stream() {
                error!("Failed to open microphone stream: {e}");
                let _ = cleanup_failed_start();
                return RecordingStartOutcome::Failed(RecordingStartFailure::StreamOpenFailed(
                    e.to_string(),
                ));
            }
        }

        let is_stream_open = *self.is_open.lock().unwrap();
        if is_stream_open {
            let active_device_name = self.current_device_name.lock().unwrap().clone();
            info!(
                session = session_id,
                binding = binding_id,
                device = active_device_name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                event_code = "stream_open_ready",
                "Microphone stream ready before capture-ready wait"
            );
        } else {
            warn!(
                session = session_id,
                binding = binding_id,
                event_code = "stream_not_yet_open",
                owner = "self",
                "Microphone stream not open yet; continuing to wait for capture-ready"
            );
        }

        const CAPTURE_READY_TIMEOUT: Duration = Duration::from_millis(500);

        let start_wait: Option<Result<RecorderStartWait, RecorderStartError>> = {
            let recorder_guard = self.recorder.lock().unwrap();
            recorder_guard
                .as_ref()
                .map(|rec| rec.begin_start_blocking())
        };
        let Some(start_wait) = start_wait else {
            error!("Recorder not available");
            let _ = cleanup_failed_start();
            return RecordingStartOutcome::Failed(RecordingStartFailure::RecorderUnavailable);
        };
        let capture_ready_wait_started = Instant::now();
        let start_result = start_wait.and_then(|wait| wait.wait(CAPTURE_READY_TIMEOUT));

        match start_result {
            Ok(()) => {
                let capture_ready_latency = capture_ready_wait_started.elapsed();
                let active_device_name = self.current_device_name.lock().unwrap().clone();

                // Phase C: commit start atomically under lock.
                let commit_mismatch = {
                    let mut state = self.state.lock().unwrap();
                    match classify_start_commit_mismatch(&state, binding_id, prepare_token) {
                        None => {
                            *self.is_recording.lock().unwrap() = true;
                            *state = RecordingState::Recording {
                                binding_id: binding_id.to_string(),
                                session_id: session_id.to_string(),
                            };
                            None
                        }
                        Some(mismatch) => Some(mismatch),
                    }
                };

                if let Some((mismatch_kind, owner)) = commit_mismatch {
                    let detail = mismatch_kind.details();
                    let owner_label = owner.as_str();
                    debug_assert_eq!(owner_label, mismatch_kind.owner().as_str());
                    warn!(
                        session = session_id,
                        binding = binding_id,
                        owner = owner_label,
                        event_code = "recording_start_failed",
                        detail = detail,
                        "Recording start acknowledged but state no longer allows commit; cancelling start"
                    );

                    if owner == StartCommitMismatchOwner::SelfCancelled {
                        if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                            if let Err(err) = rec.stop() {
                                let is_channel_unavailable = err
                                    .downcast_ref::<std::io::Error>()
                                    .is_some_and(|io_err| {
                                        io_err.kind() == std::io::ErrorKind::NotConnected
                                    });
                                if is_channel_unavailable {
                                    debug!(
                                        session = session_id,
                                        binding = binding_id,
                                        "Recorder stop skipped during self-cancel cleanup: command channel unavailable"
                                    );
                                } else {
                                    warn!(
                                        session = session_id,
                                        binding = binding_id,
                                        error = err.to_string(),
                                        "Recorder stop failed during self-cancel cleanup"
                                    );
                                }
                            }
                        }
                        let should_close_stream = {
                            let mode = self.mode.lock().unwrap();
                            should_stop_stream_for_start_cleanup(
                                &mode,
                                StartFailureCleanupAction::CloseStream,
                            )
                        };
                        if should_close_stream {
                            self.stop_microphone_stream();
                        }
                        return RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(
                            mismatch_kind,
                        ));
                    }

                    return RecordingStartOutcome::Failed(
                        RecordingStartFailure::StartAbandonedDueToSupersededState(detail),
                    );
                }

                // Start recording timer
                let start_time = Instant::now();
                *self.recording_start_time.lock().unwrap() = Some(start_time);
                self.start_recording_timer(binding_id.to_string());

                // Mark that we've successfully recorded (for first-trigger detection)
                self.mark_recording_started();

                info!(
                    session = session_id,
                    binding = binding_id,
                    latency_ms = capture_ready_latency.as_millis(),
                    device = active_device_name.clone().unwrap_or_else(|| "unknown".to_string()),
                    event_code = "capture_ready_ack",
                    "Recording capture-ready acknowledgement received"
                );

                RecordingStartOutcome::Started(RecordingStartSuccess {
                    capture_ready_latency,
                    active_device_name,
                })
            }
            Err(err) => {
                let cleanup_action = cleanup_failed_start();
                let failure = map_recorder_start_error(err, cleanup_action);

                match &failure {
                    RecordingStartFailure::CaptureReadyTimeout(timeout) => {
                        error!(
                            session = session_id,
                            binding = binding_id,
                            timeout_ms = timeout.as_millis(),
                            event_code = "recording_start_failed",
                            "Capture-ready acknowledgement timed out"
                        );
                    }
                    RecordingStartFailure::RecorderUnavailable => {
                        error!(
                            session = session_id,
                            binding = binding_id,
                            event_code = "recording_start_failed",
                            "Recorder command channel unavailable before capture-ready"
                        );
                    }
                    RecordingStartFailure::StartCommandFailed(detail) => {
                        error!(
                            session = session_id,
                            binding = binding_id,
                            error = detail,
                            event_code = "recording_start_failed",
                            "Recorder start command failed before capture-ready"
                        );
                    }
                    RecordingStartFailure::StartAbandonedDueToSupersededState(detail) => {
                        warn!(
                            session = session_id,
                            binding = binding_id,
                            owner = failure.owner().as_str(),
                            detail = detail,
                            event_code = "recording_start_failed",
                            "Recorder start attempt superseded by a newer start request"
                        );
                    }
                    RecordingStartFailure::StateMismatch(_)
                    | RecordingStartFailure::MaintenanceMode
                    | RecordingStartFailure::StreamOpenFailed(_) => {
                        unreachable!(
                            "Typed recorder start errors only map to timeout/superseded/recorder/start-command failures"
                        );
                    }
                }

                RecordingStartOutcome::Failed(failure)
            }
        }
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

    pub fn stop_recording(&self, binding_id: &str) -> Option<StoppedRecording> {
        if self.cancel_preparing_recording(Some(binding_id)) {
            debug!("stop_recording called while Preparing. Cancelling start.");
            return None;
        }

        let mut state = self.state.lock().unwrap();

        match *state {
            RecordingState::Recording {
                binding_id: ref active,
                session_id: _,
            } if active == binding_id => {
                *state = RecordingState::Idle;
                drop(state);

                let recording_duration_ms = {
                    let recording_start = *self.recording_start_time.lock().unwrap();
                    recording_start
                        .map(|start| start.elapsed().as_millis() as i64)
                        .unwrap_or_else(|| {
                            warn!(
                                "Recording start time missing at stop; falling back to speech duration for stats"
                            );
                            0
                        })
                };
                
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

                let speech_sample_count = samples.len();
                // debug!("Got {} samples", s_len);
                
                // Check for 0 samples - this likely means the audio stream died (e.g. device disconnected)
                // User will see no audio movement in the visualizer and can switch manually
                if speech_sample_count == 0 {
                    warn!("Recording yielded 0 samples - device may have stopped working. User should check audio visualizer and switch microphone if needed.");
                }

                Some(build_stopped_recording(samples, recording_duration_ms))
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
            RecordingState::CancellingPrepare { binding_id, .. } => Some(binding_id.clone()),
            RecordingState::Idle => None,
        }
    }

    /// Get the current session ID if recording is active
    pub fn get_current_session_id(&self) -> Option<String> {
        match &*self.state.lock().unwrap() {
            RecordingState::Recording { session_id, .. } => Some(session_id.clone()),
            RecordingState::Idle
            | RecordingState::Preparing { .. }
            | RecordingState::CancellingPrepare { .. } => None,
        }
    }




    /// Cancel any ongoing recording without returning audio samples
    pub fn cancel_recording(&self) {
        if self.cancel_preparing_recording(None) {
            debug!("Cancelled recording (while preparing)");
            return;
        }

        let mut state = self.state.lock().unwrap();

        match *state {
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
            RecordingState::Idle
            | RecordingState::Preparing { .. }
            | RecordingState::CancellingPrepare { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        begin_prepare_cancellation, build_stopped_recording, classify_start_commit_mismatch,
        classify_start_failure_cleanup, finalize_prepare_cancellation, map_recorder_start_error,
        should_stop_stream_for_start_cleanup, MicrophoneMode, PrepareToken, RecordingStartFailure, RecordingState,
        StateMismatchKind,
        StartCommitMismatchOwner, StartFailureCleanupAction, WHISPER_SAMPLE_RATE,
    };
    use crate::audio_toolkit::audio::RecorderStartError;
    use std::time::Duration;

    #[test]
    fn short_clip_padding_does_not_change_duration_fields() {
        let short_speech_samples = vec![0.1_f32; 8_000];
        let stopped = build_stopped_recording(short_speech_samples, 1_500);

        assert_eq!(stopped.samples_for_transcription.len(), WHISPER_SAMPLE_RATE * 5 / 4);
        assert_eq!(stopped.speech_duration_ms, 500);
        assert_eq!(stopped.recording_duration_ms, 1_500);
    }

    #[test]
    fn speech_duration_is_clamped_to_recording_duration() {
        let speech_samples = vec![0.1_f32; 16_000];
        let stopped = build_stopped_recording(speech_samples, 400);

        assert_eq!(stopped.speech_duration_ms, 400);
        assert_eq!(stopped.recording_duration_ms, 400);
    }

    #[test]
    fn cleanup_classification_clears_matching_preparing_state() {
        let prepare_token: PrepareToken = 10;
        let state = RecordingState::Preparing {
            binding_id: "transcribe".to_string(),
            prepare_token,
        };
        let action = classify_start_failure_cleanup(&state, "transcribe", prepare_token);
        assert_eq!(
            action,
            StartFailureCleanupAction::ClearPreparingAndCloseStream
        );
    }

    #[test]
    fn cleanup_classification_closes_stream_when_state_is_idle() {
        let action = classify_start_failure_cleanup(&RecordingState::Idle, "transcribe", 10);
        assert_eq!(action, StartFailureCleanupAction::CloseStream);
    }

    #[test]
    fn cleanup_stream_policy_on_demand_allows_stream_teardown_actions() {
        assert!(should_stop_stream_for_start_cleanup(
            &MicrophoneMode::OnDemand,
            StartFailureCleanupAction::ClearPreparingAndCloseStream
        ));
        assert!(should_stop_stream_for_start_cleanup(
            &MicrophoneMode::OnDemand,
            StartFailureCleanupAction::CloseStream
        ));
        assert!(!should_stop_stream_for_start_cleanup(
            &MicrophoneMode::OnDemand,
            StartFailureCleanupAction::Noop
        ));
    }

    #[test]
    fn cleanup_stream_policy_always_on_blocks_stream_teardown_actions() {
        assert!(!should_stop_stream_for_start_cleanup(
            &MicrophoneMode::AlwaysOn,
            StartFailureCleanupAction::ClearPreparingAndCloseStream
        ));
        assert!(!should_stop_stream_for_start_cleanup(
            &MicrophoneMode::AlwaysOn,
            StartFailureCleanupAction::CloseStream
        ));
        assert!(!should_stop_stream_for_start_cleanup(
            &MicrophoneMode::AlwaysOn,
            StartFailureCleanupAction::Noop
        ));
    }

    #[test]
    fn cleanup_classification_skips_when_other_work_is_active() {
        let preparing_other = RecordingState::Preparing {
            binding_id: "transcribe_handsfree".to_string(),
            prepare_token: 2,
        };
        let cancelling_other = RecordingState::CancellingPrepare {
            binding_id: "transcribe_handsfree".to_string(),
            prepare_token: 2,
        };
        let recording_other = RecordingState::Recording {
            binding_id: "transcribe_handsfree".to_string(),
            session_id: "abc12345".to_string(),
        };
        assert_eq!(
            classify_start_failure_cleanup(&preparing_other, "transcribe", 1),
            StartFailureCleanupAction::Noop
        );
        assert_eq!(
            classify_start_failure_cleanup(&cancelling_other, "transcribe", 1),
            StartFailureCleanupAction::Noop
        );
        assert_eq!(
            classify_start_failure_cleanup(&recording_other, "transcribe", 1),
            StartFailureCleanupAction::Noop
        );
    }

    #[test]
    fn idle_mismatch_performs_self_cleanup() {
        let mismatch = classify_start_commit_mismatch(&RecordingState::Idle, "transcribe", 1)
            .expect("idle state should produce self-cancelled mismatch");
        assert_eq!(
            mismatch,
            (
                StateMismatchKind::IdleBeforeCommit,
                StartCommitMismatchOwner::SelfCancelled,
            )
        );
    }

    #[test]
    fn commit_mismatch_with_other_owner_is_non_destructive() {
        let mismatch = classify_start_commit_mismatch(
            &RecordingState::Recording {
                binding_id: "transcribe_handsfree".to_string(),
                session_id: "abc12345".to_string(),
            },
            "transcribe",
            10,
        )
        .expect("different owner state should produce superseded mismatch");
        assert_eq!(mismatch.1, StartCommitMismatchOwner::SupersededByOther);
        match mismatch.0 {
            StateMismatchKind::StateChangedBeforeCommit { state } => {
                assert!(state.contains("Recording"));
            }
            other => panic!("unexpected mismatch kind: {other:?}"),
        }
    }

    #[test]
    fn cleanup_classification_is_noop_when_same_binding_has_newer_prepare_token() {
        let state = RecordingState::Preparing {
            binding_id: "transcribe".to_string(),
            prepare_token: 11,
        };
        assert_eq!(
            classify_start_failure_cleanup(&state, "transcribe", 10),
            StartFailureCleanupAction::Noop
        );
    }

    #[test]
    fn commit_mismatch_with_same_binding_and_different_token_is_other_owned() {
        let mismatch = classify_start_commit_mismatch(
            &RecordingState::Preparing {
                binding_id: "transcribe".to_string(),
                prepare_token: 11,
            },
            "transcribe",
            10,
        )
        .expect("different token must be treated as superseded");
        assert_eq!(mismatch.1, StartCommitMismatchOwner::SupersededByOther);
        assert_eq!(
            mismatch.0,
            StateMismatchKind::SupersededByNewPrepareToken {
                active_prepare_token: 11,
                requested_prepare_token: 10,
            }
        );
    }

    #[test]
    fn recorder_start_error_mapping_preserves_timeout_variant() {
        let failure = map_recorder_start_error(
            RecorderStartError::CaptureReadyTimeout(Duration::from_millis(500)),
            StartFailureCleanupAction::ClearPreparingAndCloseStream,
        );
        assert_eq!(
            failure,
            RecordingStartFailure::CaptureReadyTimeout(Duration::from_millis(500))
        );
    }

    #[test]
    fn recorder_start_error_mapping_treats_unavailable_channel_as_recorder_unavailable() {
        let failure = map_recorder_start_error(
            RecorderStartError::CommandChannelUnavailable,
            StartFailureCleanupAction::Noop,
        );
        assert_eq!(failure, RecordingStartFailure::RecorderUnavailable);
    }

    #[test]
    fn recorder_start_error_mapping_treats_superseded_start_as_abandoned() {
        let failure = map_recorder_start_error(
            RecorderStartError::SupersededByNewStart,
            StartFailureCleanupAction::ClearPreparingAndCloseStream,
        );
        assert_eq!(
            failure,
            RecordingStartFailure::StartAbandonedDueToSupersededState(
                "recorder_start_superseded_by_new_start".to_string()
            )
        );
    }

    #[test]
    fn recorder_start_error_mapping_uses_start_command_failed_for_self_owned_worker_errors() {
        let failure = map_recorder_start_error(
            RecorderStartError::WorkerDisconnected,
            StartFailureCleanupAction::CloseStream,
        );
        match failure {
            RecordingStartFailure::StartCommandFailed(detail) => {
                assert!(detail.contains("disconnected"));
            }
            other => panic!("unexpected mapping for worker error: {other:?}"),
        }
    }

    #[test]
    fn recorder_start_error_mapping_treats_worker_disconnect_as_abandoned_when_superseded() {
        let failure = map_recorder_start_error(
            RecorderStartError::WorkerDisconnected,
            StartFailureCleanupAction::Noop,
        );
        match failure {
            RecordingStartFailure::StartAbandonedDueToSupersededState(detail) => {
                assert!(detail.contains("disconnected"));
            }
            other => panic!("unexpected mapping for superseded worker disconnect: {other:?}"),
        }
    }

    #[test]
    fn recorder_start_error_mapping_treats_send_failure_as_abandoned_when_superseded() {
        let failure = map_recorder_start_error(
            RecorderStartError::CommandSendFailed,
            StartFailureCleanupAction::Noop,
        );
        match failure {
            RecordingStartFailure::StartAbandonedDueToSupersededState(detail) => {
                assert!(detail.contains("Failed to send start command"));
            }
            other => panic!("unexpected mapping for superseded send failure: {other:?}"),
        }
    }

    #[test]
    fn commit_mismatch_in_cancelling_prepare_is_self_owned() {
        let mismatch = classify_start_commit_mismatch(
            &RecordingState::CancellingPrepare {
                binding_id: "transcribe".to_string(),
                prepare_token: 11,
            },
            "transcribe",
            10,
        )
        .expect("cancelling state must reject stale commit as self-cancelled");
        assert_eq!(mismatch.1, StartCommitMismatchOwner::SelfCancelled);
        assert_eq!(
            mismatch.0,
            StateMismatchKind::CancellingPrepare {
                active_binding: "transcribe".to_string(),
                active_prepare_token: 11,
            }
        );
    }

    #[test]
    fn begin_prepare_cancellation_moves_state_to_transient_and_finalize_returns_idle() {
        let mut state = RecordingState::Preparing {
            binding_id: "transcribe".to_string(),
            prepare_token: 42,
        };

        let cancelled = begin_prepare_cancellation(&mut state, Some("transcribe"))
            .expect("matching prepare should enter cancellation state");
        assert_eq!(cancelled, ("transcribe".to_string(), 42));
        assert!(matches!(
            state,
            RecordingState::CancellingPrepare {
                ref binding_id,
                prepare_token: 42,
            } if binding_id == "transcribe"
        ));

        assert!(finalize_prepare_cancellation(&mut state, "transcribe", 42));
        assert!(matches!(state, RecordingState::Idle));
    }

    #[test]
    fn begin_prepare_cancellation_respects_binding_filter_and_blocks_reentry() {
        let mut state = RecordingState::Preparing {
            binding_id: "transcribe_handsfree".to_string(),
            prepare_token: 9,
        };

        assert_eq!(
            begin_prepare_cancellation(&mut state, Some("transcribe")),
            None,
            "different binding must not be cancelled"
        );
        assert_eq!(
            begin_prepare_cancellation(&mut state, Some("transcribe_handsfree")),
            Some(("transcribe_handsfree".to_string(), 9))
        );
        assert_eq!(
            begin_prepare_cancellation(&mut state, Some("transcribe_handsfree")),
            None,
            "cancellation is one-way until finalized"
        );
    }

    #[test]
    fn superseded_start_then_cancelled_new_prepare_never_reverts_to_new_owner() {
        let mut state = RecordingState::Preparing {
            binding_id: "transcribe".to_string(),
            prepare_token: 11,
        };

        let stale_commit = classify_start_commit_mismatch(&state, "transcribe", 10)
            .expect("older token must be treated as superseded");
        assert_eq!(stale_commit.1, StartCommitMismatchOwner::SupersededByOther);

        let cancelled = begin_prepare_cancellation(&mut state, Some("transcribe"))
            .expect("stop during preparing should enter cancellation state");
        assert_eq!(cancelled.1, 11);

        let stale_commit_during_cancel = classify_start_commit_mismatch(&state, "transcribe", 10)
            .expect("stale commit must still be rejected during cancellation");
        assert_eq!(
            stale_commit_during_cancel.1,
            StartCommitMismatchOwner::SelfCancelled
        );

        assert!(finalize_prepare_cancellation(&mut state, "transcribe", 11));
        assert!(matches!(state, RecordingState::Idle));
    }
}
