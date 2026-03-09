# Hotkey Shortcut System

Concise documentation of Codictate's keyboard shortcut system for speech-to-text.

## Default Shortcuts (macOS)

| Action | Default Shortcut | Behavior | Description |
|--------|------------------|----------|-------------|
| **Transcribe** | `Option+Space` (or `fn`) | **Push-to-Talk** | Hold to record, release to transcribe. Always instant. |
| **Hands-free** | `fn+space` | **Toggle** | Press to start, press again to stop. |
| Cancel | `esc` | Instant | Cancel current operation (recording or transcription) |

> **Note**: The behavior is tied to the **Action**, not the specific key. Any key mapped to "Transcribe" will function as Push-to-Talk. Any key mapped to "Hands-free" will function as a Toggle.

## How It Works

### Distinct Action Behaviors

The system enforces distinct behaviors for the two main actions to prevent confusion:

1.  **Transcribe Action**: Always **Push-to-Talk**.
    - Records while the key combination is held down.
    - Stops recording immediately upon release.
    - Best for quick commands or short dictations.

2.  **Hands-Free Action**: Always **Toggle**.
    - Starts recording on first press.
    - Stops recording on second press.
    - Ignores key release events.
3.  **Cancellation Logic**:
    -   **During Recording**: Stops the recording immediately and discards audio.
    -   **During Transcription**: The underlying transcription process continues (cannot be aborted safely), but the **result is discarded**. The overlay disappears immediately, and no text is pasted.
    -   **Shortcut**: `Escape` works in all states (Recording, Transcribing, Processing).

### Mode Detection (Fn Key Special Case)

On macOS, the standalone `Fn` key requires special handling via `fn_key_monitor`.

 ```
  User presses Fn
        │
        ▼
  ┌─────────────────┐
  │ Start Recording │  ──▶ PUSH-TO-TALK MODE
  │  Immediately    │      (recording active)
  └─────────────────┘
        │
        ├──── Space pressed while holding Fn ──▶ HANDS-FREE MODE (Cancel PTT, Start Hands-free)
        │
        └──── Released without Space ──────────▶ TRANSCRIBE (Normal PTT end)
 ```

 ### Push-to-Talk Flow (Wait for Ready)

 ```
   Fn Down          Mic Init           Recording           Fn Up
      │                │                  │                  │
      ▼                ▼                  ▼                  ▼
  ┌──────────┐     ┌──────────┐       ┌──────────┐       ┌───────────┐
  │ Start    │ ──▶ │ Block    │ ────▶ │ Overlay  │ ────▶ │Transcribe │
  │ record   │     │ until OK │       │ appears  │       │   & type  │
  └──────────┘     └──────────┘       └──────────┘       └───────────┘
 ```

 ### Audio Latency Optimization

 To eliminate audio cutoff (the "first word missing" problem), the system employs several strategies:

 1. **Config Caching**: The `AudioRecorder` caches the `cpal` stream configuration. This bypasses slow device enumeration on subsequent uses, reducing startup time.
 2. **VAD & Model Warmup**: The system pre-loads the VAD model (`warmup_recorder`) and starts loading the ASR model (`initiate_model_load`) at app startup. This eliminates the ~700ms cold start delay for the first recording.
 3. **Wait for Ready (UI)**: The overlay is **only shown** after the audio stream is fully active.
  1. **Config Caching**: The `AudioRecorder` caches the `cpal` stream configuration. This bypasses slow device enumeration on subsequent uses, reducing startup time.
  2. **VAD & Model Warmup**: The system pre-loads the VAD model (`warmup_recorder`) and starts loading the ASR model (`initiate_model_load`) at app startup. This eliminates the ~700ms cold start delay for the first recording.
  3. **Wait for Ready (UI)**: The overlay is **only shown** after the audio stream is fully active.
    - **Pros**: Guaranteed data integrity. If the user sees "Recording", the mic is definitely capturing audio.
    - **Cons**: Small initial delay (typically ~100-200ms) before UI appears; start fails after capture-ready timeout (500ms) instead of showing a false recording state.
    - **Implementation**: `TranscribeAction::start` waits for `try_start_recording()` to return `RecordingStartOutcome::Started(...)` before showing recording UI. If startup is slow, a "Starting microphone..." connecting overlay appears (immediate for known Bluetooth devices, or after a 120ms threshold for slower non-Bluetooth starts). ASR model warm-up continues in the background and does not block the recording bars.
 5. **AX Fast Fallback (macOS insertion path)**: Smart-insertion AX context capture now applies a short messaging timeout and degrades to context-unavailable behavior when focus lookup is unresponsive (`kAXErrorCannotComplete`) so delivery does not stall behind long AX waits.

