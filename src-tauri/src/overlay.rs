use crate::input;
use crate::settings;
use crate::settings::OverlayPosition;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

#[cfg(not(target_os = "macos"))]
use tracing::debug;

#[cfg(not(target_os = "macos"))]
use tauri::WebviewWindowBuilder;

#[cfg(target_os = "macos")]
use tauri::WebviewUrl;

#[cfg(target_os = "macos")]
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt as PanelManagerExt, PanelBuilder, PanelLevel,
    StyleMask, TrackingAreaOptions,
};

#[cfg(target_os = "linux")]
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
#[cfg(target_os = "linux")]
use std::env;

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(RecordingOverlayPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            is_floating_panel: true
        }
        with: {
            tracking_area: {
                options: TrackingAreaOptions::new()
                    .active_always()
                    .mouse_entered_and_exited()
                    .mouse_moved()
                    .cursor_update(),
                auto_resize: true
            }
        }
    })
}

const OVERLAY_WIDTH: f64 = 234.0;
const OVERLAY_HEIGHT: f64 = 40.0;

#[cfg(target_os = "macos")]
const OVERLAY_TOP_OFFSET: f64 = 46.0;
#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_TOP_OFFSET: f64 = 4.0;

#[cfg(target_os = "macos")]
const OVERLAY_BOTTOM_OFFSET: f64 = 15.0;

#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_BOTTOM_OFFSET: f64 = 40.0;

#[cfg(target_os = "linux")]
fn update_gtk_layer_shell_anchors(overlay_window: &tauri::webview::WebviewWindow) {
    let window_clone = overlay_window.clone();
    let _ = overlay_window.run_on_main_thread(move || {
        // Try to get the GTK window from the Tauri webview
        if let Ok(gtk_window) = window_clone.gtk_window() {
            let settings = settings::get_settings(window_clone.app_handle());
            match settings.overlay_position {
                OverlayPosition::Top => {
                    gtk_window.set_anchor(Edge::Top, true);
                    gtk_window.set_anchor(Edge::Bottom, false);
                }
                OverlayPosition::Bottom | OverlayPosition::None => {
                    gtk_window.set_anchor(Edge::Bottom, true);
                    gtk_window.set_anchor(Edge::Top, false);
                }
            }
        }
    });
}

/// Initializes GTK layer shell for Linux overlay window
/// Returns true if layer shell was successfully initialized, false otherwise
#[cfg(target_os = "linux")]
fn init_gtk_layer_shell(overlay_window: &tauri::webview::WebviewWindow) -> bool {
    // On KDE Wayland, layer-shell init has shown protocol instability.
    // Fall back to regular always-on-top overlay behavior (as in v0.7.1).
    let is_wayland = env::var("WAYLAND_DISPLAY").is_ok()
        || env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);
    let is_kde = env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
        || env::var("KDE_SESSION_VERSION").is_ok();
    if is_wayland && is_kde {
        debug!("Skipping GTK layer shell init on KDE Wayland");
        return false;
    }

    if !gtk_layer_shell::is_supported() {
        return false;
    }

    // Try to get the GTK window from the Tauri webview
    if let Ok(gtk_window) = overlay_window.gtk_window() {
        // Initialize layer shell
        gtk_window.init_layer_shell();
        gtk_window.set_layer(Layer::Overlay);
        gtk_window.set_keyboard_mode(KeyboardMode::None);
        gtk_window.set_exclusive_zone(0);

        update_gtk_layer_shell_anchors(overlay_window);

        return true;
    }
    false
}

/// Forces a window to be topmost using Win32 API (Windows only)
/// This is more reliable than Tauri's set_always_on_top which can be overridden
#[cfg(target_os = "windows")]
fn force_overlay_topmost(overlay_window: &tauri::webview::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    // Clone because run_on_main_thread takes 'static
    let overlay_clone = overlay_window.clone();

    // Make sure the Win32 call happens on the UI thread
    let _ = overlay_clone.clone().run_on_main_thread(move || {
        if let Ok(hwnd) = overlay_clone.hwnd() {
            unsafe {
                // Force Z-order: make this window topmost without changing size/pos or stealing focus
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    });
}

fn get_monitor_with_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    if let Some(mouse_location) = input::get_cursor_position(app_handle) {
        if let Ok(monitors) = app_handle.available_monitors() {
            for monitor in monitors {
                let is_within =
                    is_mouse_within_monitor(mouse_location, monitor.position(), monitor.size());
                if is_within {
                    return Some(monitor);
                }
            }
        }
    }

    app_handle.primary_monitor().ok().flatten()
}

fn is_mouse_within_monitor(
    mouse_pos: (i32, i32),
    monitor_pos: &PhysicalPosition<i32>,
    monitor_size: &PhysicalSize<u32>,
) -> bool {
    let (mouse_x, mouse_y) = mouse_pos;
    let PhysicalPosition {
        x: monitor_x,
        y: monitor_y,
    } = *monitor_pos;
    let PhysicalSize {
        width: monitor_width,
        height: monitor_height,
    } = *monitor_size;

    mouse_x >= monitor_x
        && mouse_x < (monitor_x + monitor_width as i32)
        && mouse_y >= monitor_y
        && mouse_y < (monitor_y + monitor_height as i32)
}

fn calculate_overlay_position(app_handle: &AppHandle) -> Option<(f64, f64)> {
    if let Some(monitor) = get_monitor_with_cursor(app_handle) {
        let work_area = monitor.work_area();
        let scale = monitor.scale_factor();
        let work_area_width = work_area.size.width as f64 / scale;
        let work_area_height = work_area.size.height as f64 / scale;
        let work_area_x = work_area.position.x as f64 / scale;
        let work_area_y = work_area.position.y as f64 / scale;

        let settings = settings::get_settings(app_handle);

        let x = work_area_x + (work_area_width - OVERLAY_WIDTH) / 2.0;
        let y = match settings.overlay_position {
            OverlayPosition::Top => work_area_y + OVERLAY_TOP_OFFSET,
            OverlayPosition::Bottom | OverlayPosition::None => {
                #[cfg(target_os = "macos")]
                {
                    // don't subtract the overlay height it puts it too far up
                    work_area_y + work_area_height - OVERLAY_BOTTOM_OFFSET
                }
                #[cfg(not(target_os = "macos"))]
                {
                    work_area_y + work_area_height - OVERLAY_HEIGHT - OVERLAY_BOTTOM_OFFSET
                }
            }
        };

        return Some((x, y));
    }
    None
}

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    Recording,
    Transcribing,
    Processing,
    Connecting,
    Cancelling,
    Correcting,
}

