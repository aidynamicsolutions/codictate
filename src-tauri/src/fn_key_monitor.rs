//! macOS Fn key monitoring using CGEventTap
//!
//! This module provides functionality to detect and use the Fn key for transcription on macOS
//! by using Core Graphics' CGEventTap API to monitor kCGEventFlagsChanged events.
//!
//! The Fn key is a special modifier that macOS intercepts at the system level.
//! By using CGEventTap with FlagsChanged event type, we can detect when the
//! CGEventFlagSecondaryFn flag (0x800000) is set or cleared.
//!
//! Supports two modes:
//! - Push-to-talk (fn alone): Hold Fn to record, release to stop and transcribe
//!   NOTE: Push-to-talk has a 150ms delay before starting to distinguish from fn+space
//! - Hands-free (fn+space): Press combo to toggle recording on/off

use crate::actions::ACTION_MAP;
use crate::managers::audio::AudioRecordingManager;
use crate::shortcut;
use crate::ManagedToggleState;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, CallbackResult,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, error, info};

/// Delay before starting push-to-talk (ms) - allows time to detect fn+space
const PTT_DELAY_MS: u64 = 150;

/// Track whether Fn key monitoring is currently active
static FN_MONITORING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Track the previous Fn key state to detect press/release transitions
static FN_KEY_WAS_PRESSED: AtomicBool = AtomicBool::new(false);

/// Thread-safe storage for the app handle
/// Uses OnceLock for safe initialization (can be reset via Mutex wrapper)
static APP_HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

/// Storage for the event tap thread's run loop (to stop it properly)
static RUN_LOOP: OnceLock<Mutex<Option<CFRunLoop>>> = OnceLock::new();

/// Track if Fn key is being used for transcription (vs just visual feedback)
static FN_TRANSCRIPTION_ENABLED: AtomicBool = AtomicBool::new(false);

/// Track if fn+space was triggered during this Fn key press
/// This prevents push-to-talk from starting when fn+space was used
static FN_SPACE_TRIGGERED: AtomicBool = AtomicBool::new(false);

/// Track if push-to-talk recording was actually started
/// (after the delay expired and Space wasn't pressed)
static PTT_STARTED: AtomicBool = AtomicBool::new(false);

