#[cfg(target_os = "macos")]
use crate::accessibility;
use crate::input;
use crate::settings;
use crate::settings::OverlayPosition;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(not(target_os = "macos"))]
use tracing::debug;

#[cfg(not(target_os = "macos"))]
use tauri::WebviewWindowBuilder;

#[cfg(target_os = "macos")]
use tauri::WebviewUrl;

#[cfg(target_os = "macos")]
use tauri_nspanel::{
    tauri_panel,
    CollectionBehavior,
    ManagerExt as PanelManagerExt,
    PanelBuilder,
    PanelLevel,
    StyleMask,
    TrackingAreaOptions,
    objc2_app_kit::{NSScreen, NSWindowCollectionBehavior},
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

#[cfg(target_os = "macos")]
const MACOS_OVERLAY_PANEL_LEVEL: PanelLevel = PanelLevel::PopUpMenu;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayMonitorSelectionReason {
    ActiveCursorMonitor,
    FocusedAxWindow,
    CachedLastSuccessfulScreen,
    OnboardingActivationTargetWindow,
    FocusedMainWindow,
    MouseCursorMonitor,
    PrimaryFallback,
}

impl OverlayMonitorSelectionReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::ActiveCursorMonitor => "active_cursor_monitor",
            Self::FocusedAxWindow => "focused_ax_window",
            Self::CachedLastSuccessfulScreen => "cached_last_successful_screen",
            Self::OnboardingActivationTargetWindow => "onboarding_activation_target_window",
            Self::FocusedMainWindow => "focused_main_window",
            Self::MouseCursorMonitor => "mouse_cursor_monitor",
            Self::PrimaryFallback => "primary_fallback",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct OverlayLogicalWorkArea {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
struct OverlayTargetPoint {
    x: f64,
    y: f64,
    reason: OverlayMonitorSelectionReason,
    is_fallback: bool,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
struct OverlayScreenTarget {
    frame: OverlayLogicalWorkArea,
    visible_frame: OverlayLogicalWorkArea,
    name: String,
    separate_spaces: bool,
    resolution_strategy: &'static str,
}

fn overlay_position_label(position: OverlayPosition) -> &'static str {
    match position {
        OverlayPosition::Top => "top",
        OverlayPosition::Bottom => "bottom",
        OverlayPosition::None => "none",
    }
}

fn read_onboarding_paste_override(app_handle: &AppHandle) -> bool {
    app_handle
        .try_state::<crate::OnboardingPasteOverride>()
        .and_then(|state| state.0.lock().ok().map(|value| *value))
        .unwrap_or(false)
}

fn read_onboarding_activation_target(app_handle: &AppHandle) -> bool {
    app_handle
        .try_state::<crate::OnboardingActivationTarget>()
        .and_then(|state| state.0.lock().ok().map(|value| *value))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn read_cached_overlay_target_point() -> Option<(f64, f64)> {
    LAST_SUCCESSFUL_OVERLAY_TARGET_POINT
        .lock()
        .ok()
        .and_then(|value| *value)
}

#[cfg(target_os = "macos")]
fn primary_monitor_frame_height(app_handle: &AppHandle) -> Option<f64> {
    app_handle
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor.size().height as f64 / monitor.scale_factor())
}

#[cfg(target_os = "macos")]
fn normalize_ax_point_to_appkit(frame_height: f64, x: f64, y: f64) -> (f64, f64) {
    (x, frame_height - y)
}

#[cfg(target_os = "macos")]
fn normalize_ax_rect_to_appkit(
    frame_height: f64,
    rect: OverlayLogicalWorkArea,
) -> OverlayLogicalWorkArea {
    OverlayLogicalWorkArea {
        x: rect.x,
        y: frame_height - rect.y - rect.height,
        width: rect.width,
        height: rect.height,
    }
}

#[cfg(target_os = "macos")]
fn is_degenerate_ax_rect(rect: OverlayLogicalWorkArea) -> bool {
    rect.width <= 0.0 && rect.height <= 0.0
}

#[cfg(target_os = "macos")]
const SCREEN_FRAME_EDGE_TOLERANCE: f64 = 2.0;

fn monitor_center_point(monitor: &tauri::Monitor) -> (f64, f64) {
    let work_area = logical_work_area(monitor);
    (
        work_area.x + work_area.width / 2.0,
        work_area.y + work_area.height / 2.0,
    )
}

#[cfg(target_os = "macos")]
fn resolve_overlay_target_point(
    app_handle: &AppHandle,
    include_cached_fallback: bool,
) -> Option<OverlayTargetPoint> {
    if let Some(target) = resolve_active_cursor_target_point(app_handle) {
        return Some(target);
    }

    if let Some(target) = resolve_focused_window_target_point(app_handle) {
        return Some(target);
    }

    if include_cached_fallback {
        if let Some((x, y)) = read_cached_overlay_target_point() {
            return Some(OverlayTargetPoint {
                x,
                y,
                reason: OverlayMonitorSelectionReason::CachedLastSuccessfulScreen,
                is_fallback: true,
            });
        }
    }

    if let Some(mouse_location) = input::get_cursor_position(app_handle) {
        return Some(OverlayTargetPoint {
            x: mouse_location.0 as f64,
            y: mouse_location.1 as f64,
            reason: OverlayMonitorSelectionReason::MouseCursorMonitor,
            is_fallback: true,
        });
    }

    if read_onboarding_activation_target(app_handle) {
        if let Some(main_window) = app_handle.get_webview_window("main") {
            if main_window.is_visible().unwrap_or(false) {
                if let Ok(Some(monitor)) = main_window.current_monitor() {
                    let (x, y) = monitor_center_point(&monitor);
                    return Some(OverlayTargetPoint {
                        x,
                        y,
                        reason: OverlayMonitorSelectionReason::OnboardingActivationTargetWindow,
                        is_fallback: true,
                    });
                }
            }
        }
    }

    app_handle
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            let (x, y) = monitor_center_point(&monitor);
            OverlayTargetPoint {
                x,
                y,
                reason: OverlayMonitorSelectionReason::PrimaryFallback,
                is_fallback: true,
            }
        })
}