static OVERLAY_STATE: LazyLock<Mutex<OverlayState>> =
    LazyLock::new(|| Mutex::new(OverlayState::Hidden));

const OVERLAY_HOVER_ACTIVE_POLL_INTERVAL_MS: u64 = 33;
const OVERLAY_HOVER_IDLE_POLL_INTERVAL_MS: u64 = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayInputMode {
    Passthrough,
    InteractiveOperation,
    InteractiveUndo,
}

impl OverlayInputMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::InteractiveOperation => "interactive_operation",
            Self::InteractiveUndo => "interactive_undo",
        }
    }

    fn is_interactive(self) -> bool {
        !matches!(self, Self::Passthrough)
    }

    fn is_passthrough(self) -> bool {
        matches!(self, Self::Passthrough)
    }
}

#[derive(Debug, Clone, Copy)]
struct OverlayInteractionState {
    requested_mode: OverlayInputMode,
    effective_mode: OverlayInputMode,
    transition_seq: u64,
}

impl Default for OverlayInteractionState {
    fn default() -> Self {
        Self {
            requested_mode: OverlayInputMode::Passthrough,
            effective_mode: OverlayInputMode::Passthrough,
            transition_seq: 0,
        }
    }
}

static OVERLAY_INTERACTION_STATE: LazyLock<Mutex<OverlayInteractionState>> =
    LazyLock::new(|| Mutex::new(OverlayInteractionState::default()));

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayClientRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayInteractionRegionsPayload {
    pub overlay_visible: bool,
    pub message_lane_rect: Option<OverlayClientRect>,
    #[serde(default)]
    pub action_rects: Vec<OverlayClientRect>,
}

#[derive(Debug, Clone, Copy)]
struct OverlayScreenRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl OverlayScreenRect {
    fn from_client_rect(window: &tauri::WebviewWindow, rect: OverlayClientRect) -> Option<Self> {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return None;
        }

        let outer_position = window.outer_position().ok()?;
        let scale_factor = window.scale_factor().ok().unwrap_or(1.0);
        // outer_position() returns PhysicalPosition (device pixels).
        // Enigo location() returns screen points (logical).
        // CSS getBoundingClientRect() returns logical coordinates.
        // Convert outer_position to logical by dividing by scale_factor
        // so all values share the same coordinate space.
        let origin_x = outer_position.x as f64 / scale_factor;
        let origin_y = outer_position.y as f64 / scale_factor;

        Some(Self {
            x: origin_x + rect.x,
            y: origin_y + rect.y,
            width: rect.width,
            height: rect.height,
        })
    }

    fn contains(self, cursor: (i32, i32)) -> bool {
        let cursor_x = cursor.0 as f64;
        let cursor_y = cursor.1 as f64;

        cursor_x >= self.x
            && cursor_x <= self.x + self.width
            && cursor_y >= self.y
            && cursor_y <= self.y + self.height
    }
}

#[derive(Debug, Default)]
struct OverlayHoverFallbackState {
    overlay_visible: bool,
    message_lane_rect: Option<OverlayScreenRect>,
    action_rects: Vec<OverlayScreenRect>,
    lane_hover_active: bool,
    pointer_intent_active: bool,
    polling_active: bool,
    worker_started: bool,
}

static OVERLAY_HOVER_FALLBACK_STATE: LazyLock<Mutex<OverlayHoverFallbackState>> =
    LazyLock::new(|| Mutex::new(OverlayHoverFallbackState::default()));

fn should_poll_hover_fallback(
    effective_mode: OverlayInputMode,
    hover_state: &OverlayHoverFallbackState,
) -> bool {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = effective_mode;
        let _ = hover_state;
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        if !effective_mode.is_interactive() || !hover_state.overlay_visible {
            return false;
        }

        hover_state.message_lane_rect.is_some() || !hover_state.action_rects.is_empty()
    }
}

fn emit_overlay_cursor_intent(app_handle: &AppHandle, intent: &str) {
    if app_handle.emit("overlay-cursor-intent", intent).is_ok() {
        tracing::info!(
            event_code = "overlay_cursor_intent_changed",
            intent = intent,
            "Emitted overlay cursor intent"
        );
    }
}