/// Timestamp counter to correlate Fn press events with delayed PTT start
/// This prevents stale timers from triggering if Fn was released and pressed again
static FN_PRESS_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper to get the app handle safely
fn get_app_handle() -> Option<AppHandle> {
    APP_HANDLE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

/// Helper to set the app handle safely
fn set_app_handle(handle: Option<AppHandle>) {
    let mutex = APP_HANDLE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = mutex.lock() {
        *guard = handle;
    }
}

/// Helper to store the run loop for later stopping
fn set_run_loop(run_loop: Option<CFRunLoop>) {
    let mutex = RUN_LOOP.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = mutex.lock() {
        *guard = run_loop;
    }
}

/// Helper to stop and clear the stored run loop
fn stop_stored_run_loop() {
    let mutex = RUN_LOOP.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = mutex.lock() {
        if let Some(ref run_loop) = *guard {
            run_loop.stop();
            debug!("Stopped event tap thread's run loop");
        }
        *guard = None;
    }
}

/// Start Fn key monitoring using CGEventTap
/// This sets up an event tap to detect Fn key presses via kCGEventFlagsChanged events
/// When enable_transcription is true, pressing Fn will trigger the transcribe action
/// If the monitor is already active, this will update the transcription flag without restarting
#[tauri::command]
#[specta::specta]
pub fn start_fn_key_monitor(app: AppHandle, enable_transcription: bool) -> Result<(), String> {
    // If already monitoring, just update the transcription flag
    if FN_MONITORING_ACTIVE.load(Ordering::SeqCst) {
        FN_TRANSCRIPTION_ENABLED.swap(enable_transcription, Ordering::SeqCst);
        return Ok(());
    }

    // Store the app handle and transcription mode safely
    set_app_handle(Some(app.clone()));
    FN_TRANSCRIPTION_ENABLED.store(enable_transcription, Ordering::SeqCst);

    // Spawn the event tap on a separate thread to avoid blocking
    std::thread::spawn(move || {
        // Create the event tap to listen for FlagsChanged and KeyDown events
        // 
        // We need both event types because the Fn/Globe key generates:
        // 1. FlagsChanged events with CGEventFlagSecondaryFn flag
        // 2. KeyDown events with keycode 179 (Globe key) that trigger character picker
        //
        // IMPORTANT: We filter in the callback to ONLY block:
        // - FlagsChanged events with Fn flag set
        // - KeyDown events for Globe key (keycode 179) or Space with Fn held (fn+space)
        // All other keyboard input (arrow keys, typing, etc.) passes through normally.
        //
        // Use HID level to intercept events before they reach other parts of the system
        // Use Default (not ListenOnly) so we can consume events and prevent character picker
        let tap_result = CGEventTap::new(
            CGEventTapLocation::HID, // HID level intercepts events earlier than Session
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default, // Allow consuming events
            vec![
                CGEventType::FlagsChanged,
                CGEventType::KeyDown,  // Needed to block Globe key and detect fn+space
            ],
            |_proxy, event_type, event| {
                let flags = event.get_flags();
                let fn_pressed = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
                
                // Check if this is a KeyDown event
                if matches!(event_type, CGEventType::KeyDown) {
                    // Get the keycode
                    let keycode = event.get_integer_value_field(
                        core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE
                    );
                    
                    // Globe key is keycode 179 - this triggers the character picker
                    // ONLY block this specific key, let ALL other keys pass through
                    const GLOBE_KEY: i64 = 179;
                    if keycode == GLOBE_KEY {
                        debug!("Blocking Globe key (keycode 179) to prevent character picker");
                        return CallbackResult::Drop;
                    }
                    // Space key is keycode 49 - when pressed with Fn, trigger hands-free mode
                    // ONLY if transcription is enabled - otherwise let Space pass through for shortcut recording
                    const SPACE_KEY: i64 = 49;
                    let transcription_enabled = FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst);
                    
                    if keycode == SPACE_KEY && fn_pressed {
                        if transcription_enabled {
                            // Mark that fn+space was triggered - this prevents delayed PTT from starting
                            FN_SPACE_TRIGGERED.store(true, Ordering::SeqCst);
                            
                            // Get app handle safely
                            if let Some(app) = get_app_handle() {
                                let app = &app;
                                // Check if PTT recording has already started (user pressed Space after delay)
                                if PTT_STARTED.swap(false, Ordering::SeqCst) {
                                    debug!("PTT was already started, canceling it before hands-free");
                                    // Cancel the PTT recording
                                    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
                                    audio_manager.cancel_recording();
                                    shortcut::unregister_cancel_shortcut(app);
                                    crate::utils::hide_recording_overlay(app);
                                    crate::tray::change_tray_icon(app, crate::tray::TrayIconState::Idle);
                                }
                                
                                // Now toggle hands-free mode
                                handle_handsfree_toggle(app);
                            }
                            
                            // Block the Space key to prevent it from typing a space
                            return CallbackResult::Drop;
                        } else {
                            // Transcription disabled (e.g., shortcut recording) - pass through Space
                            return CallbackResult::Keep;
                        }
                    }
                    
                    // Pass through ALL other key events (arrow keys, typing, etc.)
                    return CallbackResult::Keep;
                }
                
                // For FlagsChanged events, handle Fn key state
                if matches!(event_type, CGEventType::FlagsChanged) {
                    handle_flags_changed_event(event);
                    
                    // Block Fn-related FlagsChanged events to prevent macOS from seeing them
                    if fn_pressed || FN_KEY_WAS_PRESSED.load(Ordering::SeqCst) {
                        debug!("Blocking Fn FlagsChanged event: fn_pressed={}", fn_pressed);
                        return CallbackResult::Drop;
                    }
                }
                
                // Pass through all other events
                CallbackResult::Keep
            },
        );

        match tap_result {
            Ok(tap) => {
                FN_MONITORING_ACTIVE.store(true, Ordering::SeqCst);
                info!("Fn key monitor started successfully at HID level (transcription: {})", 
                      FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst));

                // Get the run loop source and add it to the current run loop
                let source = tap.mach_port().create_runloop_source(0).unwrap();
                let run_loop = CFRunLoop::get_current();
                run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

                // Store the run loop so stop_fn_key_monitor can stop it
                set_run_loop(Some(run_loop));

                // Enable the tap
                tap.enable();

                // Run the run loop to process events
                // This will block until the run loop is stopped via stop_stored_run_loop()
                CFRunLoop::run_current();
                
                // Clean up after run loop exits
                FN_MONITORING_ACTIVE.store(false, Ordering::SeqCst);
                debug!("Event tap thread exiting normally");
            }
            Err(_) => {
                error!("Failed to create CGEventTap. Ensure Accessibility permissions are granted.");
            }
        }
    });

    Ok(())
}

/// Handle a FlagsChanged event and check for Fn key state
fn handle_flags_changed_event(event: &CGEvent) {
    let flags = event.get_flags();
    let fn_pressed = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
    let was_pressed = FN_KEY_WAS_PRESSED.swap(fn_pressed, Ordering::SeqCst);

    // Only act if state changed
    if fn_pressed != was_pressed {
        // Get app handle safely
        let app_handle = get_app_handle();

        if let Some(app) = app_handle {
            if fn_pressed {
                handle_fn_pressed(&app);
            } else {
                handle_fn_released(&app);
            }
        }
    }
}