#[cfg(target_os = "macos")]
fn resolve_active_cursor_target_point(app_handle: &AppHandle) -> Option<OverlayTargetPoint> {
    let probe = accessibility::capture_active_cursor_screen_probe(app_handle)?;
    let Some(frame_height) = primary_monitor_frame_height(app_handle) else {
        tracing::debug!(
            event_code = "overlay_target_candidate_rejected",
            source = "active_cursor",
            rejection_reason = "primary_monitor_unavailable",
            raw_point_x = probe.point_x,
            raw_point_y = probe.point_y,
            "Rejected active cursor overlay target because primary monitor frame height was unavailable"
        );
        return None;
    };

    let raw_rect = OverlayLogicalWorkArea {
        x: probe.rect_x,
        y: probe.rect_y,
        width: probe.rect_width,
        height: probe.rect_height,
    };
    let normalized_rect = normalize_ax_rect_to_appkit(frame_height, raw_rect);
    let (normalized_x, normalized_y) =
        normalize_ax_point_to_appkit(frame_height, probe.point_x, probe.point_y);

    if is_degenerate_ax_rect(raw_rect) {
        tracing::debug!(
            event_code = "overlay_target_candidate_rejected",
            source = "active_cursor",
            rejection_reason = "degenerate_ax_rect",
            raw_rect_x = raw_rect.x,
            raw_rect_y = raw_rect.y,
            raw_rect_width = raw_rect.width,
            raw_rect_height = raw_rect.height,
            normalized_point_x = normalized_x,
            normalized_point_y = normalized_y,
            "Rejected active cursor overlay target because AX returned a degenerate rect"
        );
        return None;
    }

    tracing::debug!(
        event_code = "overlay_target_candidate_accepted",
        source = "active_cursor",
        raw_rect_x = raw_rect.x,
        raw_rect_y = raw_rect.y,
        raw_rect_width = raw_rect.width,
        raw_rect_height = raw_rect.height,
        raw_point_x = probe.point_x,
        raw_point_y = probe.point_y,
        normalized_rect_x = normalized_rect.x,
        normalized_rect_y = normalized_rect.y,
        normalized_rect_width = normalized_rect.width,
        normalized_rect_height = normalized_rect.height,
        normalized_point_x = normalized_x,
        normalized_point_y = normalized_y,
        "Accepted normalized active cursor overlay target"
    );

    Some(OverlayTargetPoint {
        x: normalized_x,
        y: normalized_y,
        reason: OverlayMonitorSelectionReason::ActiveCursorMonitor,
        is_fallback: false,
    })
}

#[cfg(target_os = "macos")]
fn resolve_focused_window_target_point(app_handle: &AppHandle) -> Option<OverlayTargetPoint> {
    let frame = accessibility::capture_focused_window_screen_frame(app_handle)?;
    let Some(frame_height) = primary_monitor_frame_height(app_handle) else {
        tracing::debug!(
            event_code = "overlay_target_candidate_rejected",
            source = "focused_ax_window",
            rejection_reason = "primary_monitor_unavailable",
            raw_rect_x = frame.x,
            raw_rect_y = frame.y,
            raw_rect_width = frame.width,
            raw_rect_height = frame.height,
            "Rejected focused window overlay target because primary monitor frame height was unavailable"
        );
        return None;
    };

    let raw_rect = OverlayLogicalWorkArea {
        x: frame.x,
        y: frame.y,
        width: frame.width,
        height: frame.height,
    };
    let normalized_rect = normalize_ax_rect_to_appkit(frame_height, raw_rect);
    let normalized_x = normalized_rect.x + normalized_rect.width / 2.0;
    let normalized_y = normalized_rect.y + normalized_rect.height / 2.0;

    if normalized_rect.width <= 0.0 || normalized_rect.height <= 0.0 {
        tracing::debug!(
            event_code = "overlay_target_candidate_rejected",
            source = "focused_ax_window",
            rejection_reason = "degenerate_window_rect",
            raw_rect_x = raw_rect.x,
            raw_rect_y = raw_rect.y,
            raw_rect_width = raw_rect.width,
            raw_rect_height = raw_rect.height,
            normalized_rect_x = normalized_rect.x,
            normalized_rect_y = normalized_rect.y,
            normalized_rect_width = normalized_rect.width,
            normalized_rect_height = normalized_rect.height,
            "Rejected focused window overlay target because AX returned an unusable window rect"
        );
        return None;
    }

    tracing::debug!(
        event_code = "overlay_target_candidate_accepted",
        source = "focused_ax_window",
        raw_rect_x = raw_rect.x,
        raw_rect_y = raw_rect.y,
        raw_rect_width = raw_rect.width,
        raw_rect_height = raw_rect.height,
        normalized_rect_x = normalized_rect.x,
        normalized_rect_y = normalized_rect.y,
        normalized_rect_width = normalized_rect.width,
        normalized_rect_height = normalized_rect.height,
        normalized_point_x = normalized_x,
        normalized_point_y = normalized_y,
        "Accepted normalized focused window overlay target"
    );

    Some(OverlayTargetPoint {
        x: normalized_x,
        y: normalized_y,
        reason: OverlayMonitorSelectionReason::FocusedAxWindow,
        is_fallback: false,
    })
}

fn is_point_inside_work_area(work_area: OverlayLogicalWorkArea, x: f64, y: f64) -> bool {
    let max_x = work_area.x + work_area.width;
    let max_y = work_area.y + work_area.height;
    x >= work_area.x && x < max_x && y >= work_area.y && y < max_y
}

fn squared_distance_to_work_area(work_area: OverlayLogicalWorkArea, x: f64, y: f64) -> f64 {
    let max_x = work_area.x + work_area.width;
    let max_y = work_area.y + work_area.height;

    let dx = if x < work_area.x {
        work_area.x - x
    } else if x > max_x {
        x - max_x
    } else {
        0.0
    };

    let dy = if y < work_area.y {
        work_area.y - y
    } else if y > max_y {
        y - max_y
    } else {
        0.0
    };

    dx * dx + dy * dy
}

fn squared_distance_to_work_area_center(work_area: OverlayLogicalWorkArea, x: f64, y: f64) -> f64 {
    let center_x = work_area.x + (work_area.width / 2.0);
    let center_y = work_area.y + (work_area.height / 2.0);
    let dx = center_x - x;
    let dy = center_y - y;
    dx * dx + dy * dy
}