fn refresh_hover_fallback_runtime(app_handle: &AppHandle, source: &str) {
    let effective_mode = OVERLAY_INTERACTION_STATE
        .lock()
        .map(|state| state.effective_mode)
        .unwrap_or(OverlayInputMode::Passthrough);

    let mut should_start_worker = false;
    let mut emit_hover_leave = false;
    let mut emit_cursor_default = false;
    let mut started_or_stopped: Option<bool> = None;

    if let Ok(mut hover_state) = OVERLAY_HOVER_FALLBACK_STATE.lock() {
        let should_poll = should_poll_hover_fallback(effective_mode, &hover_state);
        if hover_state.polling_active != should_poll {
            hover_state.polling_active = should_poll;
            started_or_stopped = Some(should_poll);
            if !should_poll {
                emit_hover_leave = hover_state.lane_hover_active;
                emit_cursor_default = hover_state.pointer_intent_active;
                hover_state.lane_hover_active = false;
                hover_state.pointer_intent_active = false;
            }
        }

        if should_poll && !hover_state.worker_started {
            hover_state.worker_started = true;
            should_start_worker = true;
        }
    }

    if let Some(started) = started_or_stopped {
        tracing::info!(
            event_code = if started {
                "overlay_hover_fallback_started"
            } else {
                "overlay_hover_fallback_stopped"
            },
            source = source,
            mode = effective_mode.as_str(),
            "Updated overlay hover fallback state"
        );
    }

    if emit_hover_leave {
        let _ = app_handle.emit("overlay-hover-leave", ());
    }

    if emit_cursor_default {
        emit_overlay_cursor_intent(app_handle, "default");
    }

    if should_start_worker {
        let app = app_handle.clone();
        std::thread::spawn(move || {
            loop {
                let (polling_active, message_lane_rect, action_rects) =
                    if let Ok(state) = OVERLAY_HOVER_FALLBACK_STATE.lock() {
                        (
                            state.polling_active,
                            state.message_lane_rect,
                            state.action_rects.clone(),
                        )
                    } else {
                        (false, None, Vec::new())
                    };

                if !polling_active {
                    std::thread::sleep(Duration::from_millis(
                        OVERLAY_HOVER_IDLE_POLL_INTERVAL_MS,
                    ));
                    continue;
                }

                let Some(cursor) = input::get_cursor_position(&app) else {
                    std::thread::sleep(Duration::from_millis(
                        OVERLAY_HOVER_ACTIVE_POLL_INTERVAL_MS,
                    ));
                    continue;
                };

                let inside_lane = message_lane_rect.map(|rect| rect.contains(cursor)).unwrap_or(false);
                let inside_action = action_rects.iter().any(|rect| rect.contains(cursor));

                let mut emit_enter = false;
                let mut emit_leave = false;
                let mut cursor_intent: Option<&'static str> = None;

                if let Ok(mut state) = OVERLAY_HOVER_FALLBACK_STATE.lock() {
                    if !state.polling_active {
                        continue;
                    }

                    if state.lane_hover_active != inside_lane {
                        state.lane_hover_active = inside_lane;
                        if inside_lane {
                            emit_enter = true;
                        } else {
                            emit_leave = true;
                        }
                    }

                    if state.pointer_intent_active != inside_action {
                        state.pointer_intent_active = inside_action;
                        cursor_intent = Some(if inside_action { "pointer" } else { "default" });
                    }
                }

                if emit_enter {
                    let _ = app.emit("overlay-hover-enter", ());
                } else if emit_leave {
                    let _ = app.emit("overlay-hover-leave", ());
                }

                if let Some(intent) = cursor_intent {
                    emit_overlay_cursor_intent(&app, intent);
                }

                std::thread::sleep(Duration::from_millis(
                    OVERLAY_HOVER_ACTIVE_POLL_INTERVAL_MS,
                ));
            }
        });
    }
}

/// Check if the overlay is in any active state (not Hidden)
pub fn is_overlay_active() -> bool {
    if let Ok(state) = OVERLAY_STATE.lock() {
        !matches!(*state, OverlayState::Hidden)
    } else {
        false
    }
}

/// Returns true when overlay UX can currently surface interactive content.
pub fn is_overlay_available(app_handle: &AppHandle) -> bool {
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        return false;
    }

    if !OVERLAY_READY.load(Ordering::SeqCst) {
        return false;
    }

    app_handle.get_webview_window("recording_overlay").is_some()
}