/// Handle Fn key press - start delayed push-to-talk timer
fn handle_fn_pressed(app: &AppHandle) {
    debug!("Fn key pressed");
    let _ = app.emit("fn-key-down", ());

    // Reset flags for this new Fn press
    FN_SPACE_TRIGGERED.store(false, Ordering::SeqCst);
    PTT_STARTED.store(false, Ordering::SeqCst);
    
    // Increment the press counter to invalidate any stale timers
    let press_id = FN_PRESS_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;

    // If transcription is enabled, start a delayed timer for push-to-talk
    if FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst) {
        debug!("Starting {}ms delay before push-to-talk (press_id: {})", PTT_DELAY_MS, press_id);
        
        // Clone app handle for the timer thread
        let app_clone = app.clone();
        
        // Spawn a timer thread
        std::thread::spawn(move || {
            // Wait for the delay
            std::thread::sleep(Duration::from_millis(PTT_DELAY_MS));
            
            // Check if this timer is still valid (same press session)
            let current_press_id = FN_PRESS_COUNTER.load(Ordering::SeqCst);
            if current_press_id != press_id {
                debug!("Timer expired but press_id changed ({} != {}), ignoring", press_id, current_press_id);
                return;
            }
            
            // Check if Fn is still pressed
            if !FN_KEY_WAS_PRESSED.load(Ordering::SeqCst) {
                debug!("Timer expired but Fn was released, ignoring");
                return;
            }
            
            // Check if fn+space was triggered during the delay
            if FN_SPACE_TRIGGERED.load(Ordering::SeqCst) {
                debug!("Timer expired but fn+space was triggered, ignoring");
                return;
            }
            
            // All checks passed - start push-to-talk recording
            debug!("Delay expired, starting push-to-talk recording");
            PTT_STARTED.store(true, Ordering::SeqCst);
            
            // Reset hands-free toggle state to ensure mutual exclusivity
            // This prevents stale toggle state if hands-free was previously active
            {
                let toggle_state_manager = app_clone.state::<ManagedToggleState>();
                if let Ok(mut states) = toggle_state_manager.lock() {
                    states.active_toggles.insert("transcribe_handsfree".to_string(), false);
                };
            }
            
            if let Some(action) = ACTION_MAP.get("transcribe") {
                action.start(&app_clone, "transcribe", "fn");
            }
        });
    }
}

/// Handle Fn key release - stop push-to-talk if it was started
fn handle_fn_released(app: &AppHandle) {
    debug!("Fn key released");
    let _ = app.emit("fn-key-up", ());

    // If transcription is enabled and push-to-talk was started, stop it
    if FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst) {
        if PTT_STARTED.swap(false, Ordering::SeqCst) {
            // Push-to-talk was started, stop it now
            debug!("Stopping push-to-talk recording");
            if let Some(action) = ACTION_MAP.get("transcribe") {
                action.stop(app, "transcribe", "fn");
            }
        } else if FN_SPACE_TRIGGERED.load(Ordering::SeqCst) {
            // fn+space was triggered, hands-free is managing its own state
            debug!("fn+space was used, hands-free mode is active");
        } else {
            // Fn was released before delay expired (quick tap)
            debug!("Fn released before delay expired, no action taken");
        }
    }
    
    // Reset the space triggered flag for next press
    FN_SPACE_TRIGGERED.store(false, Ordering::SeqCst);
}

/// Handle fn+space toggle for hands-free mode
fn handle_handsfree_toggle(app: &AppHandle) {
    const BINDING_ID: &str = "transcribe_handsfree";
    
    if let Some(action) = ACTION_MAP.get(BINDING_ID) {
        // Use toggle state to determine whether to start or stop
        let toggle_state_manager = app.state::<ManagedToggleState>();
        let should_start: bool;
        {
            let mut states = toggle_state_manager
                .lock()
                .expect("Failed to lock toggle state manager");

            let is_currently_active = states
                .active_toggles
                .entry(BINDING_ID.to_string())
                .or_insert(false);

            should_start = !*is_currently_active;
            *is_currently_active = should_start;
        } // Lock released here

        // Now call the action without holding the lock
        if should_start {
            debug!("Hands-free mode: starting transcription");
            action.start(app, BINDING_ID, "fn+space");
        } else {
            debug!("Hands-free mode: stopping transcription");
            action.stop(app, BINDING_ID, "fn+space");
        }
    } else {
        error!("No action found for binding: {}", BINDING_ID);
    }
}

/// Stop Fn key monitoring
#[tauri::command]
#[specta::specta]
pub fn stop_fn_key_monitor() -> Result<(), String> {
    if !FN_MONITORING_ACTIVE.load(Ordering::SeqCst) {
        debug!("Fn key monitor not active, nothing to stop");
        return Ok(());
    }

    // Stop the event tap thread's run loop (stored during start_fn_key_monitor)
    stop_stored_run_loop();

    // Reset all state flags
    FN_MONITORING_ACTIVE.store(false, Ordering::SeqCst);
    FN_KEY_WAS_PRESSED.store(false, Ordering::SeqCst);
    FN_TRANSCRIPTION_ENABLED.store(false, Ordering::SeqCst);
    FN_SPACE_TRIGGERED.store(false, Ordering::SeqCst);
    PTT_STARTED.store(false, Ordering::SeqCst);

    // Clear the app handle safely
    set_app_handle(None);

    info!("Fn key monitor stopped");
    Ok(())
}
