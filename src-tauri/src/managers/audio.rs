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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordingPrearmSource {
    FnKeyDown,
    ShortcutStart,
}

impl RecordingPrearmSource {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordingPrearmSource::FnKeyDown => "fn_key_down",
            RecordingPrearmSource::ShortcutStart => "shortcut_start",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamStartOutcome {
    AlreadyOpen,
    OpenedNow { stream_epoch: u64 },
    CancelledBeforeOpen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamOpenContext {
    UserTriggered,
    StartupPrewarm,
    Prearm { owner_token: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyResolutionMode {
    Warm,
    Cache,
    Fresh,
}

impl TopologyResolutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            TopologyResolutionMode::Warm => "warm",
            TopologyResolutionMode::Cache => "cache",
            TopologyResolutionMode::Fresh => "fresh",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamStartResult {
    pub outcome: StreamStartOutcome,
    pub resolution_mode: TopologyResolutionMode,
    pub resolution_reason: String,
}

#[derive(Clone)]
struct InputDeviceCacheEntry {
    cached_at: Instant,
    route_generation: Option<u64>,
    devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>,
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

fn should_auto_close_owned_warm_stream(
    mode: &MicrophoneMode,
    state: &RecordingState,
    is_recording: bool,
    is_open: bool,
    opened_by_owner: bool,
    opened_stream_epoch: Option<u64>,
    active_stream_epoch: u64,
) -> bool {
    matches!(mode, MicrophoneMode::OnDemand)
        && matches!(state, RecordingState::Idle)
        && !is_recording
        && is_open
        && opened_by_owner
        && matches!(
            opened_stream_epoch,
            Some(epoch) if epoch != 0 && epoch == active_stream_epoch
        )
}

fn should_auto_close_prearm_stream(
    mode: &MicrophoneMode,
    state: &RecordingState,
    is_recording: bool,
    is_open: bool,
    opened_by_prearm: bool,
    opened_stream_epoch: Option<u64>,
    active_stream_epoch: u64,
) -> bool {
    should_auto_close_owned_warm_stream(
        mode,
        state,
        is_recording,
        is_open,
        opened_by_prearm,
        opened_stream_epoch,
        active_stream_epoch,
    )
}

fn is_explicit_microphone_selection(selection: Option<&str>) -> bool {
    selection.is_some_and(|name| !name.eq_ignore_ascii_case("default"))
}

fn active_selection_for_cache_policy<'a>(
    selected_microphone: Option<&'a str>,
    clamshell_microphone: Option<&'a str>,
    clamshell_selection_active: bool,
) -> Option<&'a str> {
    if clamshell_selection_active {
        return clamshell_microphone;
    }

    selected_microphone
}

fn should_force_fresh_default_route_enumeration(
    active_selected_microphone: Option<&str>,
    route_monitor_active: bool,
    cached_route_generation: Option<u64>,
    current_route_generation: Option<u64>,
) -> bool {
    if is_explicit_microphone_selection(active_selected_microphone) {
        return false;
    }

    if !route_monitor_active {
        return true;
    }

    match (cached_route_generation, current_route_generation) {
        (Some(cached), Some(current)) => cached != current,
        _ => true,
    }
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
    /// Cached topology plus the route generation it was observed under.
    device_cache: Arc<Mutex<Option<InputDeviceCacheEntry>>>,
    /// Serialize stream-open work so prearm and recording start can reuse the same open.
    stream_start_lock: Arc<Mutex<()>>,
    /// Active stream epoch for ownership-safe warm-stream auto-close.
    active_stream_epoch: Arc<AtomicU64>,
    /// Monotonic stream epoch allocator.
    next_stream_epoch: Arc<AtomicU64>,
    /// Dedupes prearm ownership so only one warm-path open runs at a time.
    prearm_owner_token: Arc<AtomicU64>,
    next_prearm_owner_token: Arc<AtomicU64>,
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
            stream_start_lock: Arc::new(Mutex::new(())),
            active_stream_epoch: Arc::new(AtomicU64::new(0)),
            next_stream_epoch: Arc::new(AtomicU64::new(1)),
            prearm_owner_token: Arc::new(AtomicU64::new(0)),
            next_prearm_owner_token: Arc::new(AtomicU64::new(1)),
            next_prepare_token: Arc::new(AtomicU64::new(1)),
        };

        // Always-on?  Open immediately.
        if matches!(mode, MicrophoneMode::AlwaysOn) {
            manager.start_microphone_stream()?;
        }

        Ok(manager)
    }

    /* ---------- helper methods --------------------------------------------- */

    const DEVICE_CACHE_TTL: Duration = Duration::from_secs(60 * 10);
    const PREARM_GRACE_TIMEOUT: Duration = Duration::from_millis(900);

    fn update_device_cache(
        &self,
        devices: Vec<crate::audio_toolkit::audio::CpalDeviceInfo>,
        route_generation: Option<u64>,
    ) {
        *self.device_cache.lock().unwrap() = Some(InputDeviceCacheEntry {
            cached_at: Instant::now(),
            route_generation,
            devices,
        });
    }

    fn get_fresh_device_cache(
        &self,
    ) -> Option<(Duration, InputDeviceCacheEntry)> {
        let cache = self.device_cache.lock().unwrap();
        let entry = cache.as_ref()?;
        let age = entry.cached_at.elapsed();
        if age > Self::DEVICE_CACHE_TTL {
            return None;
        }
        Some((age, entry.clone()))
    }

    fn current_route_generation(&self) -> Option<u64> {
        if crate::audio_device_info::is_input_route_change_monitor_active() {
            Some(crate::audio_device_info::input_route_change_generation())
        } else {
            None
        }
    }

    fn is_clamshell_selection_active(settings: &AppSettings) -> bool {
        if settings.clamshell_microphone.is_none() {
            return false;
        }

        match clamshell::is_clamshell() {
            Ok(is_clamshell) => is_clamshell,
            Err(_) => false,
        }
    }

    fn active_selected_microphone_for_cache_policy<'a>(
        settings: &'a AppSettings,
    ) -> (bool, Option<&'a str>) {
        let clamshell_selection_active = Self::is_clamshell_selection_active(settings);
        let active_selection = active_selection_for_cache_policy(
            settings.selected_microphone.as_deref(),
            settings.clamshell_microphone.as_deref(),
            clamshell_selection_active,
        );
        (clamshell_selection_active, active_selection)
    }