fn apply_overlay_input_mode(
    app_handle: &AppHandle,
    requested_mode: OverlayInputMode,
    source: &str,
    caller_file: Option<&str>,
    caller_line: Option<u32>,
) {
    let effective_mode = requested_mode;
    let requested_passthrough = requested_mode.is_passthrough();
    let effective_passthrough = effective_mode.is_passthrough();

    let (previous_mode, transition_seq) = if let Ok(mut state) = OVERLAY_INTERACTION_STATE.lock() {
        let previous_mode = state.effective_mode;
        state.requested_mode = requested_mode;
        state.effective_mode = effective_mode;
        state.transition_seq = state.transition_seq.saturating_add(1);
        (previous_mode, state.transition_seq)
    } else {
        (OverlayInputMode::Passthrough, 0)
    };

    tracing::info!(
        event_code = "overlay_input_mode_transition",
        source = source,
        previous_mode = previous_mode.as_str(),
        requested_mode = requested_mode.as_str(),
        effective_mode = effective_mode.as_str(),
        requested_passthrough = requested_passthrough,
        effective_passthrough = effective_passthrough,
        transition_seq = transition_seq,
        caller_file = caller_file.unwrap_or("unknown"),
        caller_line = caller_line.unwrap_or_default(),
        "Updated overlay input interaction mode"
    );

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        match overlay_window.set_ignore_cursor_events(effective_passthrough) {
            Ok(_) => {
                tracing::info!(
                    event_code = "overlay_cursor_passthrough_set",
                    source = source,
                    requested_passthrough = requested_passthrough,
                    effective_passthrough = effective_passthrough,
                    caller_file = caller_file.unwrap_or("unknown"),
                    caller_line = caller_line.unwrap_or_default(),
                    "Updated overlay cursor passthrough"
                );
            }
            Err(error) => {
                tracing::warn!(
                    event_code = "overlay_cursor_passthrough_set_failed",
                    source = source,
                    requested_passthrough = requested_passthrough,
                    effective_passthrough = effective_passthrough,
                    caller_file = caller_file.unwrap_or("unknown"),
                    caller_line = caller_line.unwrap_or_default(),
                    error = %error,
                    "Failed to update overlay cursor passthrough"
                );
            }
        }

        #[cfg(target_os = "macos")]
        {
            let interactive = !effective_passthrough;
            tracing::info!(
                event_code = "overlay_panel_key_state_requested",
                interactive = interactive,
                source = source,
                requested_passthrough = requested_passthrough,
                effective_passthrough = effective_passthrough,
                "Requested macOS overlay panel key state"
            );

            match set_overlay_panel_interaction_focus(app_handle, interactive, effective_passthrough)
            {
                Ok(_) => {
                    tracing::info!(
                        event_code = "overlay_panel_key_state_applied",
                        interactive = interactive,
                        source = source,
                        requested_passthrough = requested_passthrough,
                        effective_passthrough = effective_passthrough,
                        "Applied macOS overlay panel key state"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        event_code = "overlay_panel_key_state_failed",
                        interactive = interactive,
                        source = source,
                        requested_passthrough = requested_passthrough,
                        effective_passthrough = effective_passthrough,
                        error = %error,
                        "Failed to update macOS overlay panel key state"
                    );
                }
            }
        }
    } else {
        tracing::warn!(
            event_code = "overlay_cursor_passthrough_set_skipped",
            reason = "window_not_found",
            source = source,
            requested_passthrough = requested_passthrough,
            effective_passthrough = effective_passthrough,
            caller_file = caller_file.unwrap_or("unknown"),
            caller_line = caller_line.unwrap_or_default(),
            "Skipped overlay cursor passthrough update"
        );
    }

    refresh_hover_fallback_runtime(app_handle, source);
}

/// Toggle whether the overlay should pass cursor events through to the app behind it.
/// `passthrough=false` means overlay captures hover/click/cursor updates.
#[cfg(target_os = "macos")]
fn set_overlay_panel_interaction_focus(
    app_handle: &AppHandle,
    interactive: bool,
    effective_passthrough: bool,
) -> Result<(), String> {
    if app_handle.get_webview_panel("recording_overlay").is_err() {
        return Err("panel_not_found".to_string());
    }

    let app_for_main = app_handle.clone();
    app_handle
        .run_on_main_thread(move || {
            let Ok(panel) = app_for_main.get_webview_panel("recording_overlay") else {
                return;
            };

            panel.set_ignores_mouse_events(effective_passthrough);
            // Keep move delivery enabled so hover/cursor updates work when interaction is on.
            panel.set_accepts_mouse_moved_events(interactive);
        })
        .map_err(|error| format!("{error:?}"))
}

/// Toggle whether the overlay should pass cursor events through to the app behind it.
/// `passthrough=false` means overlay captures hover/click/cursor updates.
#[track_caller]
pub fn set_overlay_cursor_passthrough(app_handle: &AppHandle, passthrough: bool) {
    let caller = std::panic::Location::caller();
    let source = format!("{}:{}", caller.file(), caller.line());
    let requested_mode = if passthrough {
        OverlayInputMode::Passthrough
    } else {
        OverlayInputMode::InteractiveOperation
    };
    apply_overlay_input_mode(
        app_handle,
        requested_mode,
        &source,
        Some(caller.file()),
        Some(caller.line()),
    );
}

pub fn set_overlay_input_mode_undo(app_handle: &AppHandle, source: &str) {
    apply_overlay_input_mode(
        app_handle,
        OverlayInputMode::InteractiveUndo,
        source,
        None,
        None,
    );
}

pub fn set_overlay_input_mode_passthrough(app_handle: &AppHandle, source: &str) {
    apply_overlay_input_mode(
        app_handle,
        OverlayInputMode::Passthrough,
        source,
        None,
        None,
    );
}

/// Emits an undo overlay event. Returns true if delivery path was available.
pub fn emit_undo_overlay_event<T: Serialize>(app_handle: &AppHandle, payload: &T) -> bool {
    if !is_overlay_available(app_handle) {
        return false;
    }

    let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") else {
        return false;
    };

    // Undo cards are interactive; ensure clicks/cursor reach the overlay.
    set_overlay_input_mode_undo(app_handle, "emit_undo_overlay_event");
    if !overlay_window.is_visible().unwrap_or(false) {
        let _ = overlay_window.show();
    }

    overlay_window.emit("undo-overlay-event", payload).is_ok()
}

#[tauri::command]
#[specta::specta]
pub fn overlay_update_interaction_regions(app: AppHandle, regions: OverlayInteractionRegionsPayload) {
    let Some(overlay_window) = app.get_webview_window("recording_overlay") else {
        tracing::warn!(
            event_code = "overlay_hover_regions_update_skipped",
            reason = "window_not_found",
            "Skipped overlay interaction region update"
        );
        return;
    };

    let message_lane_rect = regions
        .message_lane_rect
        .and_then(|rect| OverlayScreenRect::from_client_rect(&overlay_window, rect));
    let action_rects: Vec<OverlayScreenRect> = regions
        .action_rects
        .into_iter()
        .filter_map(|rect| OverlayScreenRect::from_client_rect(&overlay_window, rect))
        .collect();

    if let Ok(mut hover_state) = OVERLAY_HOVER_FALLBACK_STATE.lock() {
        hover_state.overlay_visible = regions.overlay_visible;
        hover_state.message_lane_rect = message_lane_rect;
        hover_state.action_rects = action_rects;
    }

    refresh_hover_fallback_runtime(&app, "overlay_update_interaction_regions");
}