fn resolve_monitor_for_point_strict(
    app_handle: &AppHandle,
    x: f64,
    y: f64,
    source: &'static str,
) -> Option<tauri::Monitor> {
    if let Ok(Some(monitor)) = app_handle.monitor_from_point(x, y) {
        tracing::debug!(
            event_code = "overlay_monitor_point_resolved",
            source = source,
            strategy = "runtime_monitor_from_point",
            point_x = x,
            point_y = y,
            monitor_name = monitor.name().map(|name| name.as_str()).unwrap_or("unknown"),
            monitor_scale = monitor.scale_factor(),
            "Resolved monitor from runtime point lookup"
        );
        return Some(monitor);
    }

    let monitors = match app_handle.available_monitors() {
        Ok(monitors) => monitors,
        Err(error) => {
            tracing::debug!(
                event_code = "overlay_monitor_point_resolution_failed",
                source = source,
                point_x = x,
                point_y = y,
                error = %error,
                "Failed to enumerate monitors while resolving monitor for point"
            );
            return None;
        }
    };

    for monitor in monitors {
        let work_area = logical_work_area(&monitor);
        if is_point_inside_work_area(work_area, x, y) {
            tracing::debug!(
                event_code = "overlay_monitor_point_resolved",
                source = source,
                strategy = "logical_work_area_contains",
                point_x = x,
                point_y = y,
                monitor_name = monitor.name().map(|name| name.as_str()).unwrap_or("unknown"),
                monitor_scale = monitor.scale_factor(),
                "Resolved monitor from logical work area containment"
            );
            return Some(monitor);
        }
    }

    tracing::debug!(
        event_code = "overlay_monitor_point_rejected",
        source = source,
        point_x = x,
        point_y = y,
        "Rejected monitor point because it does not intersect any monitor work area"
    );

    None
}

fn resolve_monitor_for_point(
    app_handle: &AppHandle,
    x: f64,
    y: f64,
    source: &'static str,
) -> Option<tauri::Monitor> {
    if let Some(monitor) = resolve_monitor_for_point_strict(app_handle, x, y, source) {
        return Some(monitor);
    }

    let monitors = match app_handle.available_monitors() {
        Ok(monitors) => monitors,
        Err(error) => {
            tracing::debug!(
                event_code = "overlay_monitor_point_resolution_failed",
                source = source,
                point_x = x,
                point_y = y,
                error = %error,
                "Failed to enumerate monitors while resolving monitor for point"
            );
            return None;
        }
    };

    let mut best_monitor: Option<(f64, f64, tauri::Monitor)> = None;
    for monitor in monitors {
        let work_area = logical_work_area(&monitor);
        let distance_sq = squared_distance_to_work_area(work_area, x, y);
        let center_distance_sq = squared_distance_to_work_area_center(work_area, x, y);
        match &best_monitor {
            Some((best_distance, best_center_distance, _))
                if distance_sq > *best_distance
                    || (distance_sq == *best_distance
                        && center_distance_sq >= *best_center_distance) => {}
            _ => best_monitor = Some((distance_sq, center_distance_sq, monitor)),
        }
    }

    if let Some((distance_sq, center_distance_sq, monitor)) = best_monitor {
        tracing::debug!(
            event_code = "overlay_monitor_point_resolved_by_nearest_work_area",
            source = source,
            point_x = x,
            point_y = y,
            distance_sq = distance_sq,
            center_distance_sq = center_distance_sq,
            selected_monitor_name = monitor.name().map(|name| name.as_str()).unwrap_or("unknown"),
            selected_monitor_scale = monitor.scale_factor(),
            "Resolved monitor by nearest logical work area"
        );
        return Some(monitor);
    }

    None
}

#[cfg(target_os = "macos")]
fn get_monitor_with_active_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    let target = resolve_active_cursor_target_point(app_handle)?;
    resolve_monitor_for_point_strict(app_handle, target.x, target.y, "active_cursor")
}

#[cfg(not(target_os = "macos"))]
fn get_monitor_with_active_cursor(_app_handle: &AppHandle) -> Option<tauri::Monitor> {
    None
}

fn get_monitor_with_mouse_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    if let Some(mouse_location) = input::get_cursor_position(app_handle) {
        return resolve_monitor_for_point(
            app_handle,
            mouse_location.0 as f64,
            mouse_location.1 as f64,
            "mouse_cursor",
        );
    }

    None
}

fn resolve_overlay_target_monitor(
    app_handle: &AppHandle,
) -> Option<(tauri::Monitor, OverlayMonitorSelectionReason)> {
    if let Some(monitor) = get_monitor_with_active_cursor(app_handle) {
        return Some((monitor, OverlayMonitorSelectionReason::ActiveCursorMonitor));
    }

    if let Some(main_window) = app_handle.get_webview_window("main") {
        let main_visible = main_window.is_visible().unwrap_or(false);
        if main_visible {
            if read_onboarding_activation_target(app_handle) {
                if let Ok(Some(monitor)) = main_window.current_monitor() {
                    return Some((
                        monitor,
                        OverlayMonitorSelectionReason::OnboardingActivationTargetWindow,
                    ));
                }
            }

            let main_focused = main_window.is_focused().unwrap_or(false);
            if main_focused {
                if let Ok(Some(monitor)) = main_window.current_monitor() {
                    return Some((monitor, OverlayMonitorSelectionReason::FocusedMainWindow));
                }
            }
        }
    }

    if let Some(monitor) = get_monitor_with_mouse_cursor(app_handle) {
        return Some((monitor, OverlayMonitorSelectionReason::MouseCursorMonitor));
    }

    app_handle
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| (monitor, OverlayMonitorSelectionReason::PrimaryFallback))
}

fn logical_work_area(monitor: &tauri::Monitor) -> OverlayLogicalWorkArea {
    let work_area = monitor.work_area();
    let scale = monitor.scale_factor();
    OverlayLogicalWorkArea {
        x: work_area.position.x as f64 / scale,
        y: work_area.position.y as f64 / scale,
        width: work_area.size.width as f64 / scale,
        height: work_area.size.height as f64 / scale,
    }
}

fn compute_overlay_position_for_work_area(
    work_area: OverlayLogicalWorkArea,
    overlay_position: OverlayPosition,
) -> (f64, f64) {
    let centered_x = work_area.x + (work_area.width - OVERLAY_WIDTH) / 2.0;
    let max_x = work_area.x + (work_area.width - OVERLAY_WIDTH).max(0.0);
    let x = centered_x.clamp(work_area.x, max_x);

    let target_y = match overlay_position {
        OverlayPosition::Top => work_area.y + OVERLAY_TOP_OFFSET,
        OverlayPosition::Bottom | OverlayPosition::None => {
            work_area.y + work_area.height - OVERLAY_HEIGHT - OVERLAY_BOTTOM_OFFSET
        }
    };
    let max_y = work_area.y + (work_area.height - OVERLAY_HEIGHT).max(0.0);
    let y = target_y.clamp(work_area.y, max_y);

    (x, y)
}

