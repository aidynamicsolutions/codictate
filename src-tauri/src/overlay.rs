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
use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel};

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(RecordingOverlayPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
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
                // don't subtract the overlay height it puts it too far up
                work_area_y + work_area_height - OVERLAY_BOTTOM_OFFSET
            }
        };

        return Some((x, y));
    }
    None
}

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    Recording,
    Transcribing,
    Processing,
}

static OVERLAY_STATE: Lazy<Mutex<OverlayState>> = Lazy::new(|| Mutex::new(OverlayState::Hidden));

/// Track whether the overlay webview is ready to receive events.
/// This prevents the race condition where show-overlay events are emitted
/// before the React component has registered its event listeners.
static OVERLAY_READY: AtomicBool = AtomicBool::new(false);

/// Mark the overlay as ready to receive events.
/// Called when the frontend emits the "overlay-ready" event.
pub fn mark_overlay_ready() {
    tracing::info!("mark_overlay_ready: Overlay webview signaled ready");
    OVERLAY_READY.store(true, Ordering::SeqCst);
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
    if let Some((x, y)) = calculate_overlay_position(app_handle) {
        match WebviewWindowBuilder::new(
            app_handle,
            "recording_overlay",
            tauri::WebviewUrl::App("src/overlay/index.html".into()),
        )
        .title("Recording")
        .position(x, y)
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
        .visible(false)
        .build()
        {
            Ok(_window) => {
                debug!("Recording overlay window created successfully (hidden)");
            }
            Err(e) => {
                debug!("Failed to create recording overlay window: {}", e);
            }
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
            .has_shadow(false)
            .transparent(true)
            .no_activate(true)
            .corner_radius(0.0)
            .with_window(|w| w.decorations(false).transparent(true))
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .build()
        {
            Ok(panel) => {
                let _ = panel.hide();
            }
            Err(e) => {
                tracing::error!("Failed to create recording overlay panel: {}", e);
            }
        }
    }
}

/// Shows the recording overlay window with fade-in animation
pub fn show_recording_overlay(app_handle: &AppHandle) {
    use tracing::{debug, warn};
    use std::time::Duration;
    
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
        if !OVERLAY_READY.load(Ordering::SeqCst) {
            debug!("show_recording_overlay: overlay not ready yet, waiting...");
            // Short spin-wait with timeout (max ~500ms, checking every 10ms)
            for _ in 0..50 {
                if OVERLAY_READY.load(Ordering::SeqCst) {
                    debug!("show_recording_overlay: overlay became ready");
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if !OVERLAY_READY.load(Ordering::SeqCst) {
                warn!("show_recording_overlay: timeout waiting for overlay ready, proceeding anyway");
            }
        }
        
        // Log current visibility state before showing
        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        debug!("show_recording_overlay: found window, is_visible_before={}", is_visible_before);
        
        // Enable interaction immediately
        if let Err(e) = overlay_window.set_ignore_cursor_events(false) {
            warn!("show_recording_overlay: failed to set_ignore_cursor_events(false): {}", e);
        }

        // Update position before showing to prevent flicker from position changes
        if let Some((x, y)) = calculate_overlay_position(app_handle) {
            let _ = overlay_window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }

        // IMPORTANT: Emit the show-overlay event.
        // If window is already visible, this triggers the React fade-in.
        let emit_result = overlay_window.emit("show-overlay", "recording");
        debug!("show_recording_overlay: emit('show-overlay', 'recording') result={:?}", emit_result);
        
        // If the window is NOT visible (first run or force hidden), show it safely
        if !is_visible_before {
            // Small delay to allow React to process the event and update opacity
            // before the native window becomes visible.
            std::thread::sleep(Duration::from_millis(50));
            
            let show_result = overlay_window.show();
            debug!("show_recording_overlay: window.show() result={:?}", show_result);
        } else {
             debug!("show_recording_overlay: window already visible, skipping native show() to avoid flicker");
        }

        // On Windows, aggressively re-assert "topmost" in the native Z-order after showing
        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);
        
        // Log post-show visibility
        let is_visible_after = overlay_window.is_visible().unwrap_or(false);
        debug!("show_recording_overlay: is_visible_after={}", is_visible_after);
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
        debug!("show_transcribing_overlay: found window, is_visible_before={}", is_visible_before);
        
        // Ensure interaction is enabled (just in case)
        let _ = overlay_window.set_ignore_cursor_events(false);

        if !is_visible_before {
            let show_result = overlay_window.show();
            debug!("show_transcribing_overlay: window.show() result={:?}", show_result);
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
        debug!("hide_recording_overlay: found window, is_visible_before={}", is_visible_before);
        
        // Emit event to trigger fade-out animation (CSS handles the 300ms transition)
        let emit_result = overlay_window.emit("hide-overlay", ());
        debug!("hide_recording_overlay: emit('hide-overlay') result={:?}", emit_result);
        
        // Instead of hiding the window (which causes flicker on next show),
        // we set it to ignore cursor events and let React render opacity: 0
        if let Err(e) = overlay_window.set_ignore_cursor_events(true) {
            warn!("hide_recording_overlay: failed to set_ignore_cursor_events(true): {}", e);
            // Fallback to hide if ignore events fails?
            // let _ = overlay_window.hide();
        } else {
            debug!("hide_recording_overlay: set_ignore_cursor_events(true) success");
        }
    } else {
        warn!("hide_recording_overlay: overlay window NOT FOUND!");
    }
}

/// Safely hides the overlay only if it is in Recording or Hidden state.
/// This prevents clobbering a transition to Transcribing or Processing.
pub fn hide_overlay_if_recording(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("hide_overlay_if_recording: entry");

    // Check state - bail if we are Transcribing or Processing
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        match *state {
            OverlayState::Transcribing | OverlayState::Processing => {
                debug!("hide_overlay_if_recording: Ignoring hide because state is {:?}", *state);
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
        debug!("hide_overlay_if_recording: emit('hide-overlay') result={:?}", emit_result);
        
        // Use ignore cursor events instead of hide
        if let Err(e) = overlay_window.set_ignore_cursor_events(true) {
             warn!("hide_overlay_if_recording: failed to set_ignore_cursor_events: {}", e);
             // let _ = overlay_window.hide();
        }
    } else {
        warn!("hide_overlay_if_recording: overlay window NOT FOUND!");
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
            OverlayState::Recording => {
                debug!("hide_overlay_after_transcription: Ignoring hide because state is Recording (new session started)");
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
        debug!("hide_overlay_after_transcription: emit('hide-overlay') result={:?}", emit_result);
        
        // Use ignore cursor events instead of hide
        if let Err(e) = overlay_window.set_ignore_cursor_events(true) {
             warn!("hide_overlay_after_transcription: failed to set_ignore_cursor_events: {}", e);
             // let _ = overlay_window.hide();
        }
    } else {
        warn!("hide_overlay_after_transcription: overlay window NOT FOUND!");
    }
}