/// Check if the overlay is currently in the Correcting state.
/// Used by fn_key_monitor to intercept Tab/Esc for accept/dismiss.
pub fn is_correcting() -> bool {
    if let Ok(state) = OVERLAY_STATE.lock() {
        matches!(*state, OverlayState::Correcting)
    } else {
        false
    }
}

/// Atomically clear the Correcting state to Hidden.
///
/// Called by fn_key_monitor immediately after detecting Tab/Esc to prevent
/// duplicate triggering and minimize the race window for keystroke swallowing.
/// The visual hiding (emit + window ops) happens later in the spawned thread.
pub fn clear_correcting_state() {
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        if matches!(*state, OverlayState::Correcting) {
            *state = OverlayState::Hidden;
        }
    }
}

/// Track whether the overlay webview is ready to receive events.
/// This prevents the race condition where show-overlay events are emitted
/// before the React component has registered its event listeners.
static OVERLAY_READY: AtomicBool = AtomicBool::new(false);

/// Mark the overlay as ready to receive events.
/// Called when the frontend emits the "overlay-ready" event.
/// If the current state is not Hidden, we re-emit the "show-overlay" event
/// to ensure the reloaded/remounted frontend receives the correct state.
pub fn mark_overlay_ready(app_handle: &AppHandle) {
    use tauri::Manager;

    tracing::info!("mark_overlay_ready: Overlay webview signaled ready");
    OVERLAY_READY.store(true, Ordering::SeqCst);

    // Check current state
    if let Ok(state) = OVERLAY_STATE.lock() {
        let current_state = *state;
        tracing::debug!(
            "mark_overlay_ready: current backend state is {:?}",
            current_state
        );

        if !matches!(current_state, OverlayState::Hidden) {
            set_overlay_cursor_passthrough(app_handle, false);
        }

        // If we are supposed to be showing something, re-emit the event
        match current_state {
            OverlayState::Recording => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "mark_overlay_ready: State is Recording, re-emitting show-overlay"
                    );
                    let _ = overlay.emit("show-overlay", "recording");
                }
            }
            OverlayState::Transcribing => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "mark_overlay_ready: State is Transcribing, re-emitting show-overlay"
                    );
                    let _ = overlay.emit("show-overlay", "transcribing");
                }
            }
            OverlayState::Processing => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "mark_overlay_ready: State is Processing, re-emitting show-overlay"
                    );
                    let _ = overlay.emit("show-overlay", "processing");
                }
            }
            OverlayState::Connecting => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "mark_overlay_ready: State is Connecting, re-emitting show-overlay"
                    );
                    let _ = overlay.emit("show-overlay", "connecting");
                }
            }
            OverlayState::Cancelling => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "mark_overlay_ready: State is Cancelling, re-emitting show-overlay"
                    );
                    let _ = overlay.emit("show-overlay", "cancelling");
                }
            }
            OverlayState::Correcting => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "mark_overlay_ready: State is Correcting, re-emitting show-overlay"
                    );
                    let _ = overlay.emit("show-overlay", "correcting");
                }
            }
            OverlayState::Hidden => {
                // Do nothing, correct state matches default frontend state
            }
        }
    }
}

pub fn emit_levels(app_handle: &AppHandle, levels: &Vec<f32>) {
    // emit levels to main app
    let _ = app_handle.emit("mic-level", levels);

    // also emit to the recording overlay if it's open
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let _ = overlay_window.emit("mic-level", levels);
    }
}

/// Emit recording time progress to the overlay
/// elapsed_secs: seconds elapsed since recording started
/// max_secs: maximum allowed recording time in seconds
pub fn emit_recording_time(app_handle: &AppHandle, elapsed_secs: u32, max_secs: u32) {
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let _ = overlay_window.emit("recording-time", (elapsed_secs, max_secs));
    }
}