#[cfg(target_os = "macos")]
fn compute_overlay_position_for_visible_frame(
    visible_frame: OverlayLogicalWorkArea,
    overlay_position: OverlayPosition,
) -> (f64, f64) {
    let centered_x = visible_frame.x + (visible_frame.width - OVERLAY_WIDTH) / 2.0;
    let max_x = visible_frame.x + (visible_frame.width - OVERLAY_WIDTH).max(0.0);
    let x = centered_x.clamp(visible_frame.x, max_x);

    let target_y = match overlay_position {
        OverlayPosition::Top => {
            visible_frame.y + visible_frame.height - OVERLAY_HEIGHT - OVERLAY_TOP_OFFSET
        }
        OverlayPosition::Bottom | OverlayPosition::None => visible_frame.y + OVERLAY_BOTTOM_OFFSET,
    };
    let max_y = visible_frame.y + (visible_frame.height - OVERLAY_HEIGHT).max(0.0);
    let y = target_y.clamp(visible_frame.y, max_y);

    (x, y)
}

fn calculate_overlay_position(app_handle: &AppHandle) -> Option<(f64, f64)> {
    let (monitor, reason) = resolve_overlay_target_monitor(app_handle)?;
    let work_area = logical_work_area(&monitor);
    let settings = settings::get_settings(app_handle);
    let (x, y) = compute_overlay_position_for_work_area(work_area, settings.overlay_position);

    tracing::debug!(
        event_code = "overlay_position_resolved",
        reason = reason.as_str(),
        position_mode = overlay_position_label(settings.overlay_position),
        monitor_name = monitor.name().map(|name| name.as_str()).unwrap_or("unknown"),
        monitor_scale = monitor.scale_factor(),
        monitor_position_x = monitor.position().x,
        monitor_position_y = monitor.position().y,
        monitor_width = monitor.size().width,
        monitor_height = monitor.size().height,
        work_area_x = work_area.x,
        work_area_y = work_area.y,
        work_area_width = work_area.width,
        work_area_height = work_area.height,
        overlay_width = OVERLAY_WIDTH,
        overlay_height = OVERLAY_HEIGHT,
        x = x,
        y = y,
        "Resolved overlay screen position"
    );

    Some((x, y))
}

#[cfg(not(target_os = "macos"))]
fn apply_overlay_position_with_logging(
    app_handle: &AppHandle,
    overlay_window: &tauri::WebviewWindow,
    source: &'static str,
) {
    if let Some((x, y)) = calculate_overlay_position(app_handle) {
        let set_result =
            overlay_window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        let actual_position = overlay_window.outer_position().ok();
        let scale_factor = overlay_window.scale_factor().ok();
        tracing::debug!(
            event_code = "overlay_window_position_applied",
            source = source,
            requested_x = x,
            requested_y = y,
            set_position_ok = set_result.is_ok(),
            actual_outer_x = actual_position.as_ref().map(|position| position.x),
            actual_outer_y = actual_position.as_ref().map(|position| position.y),
            actual_scale_factor = scale_factor,
            "Applied overlay window position"
        );
    }
}

#[cfg(target_os = "macos")]
fn overlay_panel_collection_behavior() -> NSWindowCollectionBehavior {
    CollectionBehavior::new()
        .move_to_active_space()
        .full_screen_auxiliary()
        .value()
}

#[cfg(target_os = "macos")]
fn work_area_from_ns_rect(rect: NSRect) -> OverlayLogicalWorkArea {
    OverlayLogicalWorkArea {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    }
}

fn rects_intersect(a: OverlayLogicalWorkArea, b: OverlayLogicalWorkArea) -> bool {
    let a_max_x = a.x + a.width;
    let a_max_y = a.y + a.height;
    let b_max_x = b.x + b.width;
    let b_max_y = b.y + b.height;
    a.x < b_max_x && a_max_x > b.x && a.y < b_max_y && a_max_y > b.y
}

#[cfg(target_os = "macos")]
fn is_point_inside_work_area_with_tolerance(
    work_area: OverlayLogicalWorkArea,
    x: f64,
    y: f64,
    tolerance: f64,
) -> bool {
    let max_x = work_area.x + work_area.width;
    let max_y = work_area.y + work_area.height;
    x >= work_area.x - tolerance
        && x <= max_x + tolerance
        && y >= work_area.y - tolerance
        && y <= max_y + tolerance
}

#[cfg(target_os = "macos")]
fn resolve_screen_target_for_point(
    mtm: tauri_nspanel::objc2::MainThreadMarker,
    x: f64,
    y: f64,
) -> Option<OverlayScreenTarget> {
    let separate_spaces = NSScreen::screensHaveSeparateSpaces(mtm);
    let screens = NSScreen::screens(mtm);
    let mut best_match: Option<(f64, f64, OverlayScreenTarget)> = None;

    for screen in screens.iter() {
        let frame = work_area_from_ns_rect(screen.frame());
        let visible_frame = work_area_from_ns_rect(screen.visibleFrame());
        let name = screen.localizedName().to_string();
        if is_point_inside_work_area(frame, x, y) {
            return Some(OverlayScreenTarget {
                frame,
                visible_frame,
                name,
                separate_spaces,
                resolution_strategy: "screen_frame_strict",
            });
        }

        if is_point_inside_work_area_with_tolerance(frame, x, y, SCREEN_FRAME_EDGE_TOLERANCE) {
            return Some(OverlayScreenTarget {
                frame,
                visible_frame,
                name,
                separate_spaces,
                resolution_strategy: "screen_frame_tolerant",
            });
        }

        let distance_sq = squared_distance_to_work_area(frame, x, y);
        let center_distance_sq = squared_distance_to_work_area_center(frame, x, y);
        match &best_match {
            Some((best_distance_sq, best_center_distance_sq, _))
                if distance_sq > *best_distance_sq
                    || (distance_sq == *best_distance_sq
                        && center_distance_sq >= *best_center_distance_sq) => {}
            _ => {
                best_match = Some((
                    distance_sq,
                    center_distance_sq,
                    OverlayScreenTarget {
                        frame,
                        visible_frame,
                        name,
                        separate_spaces,
                        resolution_strategy: "screen_frame_nearest",
                    },
                ))
            }
        }
    }

    best_match.map(|(_, _, target)| target)
}

