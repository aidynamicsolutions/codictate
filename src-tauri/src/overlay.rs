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

#[cfg(target_os = "linux")]
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
#[cfg(target_os = "linux")]
use std::env;

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
                work_area_y + work_area_height - OVERLAY_HEIGHT - OVERLAY_BOTTOM_OFFSET
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
    Connecting,
    Cancelling,
}

static OVERLAY_STATE: Lazy<Mutex<OverlayState>> = Lazy::new(|| Mutex::new(OverlayState::Hidden));

/// Check if the overlay is in any active state (not Hidden)
pub fn is_overlay_active() -> bool {
    if let Ok(state) = OVERLAY_STATE.lock() {
        !matches!(*state, OverlayState::Hidden)
    } else {
        false
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
        tracing::debug!("mark_overlay_ready: current backend state is {:?}", current_state);
        
        // If we are supposed to be showing something, re-emit the event
        match current_state {
            OverlayState::Recording => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!("mark_overlay_ready: State is Recording, re-emitting show-overlay");
                    let _ = overlay.emit("show-overlay", "recording");
                    
                    // Also ensure interaction is enabled
                    let _ = overlay.set_ignore_cursor_events(false);
                }
            },
            OverlayState::Transcribing => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!("mark_overlay_ready: State is Transcribing, re-emitting show-overlay");
                    let _ = overlay.emit("show-overlay", "transcribing");
                    let _ = overlay.set_ignore_cursor_events(false);
                }
            },
            OverlayState::Processing => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!("mark_overlay_ready: State is Processing, re-emitting show-overlay");
                    let _ = overlay.emit("show-overlay", "processing");
                }
            },
            OverlayState::Connecting => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!("mark_overlay_ready: State is Connecting, re-emitting show-overlay");
                    let _ = overlay.emit("show-overlay", "connecting");
                    let _ = overlay.set_ignore_cursor_events(false);
                }
            },
            OverlayState::Cancelling => {
                if let Some(overlay) = app_handle.get_webview_window("recording_overlay") {
                    tracing::debug!("mark_overlay_ready: State is Cancelling, re-emitting show-overlay");
                    let _ = overlay.emit("show-overlay", "cancelling");
                    // Keep interaction enabled or disabled based on preference, though usually cancelling is brief
                    let _ = overlay.set_ignore_cursor_events(false);
                }
            },
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
               
               // Force the panel to be visible (but transparent) immediately.
               // This ensures the WebView process is active (not App Napped) and mounts the React app at startup.
               // Since we set transparent(true) and the CSS defaults to opacity: 0, it will be invisible to the user.
               let _ = panel.show();
               
               // Verify visibility immediately
               if let Some(w) = app_handle.get_webview_window("recording_overlay") {
                   tracing::debug!("create_recording_overlay: Panel created. Visible? {}", w.is_visible().unwrap_or(false));
               } else {
                   tracing::warn!("create_recording_overlay: Panel created but get_webview_window failed!");
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
                    debug!("show_recording_overlay: overlay became ready in {:?}", wait_start.elapsed());
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
        debug!("show_recording_overlay: found window, is_visible_before={}", is_visible_before);
        
        // Enable interaction immediately
        if let Err(e) = overlay_window.set_ignore_cursor_events(false) {
            warn!("show_recording_overlay: failed to set_ignore_cursor_events(false): {}", e);
        }

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
        debug!("show_recording_overlay: emit('show-overlay', 'recording') result={:?}", emit_result);
        
        // If the window is NOT visible (first run or force hidden), show it safely
        if !is_visible_before {
            // Small delay to allow React to process the event and update opacity
            // before the native window becomes visible.
            std::thread::sleep(Duration::from_millis(50));
            
            let show_start = std::time::Instant::now();
            let show_result = overlay_window.show();
            debug!("show_recording_overlay: window.show() completed in {:?} result={:?}", show_start.elapsed(), show_result);
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
/// This prevents clobbering a transition to Transcribing, Processing, or Connecting.
pub fn hide_overlay_if_recording(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("hide_overlay_if_recording: entry");

    // Check state - bail if we are Transcribing, Processing, or Connecting
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        match *state {
            OverlayState::Transcribing | OverlayState::Processing | OverlayState::Connecting | OverlayState::Cancelling => {
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
        debug!("show_connecting_overlay: is_visible_before={}", is_visible_before);
        
        // Enable interaction immediately
        let _ = overlay_window.set_ignore_cursor_events(false);

        // Show window FIRST, then emit event
        // This ensures the user sees something immediately, even if React needs a moment
        if !is_visible_before {
            let show_result = overlay_window.show();
            debug!("show_connecting_overlay: window.show() result={:?}", show_result);
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
        debug!("show_cancelling_overlay: Transitioning state from {:?} to Cancelling", *state);
        *state = OverlayState::Cancelling;
    }

    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_cancelling_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Ensure visible if checks pass
        let is_visible_before = overlay_window.is_visible().unwrap_or(false);
        if !is_visible_before {
             debug!("show_cancelling_overlay: Window was hidden, showing it now");
             let _ = overlay_window.show();
        }
        
        // Emit event to switch to cancelling state
        let emit_result = overlay_window.emit("show-overlay", "cancelling");
        debug!("show_cancelling_overlay: emit('show-overlay', 'cancelling') result={:?}", emit_result);
        
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