/// Creates the recording overlay window and keeps it hidden by default
#[cfg(not(target_os = "macos"))]
pub fn create_recording_overlay(app_handle: &AppHandle) {
    let position = calculate_overlay_position(app_handle);

    // On Linux (Wayland), monitor detection often fails, but we don't need exact coordinates
    // for Layer Shell as we use anchors. On other platforms, we require a position.
    #[cfg(not(target_os = "linux"))]
    if position.is_none() {
        debug!("Failed to determine overlay position, not creating overlay window");
        return;
    }

    let mut builder = WebviewWindowBuilder::new(
        app_handle,
        "recording_overlay",
        tauri::WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Recording")
    .resizable(false)
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .shadow(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .accept_first_mouse(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .focused(false)
    .visible(false);

    if let Some((x, y)) = position {
        builder = builder.position(x, y);
    }

    match builder.build() {
        Ok(window) => {
            #[cfg(target_os = "linux")]
            {
                // Try to initialize GTK layer shell, ignore errors if compositor doesn't support it
                if init_gtk_layer_shell(&window) {
                    debug!("GTK layer shell initialized for overlay window");
                } else {
                    debug!("GTK layer shell not available, falling back to regular window");
                }
            }

            debug!("Recording overlay window created successfully (hidden)");
        }
        Err(e) => {
            debug!("Failed to create recording overlay window: {}", e);
        }
    }
}

/// Creates the recording overlay panel and keeps it hidden by default (macOS)
#[cfg(target_os = "macos")]
pub fn create_recording_overlay(app_handle: &AppHandle) {
    if let Some((x, y)) = calculate_overlay_position(app_handle) {
        // PanelBuilder creates a Tauri window then converts it to NSPanel.
        // The window remains registered, so get_webview_window() still works.
        match PanelBuilder::<_, RecordingOverlayPanel>::new(app_handle, "recording_overlay")
            .url(WebviewUrl::App("src/overlay/index.html".into()))
            .title("Recording")
            .position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
            .level(PanelLevel::Status)
            .size(tauri::Size::Logical(tauri::LogicalSize {
                width: OVERLAY_WIDTH,
                height: OVERLAY_HEIGHT,
            }))
            .style_mask(StyleMask::empty().nonactivating_panel())
            .has_shadow(false)
            .transparent(true)
            .no_activate(true)
            .corner_radius(0.0)
            .with_window(|w| w.decorations(false).transparent(true).visible(true))
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .build()
        {
            Ok(panel) => {
                // Don't hide the panel! Hiding it causes macOS to suspend the webview (App Nap),
                // which causes a delay when showing it later.
                // Instead, we rely on:
                // 1. transparent(true) - so it's invisible
                // 2. set_ignore_cursor_events(true) - so clicks pass through
                // 3. CSS opacity: 0 - so content is invisible

                // Ensure it ignores mouse events initially so it doesn't block the screen
                let _ = panel.set_ignores_mouse_events(true);
                // Keep mouse-move delivery enabled for hover/cursor updates when interaction is toggled on.
                panel.set_accepts_mouse_moved_events(true);

                // Force the panel to be visible (but transparent) immediately.
                // This ensures the WebView process is active (not App Napped) and mounts the React app at startup.
                // Since we set transparent(true) and the CSS defaults to opacity: 0, it will be invisible to the user.
                let _ = panel.show();

                // Verify visibility immediately
                if let Some(w) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!(
                        "create_recording_overlay: Panel created. Visible? {}",
                        w.is_visible().unwrap_or(false)
                    );
                } else {
                    tracing::warn!(
                        "create_recording_overlay: Panel created but get_webview_window failed!"
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to create recording overlay panel: {}", e);
            }
        }
    }
}

/// Shows the recording overlay window with fade-in animation
pub fn show_recording_overlay(app_handle: &AppHandle) {
    use std::time::Duration;
    use tracing::{debug, warn};

    debug!("show_recording_overlay: entry");

    // Update state to Recording
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Recording;
    }

    // Check if overlay should be shown based on position setting
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_recording_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Wait for overlay to be ready (with timeout) on first show
        // This prevents the race condition where the event is emitted before
        // React has registered its listeners, causing flicker on first Fn press
        // Optimistic show: We assume the overlay is ready because we created it at startup.
        // Waiting for the roundtrip "overlay-ready" signal is causing a ~500ms delay.
        // We log the status but proceed immediately.
        // Optimistic show: We assume the overlay is ready because we created it at startup.
        // Waiting for the roundtrip "overlay-ready" signal is causing a ~500ms delay if startup lazy-loading failed.
        // But we MUST wait if it's truly not ready, otherwise we risk a white flash (flicker).
        // If my fix in create_recording_overlay works, OVERLAY_READY will be true immediately, so this loop will exit instantly (0ms delay).
        if !OVERLAY_READY.load(Ordering::SeqCst) {
            debug!("show_recording_overlay: overlay not ready yet, waiting...");
            let wait_start = std::time::Instant::now();
            // Short spin-wait with timeout (max ~500ms, checking every 10ms)
            for _ in 0..50 {
                if OVERLAY_READY.load(Ordering::SeqCst) {
                    debug!(
                        "show_recording_overlay: overlay became ready in {:?}",
                        wait_start.elapsed()
                    );
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if !OVERLAY_READY.load(Ordering::SeqCst) {
                warn!("show_recording_overlay: timeout waiting for overlay ready after {:?}, proceeding anyway", wait_start.elapsed());
            }
        }

        // Log current visibility state before showing
        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        debug!(
            "show_recording_overlay: found window, is_visible_before={}",
            is_visible_before
        );

        // Enable interaction immediately.
        set_overlay_cursor_passthrough(app_handle, false);

        // Update position before showing to prevent flicker from position changes
        #[cfg(target_os = "linux")]
        {
            update_gtk_layer_shell_anchors(&overlay_window);
        }

        if let Some((x, y)) = calculate_overlay_position(app_handle) {
            let _ = overlay_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }

        // IMPORTANT: Emit the show-overlay event.
        // If window is already visible, this triggers the React fade-in.
        let emit_result = overlay_window.emit("show-overlay", "recording");
        debug!(
            "show_recording_overlay: emit('show-overlay', 'recording') result={:?}",
            emit_result
        );

        // If the window is NOT visible (first run or force hidden), show it safely
        if !is_visible_before {
            // Small delay to allow React to process the event and update opacity
            // before the native window becomes visible.
            std::thread::sleep(Duration::from_millis(50));

            let show_start = std::time::Instant::now();
            let show_result = overlay_window.show();
            debug!(
                "show_recording_overlay: window.show() completed in {:?} result={:?}",
                show_start.elapsed(),
                show_result
            );
        } else {
            debug!("show_recording_overlay: window already visible, skipping native show() to avoid flicker");
        }

        // On Windows, aggressively re-assert "topmost" in the native Z-order after showing
        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);

        // Log post-show visibility
        let is_visible_after = overlay_window.is_visible().unwrap_or(false);
        debug!(
            "show_recording_overlay: is_visible_after={}",
            is_visible_after
        );
    } else {
        warn!("show_recording_overlay: overlay window 'recording_overlay' NOT FOUND!");
    }
}

/// Shows the transcribing overlay window
pub fn show_transcribing_overlay(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("show_transcribing_overlay: entry");

    // Update state to Transcribing
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Transcribing;
    }

    // Check if overlay should be shown based on position setting
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_transcribing_overlay: overlay disabled in settings, skipping");
        return;
    }

    update_overlay_position(app_handle);

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        debug!(
            "show_transcribing_overlay: found window, is_visible_before={}",
            is_visible_before
        );

        // Ensure interaction is enabled (just in case).
        set_overlay_cursor_passthrough(app_handle, false);

        if !is_visible_before {
            let show_result = overlay_window.show();
            debug!(
                "show_transcribing_overlay: window.show() result={:?}",
                show_result
            );
        } else {
            debug!("show_transcribing_overlay: window already visible, skipping show()");
        }

        // On Windows, aggressively re-assert "topmost" in the native Z-order after showing
        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);

        // Emit event to switch to transcribing state
        let emit_result = overlay_window.emit("show-overlay", "transcribing");
        debug!("show_transcribing_overlay: emit result={:?}", emit_result);
    } else {
        warn!("show_transcribing_overlay: overlay window NOT FOUND!");
    }
}