#[cfg(target_os = "macos")]
fn schedule_overlay_presentation_sample(
    app_handle: &AppHandle,
    source: &'static str,
    reason: OverlayMonitorSelectionReason,
    presentation_seq: u64,
    requested_frame: OverlayLogicalWorkArea,
) {
    let app = app_handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(
            OVERLAY_PRESENTATION_SAMPLE_DELAY_MS,
        ));
        if OVERLAY_PRESENTATION_SEQ.load(Ordering::SeqCst) != presentation_seq {
            return;
        }

        let app_for_main = app.clone();
        let _ = app.run_on_main_thread(move || {
            if OVERLAY_PRESENTATION_SEQ.load(Ordering::SeqCst) != presentation_seq {
                return;
            }

            let Ok(panel) = app_for_main.get_webview_panel("recording_overlay") else {
                tracing::warn!(
                    event_code = "overlay_panel_present_sample_skipped",
                    source = source,
                    presentation_seq = presentation_seq,
                    reason = "panel_not_found",
                    "Skipped deferred overlay panel sampling because panel was not found"
                );
                return;
            };

            let actual_frame = work_area_from_ns_rect(panel.as_panel().frame());
            let actual_screen = panel.as_panel().screen();
            let actual_screen_name = actual_screen
                .as_ref()
                .map(|screen| screen.localizedName().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let actual_screen_visible_frame = actual_screen
                .as_ref()
                .map(|screen| work_area_from_ns_rect(screen.visibleFrame()));
            let intersects_target_screen = actual_screen_visible_frame
                .map(|visible_frame| rects_intersect(actual_frame, visible_frame))
                .unwrap_or(false);

            tracing::debug!(
                event_code = "overlay_panel_present_sampled",
                source = source,
                presentation_seq = presentation_seq,
                reason = reason.as_str(),
                requested_x = requested_frame.x,
                requested_y = requested_frame.y,
                requested_width = requested_frame.width,
                requested_height = requested_frame.height,
                actual_x = actual_frame.x,
                actual_y = actual_frame.y,
                actual_width = actual_frame.width,
                actual_height = actual_frame.height,
                actual_screen_name = actual_screen_name,
                actual_screen_visible_x = actual_screen_visible_frame.map(|frame| frame.x),
                actual_screen_visible_y = actual_screen_visible_frame.map(|frame| frame.y),
                actual_screen_visible_width = actual_screen_visible_frame.map(|frame| frame.width),
                actual_screen_visible_height = actual_screen_visible_frame.map(|frame| frame.height),
                intersects_target_screen = intersects_target_screen,
                "Sampled overlay panel frame after presentation"
            );
        });
    });
}

#[cfg(target_os = "macos")]
fn dispatch_overlay_panel_presentation(
    app_handle: &AppHandle,
    source: &'static str,
    target: OverlayTargetPoint,
    presentation_seq: u64,
) {
    let app = app_handle.clone();
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        return;
    }

    if let Err(error) = app_handle.run_on_main_thread(move || {
        let mtm = tauri_nspanel::objc2::MainThreadMarker::new()
            .expect("overlay panel presentation must run on main thread");
        let Ok(panel) = app.get_webview_panel("recording_overlay") else {
            tracing::warn!(
                event_code = "overlay_panel_present_skipped",
                source = source,
                presentation_seq = presentation_seq,
                reason = "panel_not_found",
                "Skipped overlay panel presentation because panel was not found"
            );
            return;
        };

        let mut screen_target = resolve_screen_target_for_point(mtm, target.x, target.y);
        if matches!(
            screen_target.as_ref().map(|screen| screen.resolution_strategy),
            Some("screen_frame_nearest")
        ) && !target.is_fallback
        {
            tracing::debug!(
                event_code = "overlay_panel_present_candidate_rejected",
                source = source,
                presentation_seq = presentation_seq,
                reason = target.reason.as_str(),
                target_x = target.x,
                target_y = target.y,
                rejection_reason = "outside_screen_frames",
                "Rejected authoritative overlay target because it did not intersect any AppKit screen frame"
            );
            screen_target = None;
        }

        let Some(screen_target) = screen_target
        else {
            tracing::warn!(
                event_code = "overlay_panel_present_skipped",
                source = source,
                presentation_seq = presentation_seq,
                reason = "screen_resolution_failed",
                target_x = target.x,
                target_y = target.y,
                "Skipped overlay panel presentation because no screen matched the target point"
            );
            return;
        };

        let (frame_x, frame_y) =
            compute_overlay_position_for_visible_frame(
                screen_target.visible_frame,
                settings.overlay_position,
            );
        let requested_frame = OverlayLogicalWorkArea {
            x: frame_x,
            y: frame_y,
            width: OVERLAY_WIDTH,
            height: OVERLAY_HEIGHT,
        };

        let panel_was_visible = panel.is_visible();
        panel.set_collection_behavior(overlay_panel_collection_behavior());
        panel.set_level(MACOS_OVERLAY_PANEL_LEVEL.value());
        panel.as_panel().setFrame_display(
            tauri_nspanel::objc2_foundation::NSRect::new(
                tauri_nspanel::objc2_foundation::NSPoint::new(frame_x, frame_y),
                tauri_nspanel::objc2_foundation::NSSize::new(OVERLAY_WIDTH, OVERLAY_HEIGHT),
            ),
            true,
        );
        if !panel_was_visible {
            panel.show();
        }
        panel.order_front_regardless();

        if let Ok(mut cached_target_point) = LAST_SUCCESSFUL_OVERLAY_TARGET_POINT.lock() {
            *cached_target_point = Some((
                screen_target.frame.x + screen_target.frame.width / 2.0,
                screen_target.frame.y + screen_target.frame.height / 2.0,
            ));
        }

        tracing::debug!(
            event_code = "overlay_panel_present_dispatched",
            source = source,
            presentation_seq = presentation_seq,
            reason = target.reason.as_str(),
            used_fallback = target.is_fallback,
            target_point_x = target.x,
            target_point_y = target.y,
            screen_name = screen_target.name,
            separate_spaces = screen_target.separate_spaces,
            screen_frame_x = screen_target.frame.x,
            screen_frame_y = screen_target.frame.y,
            screen_frame_width = screen_target.frame.width,
            screen_frame_height = screen_target.frame.height,
            visible_x = screen_target.visible_frame.x,
            visible_y = screen_target.visible_frame.y,
            visible_width = screen_target.visible_frame.width,
            visible_height = screen_target.visible_frame.height,
            screen_resolution_strategy = screen_target.resolution_strategy,
            requested_x = requested_frame.x,
            requested_y = requested_frame.y,
            requested_width = requested_frame.width,
            requested_height = requested_frame.height,
            panel_was_visible = panel_was_visible,
            "Dispatched overlay panel presentation"
        );

        schedule_overlay_presentation_sample(&app, source, target.reason, presentation_seq, requested_frame);
    }) {
        tracing::warn!(
            event_code = "overlay_panel_present_dispatch_failed",
            source = source,
            presentation_seq = presentation_seq,
            error = %error,
            "Failed to dispatch overlay panel presentation to the main thread"
        );
    }
}