    pub fn prime_input_device_cache(&self) {
        let manager = self.clone();
        std::thread::spawn(move || {
            let start_time = Instant::now();
            match list_input_devices() {
                Ok(devices) => {
                    let count = devices.len();
                    let route_generation = manager.current_route_generation();
                    manager.update_device_cache(devices, route_generation);
                    info!(
                        route_generation = route_generation.unwrap_or_default(),
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

    fn try_claim_prearm_owner(&self, source: RecordingPrearmSource, prearm_token: u64) -> bool {
        match self.prearm_owner_token.compare_exchange(
            0,
            prearm_token,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => true,
            Err(active_token) => {
                debug!(
                    source = source.as_str(),
                    requested_token = prearm_token,
                    active_token = active_token,
                    event_code = "prearm_skipped",
                    reason = "inflight",
                    "Prearm skipped because another open is already in-flight"
                );
                false
            }
        }
    }

    fn release_prearm_owner(
        &self,
        source: RecordingPrearmSource,
        prearm_token: u64,
        reason: &'static str,
    ) {
        match self.prearm_owner_token.compare_exchange(
            prearm_token,
            0,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                debug!(
                    source = source.as_str(),
                    prearm_token = prearm_token,
                    reason = reason,
                    event_code = "prearm_owner_released",
                    "Released prearm owner token"
                );
            }
            Err(active_token) => {
                debug!(
                    source = source.as_str(),
                    prearm_token = prearm_token,
                    active_token = active_token,
                    reason = reason,
                    event_code = "prearm_owner_release_skipped",
                    "Skipped prearm owner release because ownership already moved"
                );
            }
        }
    }

    fn prearm_open_allowed(&self, context: StreamOpenContext) -> bool {
        match context {
            StreamOpenContext::Prearm { owner_token } => {
                owner_token != 0
                    && self.prearm_owner_token.load(Ordering::SeqCst) == owner_token
            }
            StreamOpenContext::UserTriggered | StreamOpenContext::StartupPrewarm => true,
        }
    }

    pub fn kickoff_on_demand_prearm(&self, source: RecordingPrearmSource, trigger_id: &str) {
        if !matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
            debug!(
                source = source.as_str(),
                trigger_id = trigger_id,
                event_code = "prearm_skipped",
                reason = "always_on_mode",
                "Prearm skipped because microphone mode is always-on"
            );
            return;
        }

        let prearm_token = self.next_prearm_owner_token.fetch_add(1, Ordering::SeqCst);
        if !self.try_claim_prearm_owner(source, prearm_token) {
            return;
        }

        let manager = self.clone();
        let trigger_id = trigger_id.to_string();
        std::thread::spawn(move || {
            info!(
                source = source.as_str(),
                trigger_id = trigger_id,
                prearm_token = prearm_token,
                event_code = "prearm_requested",
                "Starting on-demand microphone prearm"
            );

            let opened_stream_epoch = match manager
                .start_microphone_stream_with_context(StreamOpenContext::Prearm {
                    owner_token: prearm_token,
                }) {
                Ok(result) => match result.outcome {
                    StreamStartOutcome::OpenedNow { stream_epoch } => {
                        info!(
                            source = source.as_str(),
                            trigger_id = trigger_id,
                            prearm_token = prearm_token,
                            stream_epoch = stream_epoch,
                            resolution_mode = result.resolution_mode.as_str(),
                            resolution_reason = result.resolution_reason.as_str(),
                            event_code = "prearm_completed",
                            "Warm path stream is ready"
                        );
                        Some(stream_epoch)
                    }
                    StreamStartOutcome::AlreadyOpen => {
                        debug!(
                            source = source.as_str(),
                            trigger_id = trigger_id,
                            prearm_token = prearm_token,
                            resolution_mode = result.resolution_mode.as_str(),
                            resolution_reason = result.resolution_reason.as_str(),
                            event_code = "prearm_skipped",
                            reason = "stream_already_open",
                            "Prearm skipped because the microphone stream is already open"
                        );
                        manager.release_prearm_owner(source, prearm_token, "stream_already_open");
                        return;
                    }
                    StreamStartOutcome::CancelledBeforeOpen => {
                        info!(
                            source = source.as_str(),
                            trigger_id = trigger_id,
                            prearm_token = prearm_token,
                            event_code = "prearm_cancelled",
                            reason = "owner_lost_before_open",
                            "Prearm cancelled before stream open"
                        );
                        manager
                            .release_prearm_owner(source, prearm_token, "owner_lost_before_open");
                        return;
                    }
                },
                Err(err) => {
                    warn!(
                        source = source.as_str(),
                        trigger_id = trigger_id,
                        prearm_token = prearm_token,
                        error = err.to_string(),
                        event_code = "prearm_cancelled",
                        reason = "stream_open_failed",
                        "Prearm failed while opening the microphone stream"
                    );
                    manager.release_prearm_owner(source, prearm_token, "stream_open_failed");
                    return;
                }
            };

            std::thread::sleep(Self::PREARM_GRACE_TIMEOUT);

            let (is_recording, is_open, state, active_stream_epoch, auto_closed) = {
                let _stream_start_guard = manager.stream_start_lock.lock().unwrap();
                let mode = manager.mode.lock().unwrap().clone();
                let is_recording = *manager.is_recording.lock().unwrap();
                let is_open = *manager.is_open.lock().unwrap();
                let state = manager.state.lock().unwrap().clone();
                let active_stream_epoch = manager.active_stream_epoch.load(Ordering::SeqCst);
                let auto_closed = if should_auto_close_prearm_stream(
                    &mode,
                    &state,
                    is_recording,
                    is_open,
                    opened_stream_epoch.is_some(),
                    opened_stream_epoch,
                    active_stream_epoch,
                ) {
                    manager.stop_microphone_stream_locked()
                } else {
                    false
                };
                (
                    is_recording,
                    is_open,
                    state,
                    active_stream_epoch,
                    auto_closed,
                )
            };

            if auto_closed {
                info!(
                    source = source.as_str(),
                    trigger_id = trigger_id,
                    prearm_token = prearm_token,
                    timeout_ms = Self::PREARM_GRACE_TIMEOUT.as_millis(),
                    opened_stream_epoch = opened_stream_epoch.unwrap_or_default(),
                    active_stream_epoch = active_stream_epoch,
                    event_code = "prearm_autoclosed",
                    "Closed unused warm stream after the prearm grace window"
                );
            } else {
                debug!(
                    source = source.as_str(),
                    trigger_id = trigger_id,
                    prearm_token = prearm_token,
                    timeout_ms = Self::PREARM_GRACE_TIMEOUT.as_millis(),
                    opened_stream_epoch = opened_stream_epoch.unwrap_or_default(),
                    active_stream_epoch = active_stream_epoch,
                    state = format!("{state:?}"),
                    is_recording = is_recording,
                    is_open = is_open,
                    event_code = "prearm_cancelled",
                    reason = "recording_progressed_or_stream_changed",
                    "Prearm auto-close skipped because recording progressed or stream ownership changed"
                );
            }

            manager.release_prearm_owner(source, prearm_token, "completed");
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

        info!("Pre-warming Bluetooth microphone in background");

        let manager = self.clone();
        std::thread::spawn(move || {
            let start_time = Instant::now();
            let opened_stream_epoch = match manager
                .start_microphone_stream_with_context(StreamOpenContext::StartupPrewarm)
            {
                Ok(result) => match result.outcome {
                    StreamStartOutcome::OpenedNow { stream_epoch } => {
                        info!(
                            stream_epoch = stream_epoch,
                            resolution_mode = result.resolution_mode.as_str(),
                            resolution_reason = result.resolution_reason.as_str(),
                            event_code = "prewarm_stream_ready",
                            "Pre-warm opened microphone stream"
                        );
                        Some(stream_epoch)
                    }
                    StreamStartOutcome::AlreadyOpen => {
                        debug!(
                            event_code = "prewarm_cancelled",
                            reason = "stream_already_open",
                            "Pre-warm skipped because microphone stream is already active"
                        );
                        return;
                    }
                    StreamStartOutcome::CancelledBeforeOpen => {
                        debug!(
                            event_code = "prewarm_cancelled",
                            reason = "cancelled_before_open",
                            "Pre-warm was cancelled before the stream opened"
                        );
                        return;
                    }
                },
                Err(err) => {
                    debug!("Pre-warm failed to open stream: {}", err);
                    return;
                }
            };

            info!(
                "Pre-warm: Bluetooth profile switch triggered in {:?}",
                start_time.elapsed()
            );

            std::thread::sleep(std::time::Duration::from_millis(500));

            let (is_recording, is_open, state, active_stream_epoch, auto_closed) = {
                let _stream_start_guard = manager.stream_start_lock.lock().unwrap();
                let mode = manager.mode.lock().unwrap().clone();
                let is_recording = *manager.is_recording.lock().unwrap();
                let is_open = *manager.is_open.lock().unwrap();
                let state = manager.state.lock().unwrap().clone();
                let active_stream_epoch = manager.active_stream_epoch.load(Ordering::SeqCst);
                let auto_closed = if should_auto_close_owned_warm_stream(
                    &mode,
                    &state,
                    is_recording,
                    is_open,
                    opened_stream_epoch.is_some(),
                    opened_stream_epoch,
                    active_stream_epoch,
                ) {
                    manager.stop_microphone_stream_locked()
                } else {
                    false
                };
                (
                    is_recording,
                    is_open,
                    state,
                    active_stream_epoch,
                    auto_closed,
                )
            };

            if auto_closed {
                info!(
                    opened_stream_epoch = opened_stream_epoch.unwrap_or_default(),
                    active_stream_epoch = active_stream_epoch,
                    event_code = "prewarm_timeout_autoclose",
                    "Pre-warm complete: Bluetooth microphone ready"
                );
            } else {
                debug!(
                    opened_stream_epoch = opened_stream_epoch.unwrap_or_default(),
                    active_stream_epoch = active_stream_epoch,
                    state = format!("{state:?}"),
                    is_recording = is_recording,
                    is_open = is_open,
                    event_code = "prewarm_cancelled",
                    reason = "recording_progressed_or_stream_changed",
                    "Pre-warm auto-close skipped because recording progressed or stream ownership changed"
                );
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

    pub fn start_microphone_stream(&self) -> Result<StreamStartResult, anyhow::Error> {
        self.start_microphone_stream_with_context(StreamOpenContext::UserTriggered)
    }

    fn start_microphone_stream_with_context(
        &self,
        context: StreamOpenContext,
    ) -> Result<StreamStartResult, anyhow::Error> {
        {
            let _stream_start_guard = self.stream_start_lock.lock().unwrap();
            if *self.is_open.lock().unwrap() {
                debug!("Microphone stream already active");
                return Ok(StreamStartResult {
                    outcome: StreamStartOutcome::AlreadyOpen,
                    resolution_mode: TopologyResolutionMode::Warm,
                    resolution_reason: "reused_open_stream".to_string(),
                });
            }
            if !self.prearm_open_allowed(context) {
                return Ok(StreamStartResult {
                    outcome: StreamStartOutcome::CancelledBeforeOpen,
                    resolution_mode: TopologyResolutionMode::Warm,
                    resolution_reason: "owner_lost_before_open".to_string(),
                });
            }
        }

        let start_time = Instant::now();
        info!("[TIMING] start_microphone_stream starting...");

        let enumerate_devices = |reason: &str| -> Result<Vec<crate::audio_toolkit::audio::CpalDeviceInfo>, anyhow::Error> {
            let enum_start = Instant::now();
            let enumerated = list_input_devices()
                .map_err(|e| anyhow::anyhow!("Failed to enumerate devices: {}", e))?;
            let route_generation = self.current_route_generation();
            info!(
                reason = reason,
                route_generation = route_generation.unwrap_or_default(),
                duration_ms = enum_start.elapsed().as_millis(),
                device_count = enumerated.len(),
                event_code = "cache_refresh_completed",
                "[TIMING] Device enumeration completed and cache refreshed"
            );
            self.update_device_cache(enumerated.clone(), route_generation);
            Ok(enumerated)
        };

        let settings = get_settings(&self.app_handle);
        let (clamshell_selection_active, active_selected_microphone) =
            Self::active_selected_microphone_for_cache_policy(&settings);
        let target_device_name = active_selected_microphone.unwrap_or("Default").to_string();
        let route_monitor_active = crate::audio_device_info::is_input_route_change_monitor_active();
        let current_route_generation = self.current_route_generation();

        let (
            mut devices,
            mut used_cached_devices,
            cached_route_generation,
            mut resolution_mode,
            mut resolution_reason,
        ) =
            if let Some((cache_age, cache_entry)) = self.get_fresh_device_cache() {
                let force_fresh_default_route = should_force_fresh_default_route_enumeration(
                    active_selected_microphone,
                    route_monitor_active,
                    cache_entry.route_generation,
                    current_route_generation,
                );

                if force_fresh_default_route {
                    let resolution_reason = if !route_monitor_active {
                        "default_route_monitor_unavailable".to_string()
                    } else {
                        "default_route_generation_changed".to_string()
                    };
                    (
                        enumerate_devices(&resolution_reason)?,
                        false,
                        cache_entry.route_generation,
                        TopologyResolutionMode::Fresh,
                        resolution_reason,
                    )
                } else {
                    let resolution_reason =
                        if is_explicit_microphone_selection(active_selected_microphone) {
                            "explicit_selection_cache_hit".to_string()
                        } else {
                            "default_route_confirmed".to_string()
                        };
                    info!(
                        age_ms = cache_age.as_millis(),
                        route_generation = cache_entry.route_generation.unwrap_or_default(),
                        device_count = cache_entry.devices.len(),
                        event_code = "cache_hit",
                        "[TIMING] Device enumeration skipped via cache"
                    );
                    (
                        cache_entry.devices,
                        true,
                        cache_entry.route_generation,
                        TopologyResolutionMode::Cache,
                        resolution_reason,
                    )
                }
            } else {
                let resolution_reason = "cache_miss_or_stale".to_string();
                (
                    enumerate_devices(&resolution_reason)?,
                    false,
                    None,
                    TopologyResolutionMode::Fresh,
                    resolution_reason,
                )
            };

        debug!(
            target_device = target_device_name,
            active_selection = active_selected_microphone.unwrap_or("Default"),
            clamshell_selection_active = clamshell_selection_active,
            route_generation = current_route_generation.unwrap_or_default(),
            cached_route_generation = cached_route_generation.unwrap_or_default(),
            "Target device for stream start"
        );

        let has_explicit_device = is_explicit_microphone_selection(active_selected_microphone);
        if used_cached_devices
            && has_explicit_device
            && self
                .get_effective_device_from_list(&settings, devices.clone())
                .is_none()
        {
            info!(
                "Selected device '{}' missing in cached snapshot, refreshing topology before fallback",
                target_device_name
            );
            devices = enumerate_devices("cached_selected_device_miss")?;
            used_cached_devices = false;
            resolution_mode = TopologyResolutionMode::Fresh;
            resolution_reason = "selected_device_missing_in_cache".to_string();
        }

        let (mut device_to_open, mut active_device_name) = self.resolve_device_open_target(
            &settings,
            &target_device_name,
            devices.clone(),
            matches!(context, StreamOpenContext::UserTriggered) && !used_cached_devices,
        );

        let vad_path = self
            .app_handle
            .path()
            .resolve(
                "resources/models/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| anyhow::anyhow!("Failed to resolve VAD path: {}", e))?;

        enum StreamOpenAttempt {
            Opened(u64),
            AlreadyOpen,
            CancelledBeforeOpen,
            RetryWithFreshTopology(String),
            Failed(String),
        }

        let mut should_retry_with_fresh_topology = used_cached_devices;
        loop {
            let attempt = {
                let lock_start = Instant::now();
                let _stream_start_guard = self.stream_start_lock.lock().unwrap();

                if *self.is_open.lock().unwrap() {
                    info!("[TIMING] Lock check: stream opened by another thread during enumeration");
                    StreamOpenAttempt::AlreadyOpen
                } else if !self.prearm_open_allowed(context) {
                    StreamOpenAttempt::CancelledBeforeOpen
                } else {
                    let mut did_mute_guard = self.did_mute.lock().unwrap();
                    *did_mute_guard = false;
                    drop(did_mute_guard);

                    let open_result: Result<(), String> = 'open_attempt: {
                        let mut recorder_opt = self.recorder.lock().unwrap();
                        if recorder_opt.is_none() {
                            match create_audio_recorder(vad_path.to_str().unwrap(), &self.app_handle)
                            {
                                Ok(rec) => *recorder_opt = Some(rec),
                                Err(err) => break 'open_attempt Err(err.to_string()),
                            }
                        }

                        if let Some(rec) = recorder_opt.as_mut() {
                            rec.open(device_to_open.clone()).map_err(|e| e.to_string())
                        } else {
                            Err("Audio recorder unavailable after initialization".to_string())
                        }
                    };

                    match open_result {
                        Ok(()) => {
                            let stream_epoch = self.next_stream_epoch.fetch_add(1, Ordering::SeqCst);
                            self.active_stream_epoch.store(stream_epoch, Ordering::SeqCst);
                            *self.current_device_name.lock().unwrap() = Some(active_device_name.clone());
                            *self.is_open.lock().unwrap() = true;
                            info!(
                                stream_epoch = stream_epoch,
                                "[TIMING] Lock held for {:?}, total init: {:?} (active: {})",
                                lock_start.elapsed(),
                                start_time.elapsed(),
                                active_device_name
                            );
                            StreamOpenAttempt::Opened(stream_epoch)
                        }
                        Err(open_error) => {
                            if should_retry_with_fresh_topology {
                                StreamOpenAttempt::RetryWithFreshTopology(open_error)
                            } else {
                                StreamOpenAttempt::Failed(open_error)
                            }
                        }
                    }
                }
            };

            match attempt {
                StreamOpenAttempt::Opened(stream_epoch) => {
                    return Ok(StreamStartResult {
                        outcome: StreamStartOutcome::OpenedNow { stream_epoch },
                        resolution_mode,
                        resolution_reason,
                    });
                }
                StreamOpenAttempt::AlreadyOpen => {
                    return Ok(StreamStartResult {
                        outcome: StreamStartOutcome::AlreadyOpen,
                        resolution_mode: TopologyResolutionMode::Warm,
                        resolution_reason: "reused_open_stream".to_string(),
                    });
                }
                StreamOpenAttempt::CancelledBeforeOpen => {
                    return Ok(StreamStartResult {
                        outcome: StreamStartOutcome::CancelledBeforeOpen,
                        resolution_mode: TopologyResolutionMode::Warm,
                        resolution_reason: "owner_lost_before_open".to_string(),
                    });
                }
                StreamOpenAttempt::RetryWithFreshTopology(initial_error) => {
                    warn!(
                        "Failed to open recorder from cached topology: {}. Retrying once with fresh enumeration",
                        initial_error
                    );
                    let refreshed = enumerate_devices("cached_open_failure_retry").map_err(|refresh_err| {
                        anyhow::anyhow!(
                            "Failed to refresh devices after cached open failure (initial error: {}, refresh error: {})",
                            initial_error,
                            refresh_err
                        )
                    })?;
                    let (retry_device_to_open, retry_active_device_name) =
                        self.resolve_device_open_target(
                            &settings,
                            &target_device_name,
                            refreshed,
                            matches!(context, StreamOpenContext::UserTriggered),
                        );
                    device_to_open = retry_device_to_open;
                    active_device_name = retry_active_device_name;
                    should_retry_with_fresh_topology = false;
                    resolution_mode = TopologyResolutionMode::Fresh;
                    resolution_reason = "cached_open_failed_retry".to_string();
                }
                StreamOpenAttempt::Failed(open_error) => {
                    error!("Failed to open recorder: {}", open_error);
                    return Err(anyhow::anyhow!("Failed to open microphone: {}", open_error));
                }
            }
        }
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

    fn stop_microphone_stream_locked(&self) -> bool {
        let mut open_flag = self.is_open.lock().unwrap();
        if !*open_flag {
            return false;
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
        self.prearm_owner_token.store(0, Ordering::SeqCst);
        self.active_stream_epoch.store(0, Ordering::SeqCst);
        debug!("Microphone stream stopped");
        true
    }

    pub fn stop_microphone_stream(&self) {
        let _stream_start_guard = self.stream_start_lock.lock().unwrap();
        let _ = self.stop_microphone_stream_locked();
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
        trigger_id: &str,
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
            match self.start_microphone_stream() {
                Ok(stream_start_result) => {
                    info!(
                        session = session_id,
                        binding = binding_id,
                        trigger_id = trigger_id,
                        resolution_mode = stream_start_result.resolution_mode.as_str(),
                        resolution_reason = stream_start_result.resolution_reason.as_str(),
                        event_code = "topology_resolution",
                        "Resolved microphone topology for startup"
                    );
                }
                Err(e) => {
                    error!("Failed to open microphone stream: {e}");
                    let _ = cleanup_failed_start();
                    return RecordingStartOutcome::Failed(RecordingStartFailure::StreamOpenFailed(
                        e.to_string(),
                    ));
                }
            }
        }

        let is_stream_open = *self.is_open.lock().unwrap();
        if is_stream_open {
            let active_device_name = self.current_device_name.lock().unwrap().clone();
            info!(
                session = session_id,
                binding = binding_id,
                trigger_id = trigger_id,
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
                trigger_id = trigger_id,
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
                        trigger_id = trigger_id,
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
                                        trigger_id = trigger_id,
                                        "Recorder stop skipped during self-cancel cleanup: command channel unavailable"
                                    );
                                } else {
                                    warn!(
                                        session = session_id,
                                        binding = binding_id,
                                        trigger_id = trigger_id,
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
                    trigger_id = trigger_id,
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
                            trigger_id = trigger_id,
                            timeout_ms = timeout.as_millis(),
                            event_code = "recording_start_failed",
                            "Capture-ready acknowledgement timed out"
                        );
                    }
                    RecordingStartFailure::RecorderUnavailable => {
                        error!(
                            session = session_id,
                            binding = binding_id,
                            trigger_id = trigger_id,
                            event_code = "recording_start_failed",
                            "Recorder command channel unavailable before capture-ready"
                        );
                    }
                    RecordingStartFailure::StartCommandFailed(detail) => {
                        error!(
                            session = session_id,
                            binding = binding_id,
                            trigger_id = trigger_id,
                            error = detail,
                            event_code = "recording_start_failed",
                            "Recorder start command failed before capture-ready"
                        );
                    }
                    RecordingStartFailure::StartAbandonedDueToSupersededState(detail) => {
                        warn!(
                            session = session_id,
                            binding = binding_id,
                            trigger_id = trigger_id,
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
        should_auto_close_prearm_stream, should_force_fresh_default_route_enumeration,
        should_stop_stream_for_start_cleanup, MicrophoneMode, PrepareToken,
        RecordingStartFailure, RecordingState, StateMismatchKind,
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
    fn default_route_requires_fresh_enumeration_without_monitor() {
        assert!(should_force_fresh_default_route_enumeration(
            Some("default"),
            false,
            Some(5),
            Some(5),
        ));
        assert!(should_force_fresh_default_route_enumeration(
            None,
            false,
            Some(5),
            Some(5),
        ));
    }

    #[test]
    fn default_route_requires_fresh_enumeration_when_generation_changes() {
        assert!(should_force_fresh_default_route_enumeration(
            Some("default"),
            true,
            Some(4),
            Some(5),
        ));
        assert!(should_force_fresh_default_route_enumeration(
            Some("default"),
            true,
            None,
            Some(5),
        ));
    }

    #[test]
    fn explicit_selection_does_not_force_fresh_default_route_policy() {
        assert!(!should_force_fresh_default_route_enumeration(
            Some("USB Mic"),
            false,
            None,
            None,
        ));
    }

    #[test]
    fn prearm_autoclose_requires_owned_idle_on_demand_stream() {
        assert!(should_auto_close_prearm_stream(
            &MicrophoneMode::OnDemand,
            &RecordingState::Idle,
            false,
            true,
            true,
            Some(7),
            7,
        ));
        assert!(!should_auto_close_prearm_stream(
            &MicrophoneMode::OnDemand,
            &RecordingState::Idle,
            false,
            true,
            true,
            Some(7),
            8,
        ));
        assert!(!should_auto_close_prearm_stream(
            &MicrophoneMode::AlwaysOn,
            &RecordingState::Idle,
            false,
            true,
            true,
            Some(7),
            7,
        ));
        assert!(!should_auto_close_prearm_stream(
            &MicrophoneMode::OnDemand,
            &RecordingState::Recording {
                binding_id: "transcribe".to_string(),
                session_id: "session".to_string(),
            },
            true,
            true,
            true,
            Some(7),
            7,
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