/// Shows the correction overlay (ghost text) near the text cursor.
/// Positions the overlay at the cursor position from the accessibility module.
pub fn show_correction_overlay(
    app_handle: &AppHandle,
    correction: &crate::accessibility::CorrectionResult,
) {
    use tracing::{debug, info, warn};

    info!("show_correction_overlay: entry");

    // Update state to Correcting
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Correcting;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Enable interaction so user can see and interact with the overlay.
        set_overlay_cursor_passthrough(app_handle, false);

        // Position the overlay — try to use the correction data's screen position
        // For now, we reuse the standard position. In a future iteration,
        // we could position it near the text cursor.
        if let Some((x, y)) = calculate_overlay_position(app_handle) {
            let _ = overlay_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }

        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        if !is_visible_before {
            let _ = overlay_window.show();
        }

        // Emit the correction data FIRST so the frontend has it before the state change
        let data_result = overlay_window.emit("correction-result", correction);
        debug!(
            "show_correction_overlay: emit('correction-result') result={:?}",
            data_result
        );

        // Then emit the state change — the UI will render correction data immediately
        let emit_result = overlay_window.emit("show-overlay", "correcting");
        debug!(
            "show_correction_overlay: emit('show-overlay', 'correcting') result={:?}",
            emit_result
        );

        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);
    } else {
        warn!("show_correction_overlay: overlay window NOT FOUND!");
    }
}

/// Shows the processing overlay window (during post-processing phase)
pub fn show_processing_overlay(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("show_processing_overlay: entry");

    // Update state to Processing
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Processing;
    }

    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_processing_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        set_overlay_cursor_passthrough(app_handle, false);
        // Emit event to switch to processing state
        let emit_result = overlay_window.emit("show-overlay", "processing");
        debug!("show_processing_overlay: emit result={:?}", emit_result);
    } else {
        warn!("show_processing_overlay: overlay window NOT FOUND!");
    }
}

/// Updates the overlay window position based on current settings
pub fn update_overlay_position(app_handle: &AppHandle) {
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        #[cfg(target_os = "linux")]
        {
            update_gtk_layer_shell_anchors(&overlay_window);
        }

        if let Some((x, y)) = calculate_overlay_position(app_handle) {
            let _ = overlay_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
    }
}

/// Hides the recording overlay window with fade-out animation
pub fn hide_recording_overlay(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("hide_recording_overlay: entry (FORCE)");

    // Always hide and reset state (Force Hide)
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Hidden;
    }

    // Always hide the overlay regardless of settings - if setting was changed while recording,
    // we still want to hide it properly
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        debug!(
            "hide_recording_overlay: found window, is_visible_before={}",
            is_visible_before
        );

        // Emit event to trigger fade-out animation (CSS handles the 300ms transition)
        let emit_result = overlay_window.emit("hide-overlay", ());
        debug!(
            "hide_recording_overlay: emit('hide-overlay') result={:?}",
            emit_result
        );

        // Instead of hiding the window (which causes flicker on next show),
        // pass cursor events through and let React render opacity: 0.
        set_overlay_cursor_passthrough(app_handle, true);
        debug!("hide_recording_overlay: set passthrough=true");
    } else {
        warn!("hide_recording_overlay: overlay window NOT FOUND!");
    }
}

/// Safely hides the overlay only if it is in Recording or Hidden state.
/// This prevents clobbering a transition to Transcribing, Processing, or Connecting.
pub fn hide_overlay_if_recording(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("hide_overlay_if_recording: entry");

    // Check state - bail if we are Transcribing, Processing, or Connecting
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        match *state {
            OverlayState::Transcribing
            | OverlayState::Processing
            | OverlayState::Connecting
            | OverlayState::Cancelling
            | OverlayState::Correcting => {
                debug!(
                    "hide_overlay_if_recording: Ignoring hide because state is {:?}",
                    *state
                );
                return;
            }
            _ => {
                // Proceed to hide
                *state = OverlayState::Hidden;
            }
        }
    }

    // Reuse the hide logic but with the check above
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let emit_result = overlay_window.emit("hide-overlay", ());
        debug!(
            "hide_overlay_if_recording: emit('hide-overlay') result={:?}",
            emit_result
        );

        // Use pass-through instead of hide.
        set_overlay_cursor_passthrough(app_handle, true);
    } else {
        warn!("hide_overlay_if_recording: overlay window NOT FOUND!");
    }
}