#[cfg(target_os = "macos")]
const OVERLAY_TARGET_REFRESH_DELAY_MS: u64 = 80;

#[cfg(target_os = "macos")]
fn schedule_overlay_target_refresh(
    app_handle: &AppHandle,
    source: &'static str,
    presentation_seq: u64,
    initial_target: OverlayTargetPoint,
) {
    let app = app_handle.clone();
    std::thread::spawn(move || {
        // Wait before retrying so the AX server has time to stabilize after the
        // initial query that triggered a fallback.
        std::thread::sleep(Duration::from_millis(OVERLAY_TARGET_REFRESH_DELAY_MS));

        if OVERLAY_PRESENTATION_SEQ.load(Ordering::SeqCst) != presentation_seq {
            return;
        }

        let Some(refined_target) = resolve_overlay_target_point(&app, false) else {
            return;
        };

        if OVERLAY_PRESENTATION_SEQ.load(Ordering::SeqCst) != presentation_seq {
            return;
        }

        if matches!(
            OVERLAY_STATE.lock().ok().map(|state| *state),
            Some(OverlayState::Hidden)
        ) {
            return;
        }

        if (refined_target.x - initial_target.x).abs() < 1.0
            && (refined_target.y - initial_target.y).abs() < 1.0
        {
            return;
        }

        dispatch_overlay_panel_presentation(
            &app,
            source,
            refined_target,
            presentation_seq,
        );
    });
}

#[cfg(target_os = "macos")]
fn present_overlay_panel(app_handle: &AppHandle, source: &'static str) {
    let target = resolve_overlay_target_point(app_handle, true);
    let Some(target) = target else {
        tracing::warn!(
            event_code = "overlay_panel_present_skipped",
            source = source,
            reason = "target_unavailable",
            "Skipped overlay panel presentation because no target point was available"
        );
        return;
    };

    let presentation_seq = OVERLAY_PRESENTATION_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    dispatch_overlay_panel_presentation(app_handle, source, target, presentation_seq);
    if target.is_fallback {
        schedule_overlay_target_refresh(app_handle, source, presentation_seq, target);
    }
}

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
static LAST_TRANSCRIBING_SHOWN_AT: LazyLock<Mutex<Option<Instant>>> =
    LazyLock::new(|| Mutex::new(None));
#[cfg(target_os = "macos")]
static LAST_SUCCESSFUL_OVERLAY_TARGET_POINT: LazyLock<Mutex<Option<(f64, f64)>>> =
    LazyLock::new(|| Mutex::new(None));
#[cfg(target_os = "macos")]
static OVERLAY_PRESENTATION_SEQ: AtomicU64 = AtomicU64::new(0);

const OVERLAY_HOVER_ACTIVE_POLL_INTERVAL_MS: u64 = 33;
const OVERLAY_HOVER_IDLE_POLL_INTERVAL_MS: u64 = 180;
const ONBOARDING_TRANSCRIBING_MIN_VISIBLE_MS: u64 = 350;
#[cfg(target_os = "macos")]
const OVERLAY_PRESENTATION_SAMPLE_DELAY_MS: u64 = 16;

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

