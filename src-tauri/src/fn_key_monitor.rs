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
//!   (If fn+space is detected during PTT, the PTT key press is cancelled to switch to hands-free)
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
use tracing::{debug, error, info, warn};
use crate::settings;



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

/// Generation counter for release events
/// Any new event (press or release) increments this, invalidating previous pending actions
static RELEASE_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Debounce time for Fn key release (ms)
const RELEASE_DEBOUNCE_MS: u64 = 150;

/// Flag to signal the permission check thread to stop
static PERMISSION_CHECK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Interval for periodic permission checks (in milliseconds)
/// Using a short interval (500ms) to minimize keyboard lockup when permission is revoked.
/// Note: Checking AXIsProcessTrusted() is very cheap (just reads a flag), so frequent polling is fine.
const PERMISSION_CHECK_INTERVAL_MS: u64 = 500;

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
    debug!("start_fn_key_monitor called (enable_transcription: {})", enable_transcription);
    
    // If already monitoring, just update the transcription flag
    if FN_MONITORING_ACTIVE.load(Ordering::SeqCst) {
        debug!("Monitor already active, just updating transcription flag");
        FN_TRANSCRIPTION_ENABLED.swap(enable_transcription, Ordering::SeqCst);
        return Ok(());
    }

    // Store the app handle and transcription mode safely
    set_app_handle(Some(app.clone()));
    FN_TRANSCRIPTION_ENABLED.store(enable_transcription, Ordering::SeqCst);

    // Check accessibility permission before starting
    debug!("Checking accessibility permission...");
    if !crate::permissions::check_accessibility_permission() {
        warn!("Accessibility permission not granted, cannot start Fn key monitor");
        // Don't show system prompt - let the frontend modal handle permission requests
        // This provides a better UX with our custom modal instead of the macOS system dialog
        return Err("Accessibility permission not granted. Please enable it in System Settings.".to_string());
    }
    debug!("Accessibility permission granted, proceeding to start event tap");

    // Start periodic permission checking
    start_permission_check_thread(app.clone());

    info!("Spawning event tap thread...");
    
    // Mark as active BEFORE spawning so the loop doesn't exit immediately
    FN_MONITORING_ACTIVE.store(true, Ordering::SeqCst);

    // Spawn the event tap on a separate thread to avoid blocking
    std::thread::spawn(move || {
        info!("Fn monitor: Event tap thread started");

        // Loop to allow restarting the event tap if it times out
        loop {
            // Check if we should still be running
            // We do this check at the very start to ensure we don't restart if stopped
            if !FN_MONITORING_ACTIVE.load(Ordering::SeqCst) {
                info!("Fn monitor: Loop detecting stop request (active=false), exiting");
                break;
            }

            info!("Fn monitor: Creating CGEventTap...");
            
            // Flag to signal restart request from callback
            let restart_signal = Arc::new(AtomicBool::new(false));
            let restart_signal_clone = restart_signal.clone();

            // Create the event tap to listen for FlagChanged and KeyDown events
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
                    // NOTE: TapDisabledByTimeout and TapDisabledByUserInput are NOT included here.
                    // They are auto-delivered to the callback when the tap is disabled by the system.
                    // Including them causes integer overflow because their enum values (0xFFFFFFFE, 0xFFFFFFFF)
                    // are too large for the bitmask calculation in core-graphics.
                ],
                move |_proxy, event_type, event| {
                    // Handle tap disabled events FIRST
                    if matches!(event_type, CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput) {
                        warn!("Fn monitor: Event tap disabled by system ({:?}). Requesting restart.", event_type);
                        
                        // Signal restart
                        restart_signal_clone.store(true, Ordering::SeqCst);
                        
                        // Stop the run loop immediately - this releases the current tap
                        // We must ensure this actually stops the loop!
                        stop_stored_run_loop();
                        
                        return CallbackResult::Keep;
                    }
                    
                    let flags = event.get_flags();
                    let fn_pressed = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
                    
                    if matches!(event_type, CGEventType::KeyDown) {
                        let keycode = event.get_integer_value_field(
                            core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE
                        );
                        
                        const GLOBE_KEY: i64 = 179;
                        if keycode == GLOBE_KEY {
                            return CallbackResult::Drop;
                        }

                        const SPACE_KEY: i64 = 49;
                        let should_trigger_handsfree = if let Some(app) = get_app_handle() {
                            let settings = settings::get_settings(&app);
                            if let Some(binding) = settings.bindings.get("transcribe_handsfree") {
                                let b = binding.current_binding.to_lowercase();
                                b == "fn+space" || b == "space+fn"
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        let transcription_enabled = FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst);
                        
                        if keycode == SPACE_KEY && fn_pressed {
                            let is_autorepeat = event.get_integer_value_field(
                                core_graphics::event::EventField::KEYBOARD_EVENT_AUTOREPEAT
                            ) != 0;
                            
                            if is_autorepeat {
                                return CallbackResult::Drop; 
                            }

                            if transcription_enabled && should_trigger_handsfree {
                                FN_SPACE_TRIGGERED.store(true, Ordering::SeqCst);
                                if let Some(app) = get_app_handle() {
                                    let app = &app;
                                    if PTT_STARTED.swap(false, Ordering::SeqCst) {
                                        debug!("PTT was already started, canceling it before hands-free");
                                        let audio_manager = app.state::<Arc<AudioRecordingManager>>();
                                        audio_manager.cancel_recording();
                                        shortcut::unregister_cancel_shortcut(app);
                                    }
                                    handle_handsfree_toggle(app);
                                }
                                return CallbackResult::Drop;
                            } else {
                                return CallbackResult::Keep;
                            }
                        }
                        return CallbackResult::Keep;
                    }
                    
                    if matches!(event_type, CGEventType::FlagsChanged) {
                        if let Some(app) = get_app_handle() {
                            check_ptt_release(&app, event);
                        }

                        handle_flags_changed_event(event);
                        
                        if fn_pressed || FN_KEY_WAS_PRESSED.load(Ordering::SeqCst) {
                            return CallbackResult::Drop;
                        }
                    }
                    
                    CallbackResult::Keep
                },
            );

            match tap_result {
                Ok(tap) => {
                    info!("Fn monitor: Tap created successfully. Entering RunLoop.");
                    // Mark as active (redundant but safe)
                    FN_MONITORING_ACTIVE.store(true, Ordering::SeqCst);

                    let source = tap.mach_port().create_runloop_source(0)
                        .expect("Failed to create CFRunLoop source from mach port");
                    let run_loop = CFRunLoop::get_current();
                    run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

                    set_run_loop(Some(run_loop));

                    tap.enable();
                    CFRunLoop::run_current();
                    
                    // Run loop exited
                    info!("Fn monitor: RunLoop exited.");

                    // Check restart signal
                    if restart_signal.load(Ordering::SeqCst) {
                        warn!("Fn monitor: Restart requested. Sleeping 1s before retry...");
                        
                        // Clear invalid state on restart
                        FN_KEY_WAS_PRESSED.store(false, Ordering::SeqCst);
                        FN_SPACE_TRIGGERED.store(false, Ordering::SeqCst);
                        
                        std::thread::sleep(Duration::from_millis(1000));
                        info!("Fn monitor: Waking up for restart.");
                        continue; // Loop back to top
                    }

                    // No restart requested -> Stopped normally or by stop_fn_key_monitor()
                    // Check if global flag was cleared externally
                    if !FN_MONITORING_ACTIVE.load(Ordering::SeqCst) {
                        info!("Fn monitor: Stopped normally (active flag cleared).");
                    } else {
                        // This shouldn't happen usually unless run loop stopped for unknown reason
                        warn!("Fn monitor: RunLoop stopped but restart not requested and active=true. Treating as error/exit.");
                        FN_MONITORING_ACTIVE.store(false, Ordering::SeqCst);
                    }
                    break;
                }
                Err(e) => {
                    error!("Fn monitor: Failed to create CGEventTap: {:?}", e);
                    // If creation fails, usually permission problem. 
                    // But maybe transient? 
                    // Let's NOT retry infinitely in a tight loop if it's a hard error.
                    // But we can verify permission.
                    
                    // Force check permission
                    if !crate::permissions::check_accessibility_permission() {
                         error!("Fn monitor: Creation failed because permission is missing.");
                         // The permission thread will handle notification.
                         FN_MONITORING_ACTIVE.store(false, Ordering::SeqCst);
                         break;
                    }
                    
                    // If permission is present but creation failed? 
                    error!("Fn monitor: Permission present but Tap creation failed. Retrying in 2s...");
                    std::thread::sleep(Duration::from_millis(2000));
                    continue;
                }
            }
        } // end loop

        info!("Fn monitor: Event tap thread DEAD.");
    });

    Ok(())
}