/// Shows the "connecting" overlay window (Connecting state)
/// This should appear IMMEDIATELY on fn press for instant visual feedback.
pub fn show_connecting_overlay(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("show_connecting_overlay: entry - showing IMMEDIATELY");

    // Update state to Connecting
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Connecting;
    }

    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_connecting_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // NOTE: We intentionally do NOT wait for OVERLAY_READY here.
        // The connecting overlay should appear IMMEDIATELY on first fn press.
        // Instant feedback is more important than perfect animation.

        // Use calculate_overlay_position to ensure correct placement
        if let Some((x, y)) = calculate_overlay_position(app_handle) {
            let _ = overlay_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }

        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        debug!(
            "show_connecting_overlay: is_visible_before={}",
            is_visible_before
        );

        // Enable interaction immediately.
        set_overlay_cursor_passthrough(app_handle, false);

        // Show window FIRST, then emit event
        // This ensures the user sees something immediately, even if React needs a moment
        if !is_visible_before {
            let show_result = overlay_window.show();
            debug!(
                "show_connecting_overlay: window.show() result={:?}",
                show_result
            );
        }

        // Emit event to switch to connecting state
        // React will update the content as soon as it processes this
        let emit_result = overlay_window.emit("show-overlay", "connecting");
        debug!("show_connecting_overlay: emit result={:?}", emit_result);

        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);
    } else {
        warn!("show_connecting_overlay: overlay window NOT FOUND!");
    }
}

/// Shows the cancelling overlay window (Cancelling state)
pub fn show_cancelling_overlay(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("show_cancelling_overlay: entry");

    // Update state to Cancelling
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        debug!(
            "show_cancelling_overlay: Transitioning state from {:?} to Cancelling",
            *state
        );
        *state = OverlayState::Cancelling;
    }

    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_cancelling_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        set_overlay_cursor_passthrough(app_handle, false);
        // Ensure visible if checks pass
        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        if !is_visible_before {
            debug!("show_cancelling_overlay: Window was hidden, showing it now");
            let _ = overlay_window.show();
        }

        // Emit event to switch to cancelling state
        let emit_result = overlay_window.emit("show-overlay", "cancelling");
        debug!(
            "show_cancelling_overlay: emit('show-overlay', 'cancelling') result={:?}",
            emit_result
        );

        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);
    } else {
        warn!("show_cancelling_overlay: overlay window NOT FOUND!");
    }
}

/// Safely hides the overlay after transcription/processing is done.
/// Only hides if the state is still Transcribing or Processing.
/// If the state has changed to Recording (new session started), it does NOT hide.
pub fn hide_overlay_after_transcription(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("hide_overlay_after_transcription: entry");

    // Check state - bail if we are Recording (new session)
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        match *state {
            OverlayState::Recording | OverlayState::Cancelling | OverlayState::Connecting => {
                debug!("hide_overlay_after_transcription: Ignoring hide because state is {:?} (active/new session)", *state);
                return;
            }
            _ => {
                // Proceed to hide (Transcribing, Processing, or already Hidden)
                *state = OverlayState::Hidden;
            }
        }
    }

    // Reuse the hide logic
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let emit_result = overlay_window.emit("hide-overlay", ());
        debug!(
            "hide_overlay_after_transcription: emit('hide-overlay') result={:?}",
            emit_result
        );

        // Use pass-through instead of hide.
        set_overlay_cursor_passthrough(app_handle, true);
    } else {
        warn!("hide_overlay_after_transcription: overlay window NOT FOUND!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_poll_hover_requires_interactive_visible_and_regions() {
        let mut state = OverlayHoverFallbackState::default();
        state.overlay_visible = true;
        assert!(!should_poll_hover_fallback(
            OverlayInputMode::InteractiveOperation,
            &state
        ));

        state.message_lane_rect = Some(OverlayScreenRect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        });
        #[cfg(target_os = "macos")]
        assert!(should_poll_hover_fallback(
            OverlayInputMode::InteractiveUndo,
            &state
        ));
        #[cfg(not(target_os = "macos"))]
        assert!(!should_poll_hover_fallback(
            OverlayInputMode::InteractiveUndo,
            &state
        ));
        assert!(!should_poll_hover_fallback(
            OverlayInputMode::Passthrough,
            &state
        ));
    }

    #[test]
    fn overlay_screen_rect_contains_logical_coords() {
        // Simulate a rect at logical position (600, 50) with size 200x40
        // This is what from_client_rect should produce after conversion:
        //   origin_x = physical(1200) / scale(2.0) = 600
        //   screen_x = origin_x(600) + client_rect.x(10) = 610
        let rect = OverlayScreenRect {
            x: 610.0,
            y: 52.0,
            width: 180.0,
            height: 36.0,
        };

        // Enigo cursor inside the rect
        assert!(rect.contains((620, 60)));
        assert!(rect.contains((610, 52))); // top-left corner
        assert!(rect.contains((790, 88))); // bottom-right corner

        // Enigo cursor outside the rect
        assert!(!rect.contains((600, 60))); // left of rect
        assert!(!rect.contains((620, 100))); // below rect
        assert!(!rect.contains((800, 60))); // right of rect
        assert!(!rect.contains((620, 40))); // above rect
    }
}