### Bluetooth Microphone Handling

Bluetooth mics (e.g., AirPods) require special handling due to the A2DP→HFP profile switch delay (~1-2s).

**Pre-warming**: On app startup, if a Bluetooth mic is selected, the system briefly opens the audio stream in the background to trigger the profile switch. This happens before the user presses fn.

**Overlay Behavior**:
- **Internal mics**: Usually go directly to recording overlay (fast ~100-200ms init); if startup crosses the 120ms threshold, "Starting microphone..." is shown briefly.
- **Bluetooth mics**: Show "Starting microphone..." connecting overlay → warmup delay → recording overlay

**Warmup Delays** (Bluetooth only):
- First trigger: 1000ms (in case pre-warm didn't complete)
- Subsequent triggers: 750ms (buffer stabilization)

**Key Files**:
- [audio.rs](../src-tauri/src/managers/audio.rs): `prewarm_bluetooth_mic()`, `should_show_connecting_overlay_pre_start()`, `is_active_stream_bluetooth()`
- [lib.rs](../src-tauri/src/lib.rs): Calls prewarm on app startup
- [actions.rs](../src-tauri/src/actions.rs): Warmup delay logic

### Seamless Mode Switching

To prevent UI flashing when transitioning from PTT to Hands-Free (Fn held -> Space pressed):
- The `hide_recording_overlay()` call is **skipped** during the transition.
- The overlay remains visible ("Recording") while the backend stops the PTT session and starts the Hands-Free session immediately.
- Visually, the user sees a single continuous recording session.

### Rapid Toggle Prevention (Autorepeat)

To prevent crash/lockup when holding `Fn + Space`:
- The system checks `KEYBOARD_EVENT_AUTOREPEAT` on the Space key.
- Autorepeat events are **ignored** (dropped), ensuring the toggle logic only fires once per physical press.

### Hands-Free Flow

 ```
   fn (down)        space (down)        fn+space (again)
      │                │                     │
      ▼                ▼                     ▼
  ┌──────────┐     ┌──────────┐        ┌───────────┐
  │ Start    │     │ Cancel   │        │  Toggle   │
  │ PTT      │ ──▶ │ PTT &    │  ───▶  │    OFF    │
  │ record   │     │ Start HF │        │(transcribe)│
  └──────────┘     └──────────┘        └───────────┘
                    (seamless)
 ```

### Key Bounce Handling (Debounce)

To prevent accidental recording stops due to key "bounces" (brief release-then-press events, common with mechanical keys or after system sleep), a **150ms debounce** is applied to the Fn key release using a **Generation Counter** approach.

**Logic**:
1. Every `Press` or `Release` event increments a global `RELEASE_GENERATION` counter.
2. On `Release`, a thread waits 150ms.
3. After waiting, check: `if current_generation != my_generation { abort }`
4. This ensures *any* new activity (press or release) immediately invalidates pending stop actions.

**Bounce Preservation**:
If a KeyDown event occurs while a Release debounce is pending (i.e. a bounce), we also check:

```rust
// In handle_fn_pressed:
if PTT_STARTED {
    // Bounce detected! We are already recording.
    // The previous Release thread was killed by generation change.
    // We just return early to keep the current session alive.
    return;
}
```

This prevents the app from entering a "zombie" state where the backend resets flags but the recording continues (or vice versa).

### Race Condition Protection

To prevent "phantom" overlay hiding when rapid actions occur (e.g. starting a new recording while the previous one is finishing transcription), the overlay system uses **State Protection**:

1. **State Tracking**: The overlay tracks its current mode: `Hidden` | `Recording` | `Transcribing` | `Processing`.
2. **Safe Hide**: The `hide_overlay_after_transcription` function checks the current state before hiding.
3. **Logic**:
    - If state is `Transcribing` or `Processing` → Safe to hide (transition to Hidden).
    - If state is `Recording` → **ABORT HIDE**. This means a new recording session has started.

This ensures that the "cleanup" phase of Session 1 does not inadvertently hide the UI for the active Session 2.

### Recording State Machine Protection

The `AudioRecordingManager` uses a state machine with branching transitions:
- Success path: `Idle` → `Preparing` → `Recording` → `Idle`
- Cancellation path: `Preparing` → `CancellingPrepare` → `Idle` (transient cancellation state)

Several race conditions are protected:

**1. Quick Release (Stop Before Ready)**

When user releases key before microphone finishes warming up:
- `stop_recording()` transitions `Preparing` to `CancellingPrepare` first, then tears down on-demand stream/recorder, then finalizes to `Idle`
- `try_start_recording()` checks for `Idle` state and **aborts** with `RecordingStartOutcome::Failed(RecordingStartFailure::StateMismatch(...))`
- In on-demand mode, **self-owned** aborted/failed starts tear down the microphone stream; `owner=other` superseded starts leave the active owner's stream/UI intact.
- This prevents orphaned recordings that act as "hands-free" when they shouldn't

**2. Stale Async Task Rejection (Binding ID Mismatch)**

When PTT is cancelled and hands-free starts before PTT's async task completes:
- PTT starts: `prepare_recording("transcribe")` → async task spawned
- User triggers hands-free: PTT cancelled, `prepare_recording("transcribe_handsfree")` 
- OLD PTT async task calls `try_start_recording("transcribe", ...)`
- State is now `Preparing{binding: "transcribe_handsfree"}` 
- `try_start_recording` checks binding ID: `"transcribe_handsfree" != "transcribe"` → **abort**

```rust
// Key logic in try_start_recording (audio.rs)
match *state {
    RecordingState::Preparing { binding_id: ref active, .. } if active == binding_id => {
        // Good - correct binding
    },
    RecordingState::Preparing { binding_id: ref active, .. } => {
        // ABORT - stale request for different binding
        return RecordingStartOutcome::Failed(
            RecordingStartFailure::StateMismatch(...)
        );
    },
    RecordingState::Idle => {
        // ABORT - stop was called during prepare
        return RecordingStartOutcome::Failed(
            RecordingStartFailure::StateMismatch(...)
        );
    },
    // ...
}
```

**Ownership-safe commit mismatch cleanup**: if capture-ready ack arrives after state ownership changed, cleanup is owner-aware using a per-attempt `prepare_token` (not just binding id).
- `owner=self` (`Idle` during commit): stop local recorder and close on-demand stream.
- `owner=other` (`Preparing/Recording` moved to another binding/session, including same-binding re-trigger with a newer `prepare_token`): do **not** stop the shared recorder; return `StartAbandonedDueToSupersededState` and let the active owner continue.
- `CancellingPrepare` is treated as `owner=self` for commit mismatch classification, so stale start acknowledgements during cancellation cannot survive as active captures.
- Recorder protocol now marks this as a cancellation-class outcome (`SupersededByNewStart`), not a recorder-worker failure.
- If a stale start loses ownership and later observes recorder disconnect/send failure during cancellation teardown, it is classified as `StartAbandonedDueToSupersededState` (cancellation-class) rather than `StartCommandFailed`.
- `owner=other` failures are log-only for UI cleanup: stale starts do **not** hide overlay or set tray icon to idle.
This prevents stale async starts from terminating newer active recordings.

**Control-path responsiveness in silent states**: recorder command handling no longer depends on incoming audio packets.
The consumer loop polls commands on a short timeout, so `Stop`/`Shutdown` complete promptly even if the device has not produced samples yet (for example during timeout/recovery windows).

**FIFO command ordering with safe pre-frame acceleration**: recorder commands are consumed in strict channel order. The consumer applies only the leading contiguous `Start` prefix before processing the current audio packet; once a non-start command (`Stop`/`Shutdown`) is encountered, that command and all subsequent commands are deferred in original order. This preserves first-word capture while preventing `Start` from overtaking older control commands.

**3. FlagsChanged Event Handling (Fn-only)**

`check_ptt_release()` in `fn_key_monitor.rs` handles modifier key release detection:
- **Fn-based shortcuts**: Use FlagsChanged events (fn key isn't a global shortcut)
- **Standard shortcuts** (option+space): Skip this check, use global shortcut's Released event

macOS sends spurious FlagsChanged events with missing modifiers even during normal key holding, causing false release detection. Standard shortcuts are now excluded from this check.

### Overlay Readiness Signal (Flicker Prevention)

To prevent overlay flicker on the first Fn press after app startup, the system uses a **two-phase readiness handshake**:

**Problem**: The overlay webview loads asynchronously after app startup. If `show-overlay` is emitted before the core React listeners are attached, the native window may appear before React applies `opacity: 1`. On remount/reload paths, replay-sensitive listeners (undo/time/correction/hover) can also attach slightly later than show/hide listeners.

**Solution (two-phase)**:

```
      App Startup                         React mount sequence                  First Fn Press
           │                                       │                                 │
           ▼                                       ▼                                 ▼
   ┌─────────────┐                     ┌───────────────────────────┐       ┌─────────────────┐
   │ Create      │                     │ Register show/hide         │       │ Check OVERLAY_  │
   │ overlay     │────────────────────▶│ listeners                  │──────▶│ READY flag      │
   │ panel       │                     │ emit("overlay-ready")      │       └─────────────────┘
   └─────────────┘                     └───────────────────────────┘                 │
                                               │                                      │
                                               ▼                                      ▼
                                   Rust marks visibility-ready             If ready: proceed immediately
                                   + starts model preload                  If not ready: wait up to 500ms
                                               │
                                               ▼
                                   Register replay-sensitive listeners
                                   (recording-time, correction-result,
                                    undo-overlay-event, hover/cursor)
                                               │
                                               ▼
                                   emit("overlay-fully-ready")
                                               │
                                               ▼
                                   Rust replays current overlay state
                                   (mark_overlay_ready + show-overlay replay)
```

**Always Mapped Strategy**:
The overlay window is created at startup and kept `visible` but transparent (`opacity: 0`). This prevents macOS "App Nap" from suspending the webview.

**Model Preload Triggering**:
Startup model preload is now boot-once. `overlay-ready` and the delayed fallback both funnel through a one-shot gate so the app performs at most one startup preload attempt.

**Show Sequence (Flicker-Free)**:

```
   ┌─────────────────────────────────────────────────────────────────────┐
   │ 1. emit("show-overlay", "recording")   ← Send event to React FIRST │
   │ 2. React sets class .fade-in           ← CSS transitions opacity: 1│
   │ 3. Window is already mapped            ← No OS-level delay/flash   │
   └─────────────────────────────────────────────────────────────────────┘
```

**Key Files**:
- [overlay.rs](../src-tauri/src/overlay.rs): `OVERLAY_READY` + `OVERLAY_REPLAY_READY` atomics, `mark_overlay_listener_ready()`, `mark_overlay_ready()`, show/replay delivery gating
- [lib.rs](../src-tauri/src/lib.rs): Listens for `overlay-ready` (visibility-ready) and `overlay-fully-ready` (state replay-ready), then starts model preload via a one-shot startup gate
- [RecordingOverlay.tsx](../src/overlay/RecordingOverlay.tsx): Emits early `overlay-ready`, then `overlay-fully-ready` after replay-sensitive listeners are attached

## Implementation

### Backend Files

| File | Purpose |
|------|---------|
| [fn_key_monitor.rs](../src-tauri/src/fn_key_monitor.rs) | CGEventTap-based Fn key detection |
| [shortcut.rs](../src-tauri/src/shortcut.rs) | Global shortcut registration (Action-based logic enforcement) |
| [actions.rs](../src-tauri/src/actions.rs) | `TranscribeAction` start/stop logic |
| [overlay.rs](../src-tauri/src/overlay.rs) | Overlay show/hide, readiness tracking |

### Shortcut Initialization at Startup

Shortcut initialization is backend-first and idempotent:

- **Backend startup bootstrap**: `initialize_core_logic()` in `lib.rs` calls `initialize_shortcuts_with_source(..., "backend_startup")` immediately.
- **Frontend fallback (normal restart)**: `checkOnboardingStatus()` in `App.tsx` still calls `initializeShortcuts()` when `onboarding_completed` is true.
- **Frontend recovery (permission grant)**: `AccessibilityPermissions.tsx` retries `initializeEnigo()`, `initializeShortcuts()`, and `startFnKeyMonitor(true)` when accessibility permission transitions from denied to granted.

On macOS, startup bootstrap may defer if accessibility permission is missing. Deferred init is expected and retries are automatic via the frontend recovery paths above.

### Binding Migration (Adding New Shortcuts)

When adding a new shortcut binding to `get_default_settings()`, the migration in `load_or_create_app_settings()` automatically adds missing bindings to existing users:

```
App Start → backend shortcut bootstrap → load_or_create_app_settings()
                                       ↓
                  For each default binding not in stored settings:
                  1. Log "Adding missing binding: {id}"
                  2. Insert into settings.bindings
                  3. store.set() + store.save()
```

**Key Files**:
- [settings.rs](../src-tauri/src/settings.rs): Migration logic in `load_or_create_app_settings()` (lines 754-790)
- [lib.rs](../src-tauri/src/lib.rs): Startup bootstrap via `initialize_shortcuts_with_source(..., "backend_startup")`
- [App.tsx](../src/App.tsx): Fallback initialization for onboarding-complete sessions
- [AccessibilityPermissions.tsx](../src/components/AccessibilityPermissions.tsx): Permission-grant recovery retry path

**Debugging Migration Issues**:
1. Check logs for `shortcut_init_attempt` / `shortcut_init_success` / `shortcut_init_deferred` / `shortcut_init_failure`
2. If deferred, confirm accessibility permission status and permission-grant recovery path logs
3. Check `settings_store.json` to confirm bindings are persisted


### Key State Variables

```rust
// Fn Key State (fn_key_monitor.rs)
FN_KEY_WAS_PRESSED    // Tracks if Fn is currently held
FN_SPACE_TRIGGERED    // True if fn+space was used this session
PTT_STARTED           // True if push-to-talk recording started
FN_PRESS_COUNTER      // Invalidates stale timers on rapid presses
RELEASE_GENERATION    // Counts events to invalidate stale release threads
RELEASE_DEBOUNCE_MS   // Debounce duration (150ms)

// Overlay State (overlay.rs)
OVERLAY_STATE         // Current mode: Hidden | Recording | Transcribing | Processing
OVERLAY_READY         // True after visibility listeners are ready (full replay-ready comes later)
OVERLAY_REPLAY_READY  // True only after replay-sensitive listeners are attached
```

### Mutual Exclusivity

- When PTT starts → hands-free toggle state is reset to `false`
- When fn+space detected with PTT active → PTT is canceled first

## Reserved Shortcuts (Blocked)

The app blocks system-critical shortcuts to prevent conflicts. Validation is handled by [reserved.rs](../src-tauri/src/shortcut/reserved.rs).

### macOS

| Category | Blocked Shortcuts |
|----------|------------------|
| **App Control** | `⌘Q`, `⌘H`, `⌘M`, `⌘W`, `⌘,` |
| **Spotlight** | `⌘Space`, `⌥⌘Space` |
| **App Switching** | `⌘Tab`, `⌘\`` |
| **System UI** | `⌥⌘D` (Dock), `⌥⌘Esc` (Force Quit), `⌃⌘Q` (Lock) |
| **Input Source** | `⌃Space`, `⌃⌥Space` |
| **Mission Control** | `⌃↑`, `⌃↓`, `⌃←`, `⌃→`, `⌃1/2/3...` |
| **Accessibility** | `⌥⌘8`, `⌘F5`, `⌥⌘F5` |
| **Screenshots** | `⇧⌘3/4/5/6` |
| **Editing** | `⌘C/V/X/Z/A` |
| **File Ops** | `⌘S/N/O/P` |
| **Fn System** | `fn+a/c/d/e/f/h/m/n/q`, `fn+arrows`, `fn+delete` |

### Windows

| Category | Blocked Shortcuts |
|----------|------------------|
| **System** | `Win+L/D/E/R/Tab`, `Alt+Tab`, `Alt+F4`, `Ctrl+Alt+Del` |
| **Editing** | `Ctrl+C/V/X/Z/Y/A` |
| **File Ops** | `Ctrl+S/N/O/P/W` |

### Linux

| Category | Blocked Shortcuts |
|----------|------------------|
| **System** | `Super+L/D`, `Alt+Tab`, `Alt+F4` |
| **Editing** | `Ctrl+C/V/X/Z/Y/A` |
| **File Ops** | `Ctrl+S/N/O/P` |

### Customization

Users can change shortcuts in Settings → Shortcuts. The app validates:
1. No conflicts with reserved shortcuts (see tables above)
2. No duplicates between push-to-talk and hands-free
3. Requires modifier key (except standalone Fn)

## Reset to Default

Use `resetBindings` (plural) to atomically reset multiple shortcuts at once. This bypasses duplicate checking between the shortcuts being reset.

```typescript
// ❌ Sequential resets can fail on conflicts
await resetBinding("transcribe");      // fails if transcribe_handsfree has "fn"
await resetBinding("transcribe_handsfree");

// ✅ Atomic reset handles any combination
await resetBindings(["transcribe", "transcribe_handsfree"]);
```

## React Best Practices (Lessons Learned)

The `useShortcutRecorder` hook follows these patterns:

1. **Use refs for async callbacks** - React state can be stale in closures. Use `useRef` to track values that async callbacks need to access synchronously.
2. **Avoid nested setState** - Don't call async functions from within `setState` updaters.
3. **Guard duplicate calls** - Use a `saveInProgress` ref to prevent concurrent save operations.
4. **Keep refs in sync with state** - When you need both reactive updates AND synchronous access, update both.

## User Feedback & UX
 
To improve usability, the shortcut recorder implements specific feedback mechanisms:
 
1.  **Delayed Error Feedback (Debounce)**:
    - Validation errors (e.g., "Modifier required") are debounced by **800ms**.
    - This prevents jarring error messages while the user is mid-typing (e.g., pressing "Option" then "Space").
    - If a valid shortcut is formed within the delay window, the error never appears.
 
2.  **Success Indication**:
    - Upon successful save, a **toast notification** ("Shortcut saved") appears.
    - A **green checkmark icon** replaces the pencil icon for 2 seconds to provide immediate visual confirmation.
 
## Shortcut Recording

When recording a new shortcut, transcription triggers are disabled to prevent interference:

```
  User clicks to record
         │
         ▼
  ┌─────────────────────────────────┐
  │ 1. await startFnKeyMonitor(false) │  ← Disable Fn transcription (awaited!)
  │ 2. suspendBinding(shortcutId)     │  ← Suspend global hotkey
  └─────────────────────────────────┘
         │
         ▼
     Recording Mode
     (user presses keys)
         │
         ▼
  ┌─────────────────────────────────┐
  │ 1. startFnKeyMonitor(true)        │  ← Re-enable Fn transcription
  │ 2. resumeBinding(shortcutId)      │  ← Resume global hotkey
  └─────────────────────────────────┘
```

**Key insight**: The `startFnKeyMonitor(false)` call must be **awaited** before entering recording mode. Fire-and-forget allows race conditions where fn+space triggers transcription before disabling completes.

## Thread Safety (Rust)

The Fn key monitor uses `OnceLock<Mutex<Option<T>>>` for thread-safe state:

```rust
// Thread-safe app handle storage
static APP_HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

// Helper functions for safe access
fn get_app_handle() -> Option<AppHandle> { ... }
fn set_app_handle(handle: Option<AppHandle>) { ... }
```

The run loop is also stored to enable proper cleanup when stopping the monitor.

## Stability & Recovery

### Robust Monitor Restart (Anti-Focus Stealing)

The `fn_key_monitor` implements a **self-healing restart loop** to prevent the "Focus Stealing" issue where the main window would pop up if the event tap timed out.

1.  **Silent Recovery**: If the system disables the event tap (e.g., `kCGEventTapDisabledByTimeout` or `TapDisabledByUserInput` during secure password entry), the monitor catches this event.
2.  **No Focus Theft**: Instead of treating this as a permission loss (which warns the user and shows the window), it **logs a warning** and prepares to restart.
3.  **Safety Backoff**:
    -   **Timeout**: Pauses for **1 second** before restarting if the tap times out.
    -   **Creation Failure**: Pauses for **2 seconds** if creating the tap fails (e.g., transient permission glitch).
    -   This prevents "hot-looping" (high CPU) if the system is persistently busy.
4.  **Startup Robustness**: The monitor explicitly handles race conditions during startup to ensure it doesn't silent exit if the thread spawns before the active flag is set.
5.  **Result**: The Fn key might be briefly unresponsive during high load or secure input, but the application remains in the background and respects the user's focus.

### Mixed Shortcut Robustness

This architecture ensures that **standard shortcuts** (e.g., `Option+Space`) are completely isolated from Fn key issues.

-   **Standard Shortcuts**: Managed by the OS global shortcut system. They **never** fail even if the Fn monitor is restarting.
-   **Fn Shortcuts**: Managed by the `fn_key_monitor`. If they timeout, they auto-recover after 1 second.
-   **Mixed Usage**: A user with `Option+Space` (PTT) and `Fn+Space` (Hands-Free) has the most robust setup. If the Fn monitor hits a snag, PTT remains fully functional while Hands-Free briefly recovers.