/// Start a background thread that periodically checks accessibility permission
/// If permission is revoked, it will stop the Fn key monitor and notify the user
fn start_permission_check_thread(app: AppHandle) {
    PERMISSION_CHECK_ACTIVE.store(true, Ordering::SeqCst);
    
    std::thread::spawn(move || {
        debug!("Permission check thread started (interval: {}ms)", PERMISSION_CHECK_INTERVAL_MS);
        
        while PERMISSION_CHECK_ACTIVE.load(Ordering::SeqCst) {
            // Sleep first, then check (we already checked at startup)
            std::thread::sleep(Duration::from_millis(PERMISSION_CHECK_INTERVAL_MS));
            
            // Check if we should still be running
            if !PERMISSION_CHECK_ACTIVE.load(Ordering::SeqCst) {
                break;
            }
            
            // Check permission
            if !crate::permissions::check_accessibility_permission() {
                // Double-check that we should still send notification
                // (TapDisabled handler may have already handled this)
                if !PERMISSION_CHECK_ACTIVE.load(Ordering::SeqCst) {
                    debug!("Permission check thread: TapDisabled handler already notified user, skipping");
                    break;
                }
                
                warn!("Accessibility permission was revoked! Stopping Fn key monitor.");
                
                // Show the main window so the modal is visible
                crate::show_main_window(&app);
                // Emit event to frontend (modal handles UX)
                let _ = app.emit("accessibility-permission-lost", ());
                
                // Stop the Fn key monitor
                let _ = stop_fn_key_monitor();
                break;
            }
        }
        
        debug!("Permission check thread exiting");
    });
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
    info!("handle_fn_pressed: entry, PTT_STARTED={}, FN_SPACE_TRIGGERED={}, FN_KEY_WAS_PRESSED={}", 
          PTT_STARTED.load(Ordering::SeqCst),
          FN_SPACE_TRIGGERED.load(Ordering::SeqCst),
          FN_KEY_WAS_PRESSED.load(Ordering::SeqCst));
    debug!("Fn key pressed");
    let _ = app.emit("fn-key-down", ());

    // Increment generation to invalidate any pending release threads immediately
    RELEASE_GENERATION.fetch_add(1, Ordering::SeqCst);

    // Bounce detection: If PTT is already started, this press is likely a result of 
    // a key bounce (or rapid repress) cancelling the release debounce.
    // We want to continue the existing session, not reset it.
    if PTT_STARTED.load(Ordering::SeqCst) {
        debug!("Bounce detected: PTT already active. Continuing session.");
        return;
    }

    // Reset flags for this new Fn press
    FN_SPACE_TRIGGERED.store(false, Ordering::SeqCst);
    PTT_STARTED.store(false, Ordering::SeqCst);
    
    // Increment the press counter to invalidate any stale timers
    let press_id = FN_PRESS_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;

    // If transcription is enabled, start push-to-talk immediately
    // We no longer wait PTT_DELAY_MS to distinguish from fn+space.
    // Instead, if fn+space is pressed later, the event tap callback will:
    // 1. Detect space key
    // 2. Cancel this PTT recording (discarding the short audio)
    // 3. Start hands-free mode
    
    // Check if we should start PTT (is it assigned to Fn?)
    // This allows Fn+Space to work even if PTT is assigned to something else (like Ctrl+Space)
    let settings = settings::get_settings(app);
    let should_trigger_ptt = if let Some(binding) = settings.bindings.get("transcribe") {
        let b = binding.current_binding.to_lowercase();
        b == "fn" // Exact match only - "fn+space" shouldn't trigger PTT
    } else {
        false
    };

    if FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst) && should_trigger_ptt {
        debug!("Starting push-to-talk recording immediately (press_id: {})", press_id);
        
        // Check microphone permission BEFORE starting recording
        // This must happen before setting PTT_STARTED to prevent
        // the "Transcribing..." overlay from appearing on fn release
        #[cfg(target_os = "macos")]
        {
            use crate::permissions::{check_microphone_permission, MicrophonePermission};
            use tauri::Emitter;
            
            if check_microphone_permission() == MicrophonePermission::Denied {
                warn!("Microphone permission denied, showing permission dialog");
                // Show the main window and emit event for permission dialog
                crate::show_main_window(app);
                let _ = app.emit("microphone-permission-denied", ());
                return; // Don't set PTT_STARTED or start recording
            }
        }
        
        // Check if Hands-Free mode is already active
        // If it is, we should NOT start a PTT session.
        // This allows the user to press Fn + Space to toggle it OFF without interference.
        {
            let toggle_state_manager = app.state::<ManagedToggleState>();
            let is_handsfree = toggle_state_manager.lock()
                .map(|states| states.active_toggles.get("transcribe_handsfree") == Some(&true))
                .unwrap_or(false);
                
            if is_handsfree {
                debug!("Hands-free active, ignoring Fn press (so we don't start PTT on top of it)");
                return;
            }
        }

        // All checks passed - start push-to-talk recording
        info!("handle_fn_pressed: Starting push-to-talk recording (press_id={})", press_id);
        PTT_STARTED.store(true, Ordering::SeqCst);
        
        // Reset hands-free toggle state to ensure mutual exclusivity
        // This prevents stale toggle state if hands-free was previously active
        {
            let toggle_state_manager = app.state::<ManagedToggleState>();
            if let Ok(mut states) = toggle_state_manager.lock() {
                states.active_toggles.insert("transcribe_handsfree".to_string(), false);
            };
        }
        
        if let Some(action) = ACTION_MAP.get("transcribe") {
            action.start(app, "transcribe", "fn");
        }
    }
}