/// Returns true when overlay replay-sensitive listeners are attached.
/// This is stricter than `is_overlay_available` and should gate event streams
/// that are not replayed automatically (e.g. undo cards).
fn is_overlay_replay_available(app_handle: &AppHandle) -> bool {
    if !is_overlay_available(app_handle) {
        return false;
    }

    OVERLAY_REPLAY_READY.load(Ordering::SeqCst)
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
    if !is_overlay_replay_available(app_handle) {
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
/// Track whether replay-sensitive listeners are attached.
/// This can lag behind `OVERLAY_READY` during mount/remount.
static OVERLAY_REPLAY_READY: AtomicBool = AtomicBool::new(false);

/// Mark the overlay as ready for visibility events only.
/// This is emitted early by the frontend after core show/hide listeners are attached.
/// Replay-sensitive state re-emission should happen via `mark_overlay_ready`.
pub fn mark_overlay_listener_ready() {
    OVERLAY_READY.store(true, Ordering::SeqCst);
    OVERLAY_REPLAY_READY.store(false, Ordering::SeqCst);
    tracing::debug!("mark_overlay_listener_ready: overlay can receive visibility events");
}

/// Mark the overlay as fully ready to receive replay-sensitive events.
/// Called when the frontend emits the "overlay-fully-ready" event.
/// If the current state is not Hidden, we re-emit the "show-overlay" event
/// to ensure the reloaded/remounted frontend receives the correct state.
pub fn mark_overlay_ready(app_handle: &AppHandle) {
    use tauri::Manager;

    tracing::info!("mark_overlay_ready: Overlay webview signaled ready");
    OVERLAY_READY.store(true, Ordering::SeqCst);
    OVERLAY_REPLAY_READY.store(true, Ordering::SeqCst);

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
            .level(MACOS_OVERLAY_PANEL_LEVEL)
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
            .collection_behavior(CollectionBehavior::from_raw(
                overlay_panel_collection_behavior(),
            ))
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
    use tracing::{debug, warn};

    debug!("show_recording_overlay: entry");

    // Update state to Recording
    if let Ok(mut state) = OVERLAY_STATE.lock() {
        *state = OverlayState::Recording;
    }
    if let Ok(mut shown_at) = LAST_TRANSCRIBING_SHOWN_AT.lock() {
        *shown_at = None;
    }

    // Check if overlay should be shown based on position setting
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_recording_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        if !OVERLAY_READY.load(Ordering::SeqCst) {
            debug!("show_recording_overlay: overlay listener not ready yet; proceeding without blocking");
        }

        // Enable interaction immediately.
        set_overlay_cursor_passthrough(app_handle, false);

        #[cfg(target_os = "linux")]
        {
            update_gtk_layer_shell_anchors(&overlay_window);
        }

        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "show_recording_overlay");
        #[cfg(not(target_os = "macos"))]
        apply_overlay_position_with_logging(app_handle, &overlay_window, "show_recording_overlay");
        #[cfg(not(target_os = "macos"))]
        {
            let is_visible_before = overlay_window.is_visible().unwrap_or(false);
            debug!(
                "show_recording_overlay: found window, is_visible_before={}",
                is_visible_before
            );
            if !is_visible_before {
                let show_result = overlay_window.show();
                debug!(
                    "show_recording_overlay: window.show() result={:?}",
                    show_result
                );
            }
        }

        let emit_result = overlay_window.emit("show-overlay", "recording");
        debug!(
            "show_recording_overlay: emit('show-overlay', 'recording') result={:?}",
            emit_result
        );

        // On Windows, aggressively re-assert "topmost" in the native Z-order after showing
        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);
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
    if let Ok(mut shown_at) = LAST_TRANSCRIBING_SHOWN_AT.lock() {
        *shown_at = Some(Instant::now());
    }

    // Check if overlay should be shown based on position setting
    let settings = settings::get_settings(app_handle);
    if settings.overlay_position == OverlayPosition::None {
        debug!("show_transcribing_overlay: overlay disabled in settings, skipping");
        return;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        set_overlay_cursor_passthrough(app_handle, false);
        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "show_transcribing_overlay");
        #[cfg(not(target_os = "macos"))]
        update_overlay_position(app_handle);
        #[cfg(not(target_os = "macos"))]
        {
            let is_visible_before = overlay_window.is_visible().unwrap_or(false);
            debug!(
                "show_transcribing_overlay: found window, is_visible_before={}",
                is_visible_before
            );
            if !is_visible_before {
                let show_result = overlay_window.show();
                debug!(
                    "show_transcribing_overlay: window.show() result={:?}",
                    show_result
                );
            }
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
        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "show_correction_overlay");
        #[cfg(not(target_os = "macos"))]
        apply_overlay_position_with_logging(app_handle, &overlay_window, "show_correction_overlay");
        #[cfg(not(target_os = "macos"))]
        if !overlay_window.is_visible().unwrap_or(false) {
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
        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "show_processing_overlay");
        #[cfg(not(target_os = "macos"))]
        if !overlay_window.is_visible().unwrap_or(false) {
            let _ = overlay_window.show();
        }
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

        #[cfg(target_os = "macos")]
        let _ = &overlay_window;
        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "update_overlay_position");
        #[cfg(not(target_os = "macos"))]
        apply_overlay_position_with_logging(app_handle, &overlay_window, "update_overlay_position");
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

fn should_hide_overlay_after_aborted_recording_state(state: OverlayState) -> bool {
    matches!(
        state,
        OverlayState::Hidden
            | OverlayState::Recording
            | OverlayState::Connecting
    )
}

/// Safely hides an overlay left behind by an aborted recording stop before transcription starts.
/// This only applies to pre-transcription states so we do not clobber active work.
pub fn hide_overlay_after_aborted_recording(app_handle: &AppHandle) {
    use tracing::{debug, warn};

    debug!("hide_overlay_after_aborted_recording: entry");

    if let Ok(mut state) = OVERLAY_STATE.lock() {
        if !should_hide_overlay_after_aborted_recording_state(*state) {
            debug!(
                "hide_overlay_after_aborted_recording: Ignoring hide because state is {:?}",
                *state
            );
            return;
        }

        *state = OverlayState::Hidden;
    }

    if let Ok(mut shown_at) = LAST_TRANSCRIBING_SHOWN_AT.lock() {
        *shown_at = None;
    }

    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        let emit_result = overlay_window.emit("hide-overlay", ());
        debug!(
            "hide_overlay_after_aborted_recording: emit('hide-overlay') result={:?}",
            emit_result
        );

        set_overlay_cursor_passthrough(app_handle, true);
    } else {
        warn!("hide_overlay_after_aborted_recording: overlay window NOT FOUND!");
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
        // Enable interaction immediately.
        set_overlay_cursor_passthrough(app_handle, false);
        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "show_connecting_overlay");
        #[cfg(not(target_os = "macos"))]
        apply_overlay_position_with_logging(app_handle, &overlay_window, "show_connecting_overlay");
        #[cfg(not(target_os = "macos"))]
        if !overlay_window.is_visible().unwrap_or(false) {
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
        #[cfg(target_os = "macos")]
        present_overlay_panel(app_handle, "show_cancelling_overlay");
        #[cfg(not(target_os = "macos"))]
        if !overlay_window.is_visible().unwrap_or(false) {
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

fn onboarding_transcribing_hide_delay_remaining(app_handle: &AppHandle) -> Option<Duration> {
    if !read_onboarding_paste_override(app_handle) {
        return None;
    }

    let is_transcribing = OVERLAY_STATE
        .lock()
        .ok()
        .map(|state| *state == OverlayState::Transcribing)
        .unwrap_or(false);
    if !is_transcribing {
        return None;
    }

    let shown_at = LAST_TRANSCRIBING_SHOWN_AT.lock().ok().and_then(|value| *value)?;
    let elapsed = shown_at.elapsed();
    let minimum = Duration::from_millis(ONBOARDING_TRANSCRIBING_MIN_VISIBLE_MS);
    if elapsed >= minimum {
        return None;
    }

    Some(minimum - elapsed)
}

fn hide_overlay_after_transcription_inner(app_handle: &AppHandle, allow_onboarding_delay: bool) {
    use tracing::{debug, warn};

    debug!("hide_overlay_after_transcription: entry");

    if allow_onboarding_delay {
        if let Some(remaining_delay) = onboarding_transcribing_hide_delay_remaining(app_handle) {
            debug!(
                delay_ms = remaining_delay.as_millis(),
                "Delaying onboarding transcribing overlay hide to preserve perceptibility"
            );
            let app_for_delay = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(remaining_delay);
                hide_overlay_after_transcription_inner(&app_for_delay, false);
            });
            return;
        }
    }

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
    if let Ok(mut shown_at) = LAST_TRANSCRIBING_SHOWN_AT.lock() {
        *shown_at = None;
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

/// Safely hides the overlay after transcription/processing is done.
/// Only hides if the state is still Transcribing or Processing.
/// If the state has changed to Recording (new session started), it does NOT hide.
pub fn hide_overlay_after_transcription(app_handle: &AppHandle) {
    hide_overlay_after_transcription_inner(app_handle, true);
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

    #[test]
    fn bottom_position_subtracts_overlay_height_and_offset() {
        let work_area = OverlayLogicalWorkArea {
            x: 0.0,
            y: 0.0,
            width: 1200.0,
            height: 800.0,
        };

        let (_, y) = compute_overlay_position_for_work_area(work_area, OverlayPosition::Bottom);
        let expected = work_area.height - OVERLAY_HEIGHT - OVERLAY_BOTTOM_OFFSET;
        assert!((y - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn bottom_position_is_clamped_inside_small_work_area() {
        let work_area = OverlayLogicalWorkArea {
            x: 100.0,
            y: 200.0,
            width: 600.0,
            height: 32.0,
        };

        let (_, y) = compute_overlay_position_for_work_area(work_area, OverlayPosition::Bottom);
        assert!((y - work_area.y).abs() < f64::EPSILON);
    }

    #[test]
    fn center_x_is_clamped_when_work_area_is_narrower_than_overlay() {
        let work_area = OverlayLogicalWorkArea {
            x: 55.0,
            y: 0.0,
            width: 120.0,
            height: 700.0,
        };

        let (x, _) = compute_overlay_position_for_work_area(work_area, OverlayPosition::Top);
        assert!((x - work_area.x).abs() < f64::EPSILON);
    }

    #[test]
    fn work_area_contains_is_half_open_on_bottom_and_right_edges() {
        let work_area = OverlayLogicalWorkArea {
            x: -1590.0,
            y: 892.0,
            width: 1590.0,
            height: 1167.0,
        };

        assert!(is_point_inside_work_area(work_area, -1.0, 1117.0));
        assert!(!is_point_inside_work_area(work_area, 0.0, 1117.0));
        assert!(!is_point_inside_work_area(work_area, -1.0, 2059.0));
    }

    #[test]
    fn distance_to_work_area_is_zero_for_seam_points() {
        let work_area = OverlayLogicalWorkArea {
            x: -1590.0,
            y: 892.0,
            width: 1590.0,
            height: 1167.0,
        };

        let distance_sq = squared_distance_to_work_area(work_area, 0.0, 1117.0);
        assert!((distance_sq - 0.0).abs() < f64::EPSILON);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_bottom_position_uses_visible_frame_min_y_offset() {
        let visible_frame = OverlayLogicalWorkArea {
            x: 1728.0,
            y: -370.0,
            width: 1920.0,
            height: 1055.0,
        };

        let (_, y) =
            compute_overlay_position_for_visible_frame(visible_frame, OverlayPosition::Bottom);
        let expected = visible_frame.y + OVERLAY_BOTTOM_OFFSET;
        assert!((y - expected).abs() < f64::EPSILON);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_top_position_uses_visible_frame_max_y_offset() {
        let visible_frame = OverlayLogicalWorkArea {
            x: 0.0,
            y: 0.0,
            width: 1728.0,
            height: 1079.0,
        };

        let (_, y) = compute_overlay_position_for_visible_frame(visible_frame, OverlayPosition::Top);
        let expected = visible_frame.y + visible_frame.height - OVERLAY_HEIGHT - OVERLAY_TOP_OFFSET;
        assert!((y - expected).abs() < f64::EPSILON);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ax_point_normalization_flips_y_into_appkit_space() {
        let (x, y) = normalize_ax_point_to_appkit(1117.0, 2276.0, 1479.5);
        assert!((x - 2276.0).abs() < f64::EPSILON);
        assert!((y - -362.5).abs() < f64::EPSILON);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ax_rect_normalization_flips_top_origin_rect_into_appkit_space() {
        let raw_rect = OverlayLogicalWorkArea {
            x: 2048.0,
            y: 559.0,
            width: 1280.0,
            height: 800.0,
        };

        let normalized = normalize_ax_rect_to_appkit(1117.0, raw_rect);
        assert!((normalized.x - 2048.0).abs() < f64::EPSILON);
        assert!((normalized.y - -242.0).abs() < f64::EPSILON);
        assert!((normalized.width - 1280.0).abs() < f64::EPSILON);
        assert!((normalized.height - 800.0).abs() < f64::EPSILON);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn zero_sized_ax_rect_is_treated_as_degenerate() {
        let rect = OverlayLogicalWorkArea {
            x: 0.0,
            y: 1117.0,
            width: 0.0,
            height: 0.0,
        };

        assert!(is_degenerate_ax_rect(rect));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn bottom_edge_point_can_be_inside_screen_frame_but_outside_visible_frame() {
        let screen_frame = OverlayLogicalWorkArea {
            x: 1728.0,
            y: -370.0,
            width: 1920.0,
            height: 1117.0,
        };
        let visible_frame = OverlayLogicalWorkArea {
            x: 1728.0,
            y: -308.0,
            width: 1920.0,
            height: 1055.0,
        };
        let point_x = 2688.0;
        let point_y = -330.0;

        assert!(is_point_inside_work_area(screen_frame, point_x, point_y));
        assert!(!is_point_inside_work_area(visible_frame, point_x, point_y));
    }

    #[test]
    fn aborted_recording_hide_only_applies_to_pre_transcription_states() {
        assert!(should_hide_overlay_after_aborted_recording_state(
            OverlayState::Hidden
        ));
        assert!(should_hide_overlay_after_aborted_recording_state(
            OverlayState::Recording
        ));
        assert!(should_hide_overlay_after_aborted_recording_state(
            OverlayState::Connecting
        ));

        assert!(!should_hide_overlay_after_aborted_recording_state(
            OverlayState::Transcribing
        ));
        assert!(!should_hide_overlay_after_aborted_recording_state(
            OverlayState::Processing
        ));
        assert!(!should_hide_overlay_after_aborted_recording_state(
            OverlayState::Cancelling
        ));
        assert!(!should_hide_overlay_after_aborted_recording_state(
            OverlayState::Correcting
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn tolerant_screen_frame_match_accepts_minor_edge_drift() {
        let screen_frame = OverlayLogicalWorkArea {
            x: 1728.0,
            y: -370.0,
            width: 1920.0,
            height: 1117.0,
        };

        assert!(is_point_inside_work_area_with_tolerance(
            screen_frame,
            3649.0,
            -100.0,
            SCREEN_FRAME_EDGE_TOLERANCE,
        ));
        assert!(!is_point_inside_work_area(screen_frame, 3649.0, -100.0));
    }
}