/// Handle Fn key release - stop push-to-talk if it was started
fn handle_fn_released(app: &AppHandle) {
    info!("handle_fn_released: entry, PTT_STARTED={}, FN_SPACE_TRIGGERED={}", 
          PTT_STARTED.load(Ordering::SeqCst),
          FN_SPACE_TRIGGERED.load(Ordering::SeqCst));
    debug!("Fn key released");
    let _ = app.emit("fn-key-up", ());

    // If transcription is enabled...
    if FN_TRANSCRIPTION_ENABLED.load(Ordering::SeqCst) {
        // If push-to-talk was started, we need to stop it
        // Check PTT_STARTED separately to handle debouncing
        if PTT_STARTED.load(Ordering::SeqCst) {
            // Increment generation to track this specific release event
            // This invalidates any previous debounce threads
            let gen = RELEASE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
            
            let app_clone = app.clone();
            std::thread::spawn(move || {
                // Wait to see if this is a real release or a bounce
                std::thread::sleep(Duration::from_millis(RELEASE_DEBOUNCE_MS));
                
                // Check if generation changed (meaning a new press or release happened)
                let current_gen = RELEASE_GENERATION.load(Ordering::SeqCst);
                if current_gen != gen {
                    debug!("Release debounce cancelled by new activity (gen {} -> {}), likely bounce", gen, current_gen);
                    return;
                }
                
                // Double check key state (should be released)
                if FN_KEY_WAS_PRESSED.load(Ordering::SeqCst) {
                    debug!("Release debounce cancelled: key is pressed again");
                    return;
                }
                
                // Confirmed release
                // Now actually stop PTT
                // Atomically swap PTT_STARTED to false to ensure we only stop once
                if PTT_STARTED.swap(false, Ordering::SeqCst) {
                    debug!("Stopping push-to-talk recording (after debounce)");
                    if let Some(action) = ACTION_MAP.get("transcribe") {
                        action.stop(&app_clone, "transcribe", "fn");
                    }
                }
            });
            return;
        } 
        
        // Handle other cases (fn+space, or release before PTT start)
        if FN_SPACE_TRIGGERED.load(Ordering::SeqCst) {
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
    
    // Check microphone permission BEFORE modifying toggle state
    // This prevents the toggle from getting into an inconsistent state
    #[cfg(target_os = "macos")]
    {
        use crate::permissions::{check_microphone_permission, MicrophonePermission};
        use tauri::Emitter;
        
        if check_microphone_permission() == MicrophonePermission::Denied {
            warn!("Microphone permission denied, showing permission dialog");
            crate::show_main_window(app);
            let _ = app.emit("microphone-permission-denied", ());
            return;
        }
    }
    
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

    // Stop the permission check thread
    PERMISSION_CHECK_ACTIVE.store(false, Ordering::SeqCst);

    // Reset all state flags
    FN_MONITORING_ACTIVE.store(false, Ordering::SeqCst);
    FN_KEY_WAS_PRESSED.store(false, Ordering::SeqCst);
    FN_TRANSCRIPTION_ENABLED.store(false, Ordering::SeqCst);
    FN_SPACE_TRIGGERED.store(false, Ordering::SeqCst);
    PTT_STARTED.store(false, Ordering::SeqCst);
    
    // Reset counters to clean state (improves debuggability)
    FN_PRESS_COUNTER.store(0, Ordering::SeqCst);
    RELEASE_GENERATION.store(0, Ordering::SeqCst);

    // Clear the app handle safely
    set_app_handle(None);

    info!("Fn key monitor stopped");
    Ok(())
}

/// Check if the standard PTT shortcut keys were released
/// This handles the case where users release keys in an order that standard plugins miss
/// (e.g. releasing Modifier before Key)
///
/// NOTE: This is ONLY used for fn-based shortcuts. Standard shortcuts like option+space
/// use the global shortcut's Released event which works correctly. We skip standard
/// shortcuts here because macOS sends spurious FlagsChanged events during normal key holding.
fn check_ptt_release(app: &AppHandle, event: &CGEvent) {
    // If Fn PTT is active, ignore (it has its own release logic in fn_key_callback)
    if PTT_STARTED.load(Ordering::SeqCst) {
        return;
    }

    // Check if "transcribe" is the active recording
    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    match audio_manager.get_active_binding_id() {
         Some(id) if id == "transcribe" => {
             // Continue to check release
         },
         _ => return // Not recording or different binding
    }

    // Check settings for validity
    let settings = settings::get_settings(app);
    let binding = match settings.bindings.get("transcribe") {
        Some(b) => b,
        None => return
    };

    // ONLY handle fn-based shortcuts here. Standard shortcuts (option+space, ctrl+space, etc.)
    // use the global shortcut's Released event which works correctly.
    // We skip standard shortcuts because macOS sends spurious FlagsChanged events
    // with missing modifiers even while keys are held, causing false release detection.
    let binding_lower = binding.current_binding.to_lowercase();
    if !binding_lower.starts_with("fn") && !binding_lower.contains("+fn") {
        // Standard shortcut - skip, let global shortcut handler deal with release
        return;
    }

    let required_flags = parse_shortcut_modifiers(&binding.current_binding);
    
    // If no modifiers required (e.g. just "fn"), skip this check
    if required_flags.is_empty() {
        return;
    }

    let current_flags = event.get_flags();
    
    // If we lost any required modifier, stop recording
    if !current_flags.contains(required_flags) {
        debug!("Fn-based PTT release detected via FlagsChanged (modifiers missing). Current: {:?}, Required: {:?}", current_flags, required_flags);
        
        if let Some(action) = ACTION_MAP.get("transcribe") {
            action.stop(app, "transcribe", "monitor_release");
        }
    }
}

/// Helper to parse modifiers from a shortcut string
fn parse_shortcut_modifiers(shortcut: &str) -> CGEventFlags {
    let mut flags = CGEventFlags::empty();
    let lower = shortcut.to_lowercase();
    for part in lower.split('+') {
        match part.trim() {
             "cmd" | "command" | "super" => flags.insert(CGEventFlags::CGEventFlagCommand),
             "shift" => flags.insert(CGEventFlags::CGEventFlagShift),
             "alt" | "option" => flags.insert(CGEventFlags::CGEventFlagAlternate),
             "ctrl" | "control" => flags.insert(CGEventFlags::CGEventFlagControl),
             _ => {}
        }
    }
    flags
}
