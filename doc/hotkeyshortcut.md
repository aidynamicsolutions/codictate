# Hotkey Shortcut System

Concise documentation of Codictate's keyboard shortcut system for speech-to-text.

## Default Shortcuts (macOS)

| Action | Shortcut | Mode | Description |
|--------|----------|------|-------------|
| Push-to-talk | `fn` | Hold | Hold to record, release to transcribe |
| Hands-free | `fn+space` | Toggle | Press to start, press again to stop |
| Cancel | `esc` | Instant | Cancel current recording (auto-registered) |

## How It Works

### Mode Detection (Immediate Start)
 
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
 
 1. **Config Caching**: The `AudioRecorder` caches the `cpal` stream configuration. This bypasses slow device enumeration on subsequent uses, reducing startup time from ~300ms to ~100ms.
 2. **Wait for Ready (UI)**: The overlay is **only shown** after the audio stream is fully active.
    - **Pros**: Guaranteed data integrity. If the user sees "Recording", the mic is definitely capturing audio.
    - **Cons**: Small initial delay (~100-200ms) before UI appears.
    - **Implementation**: `TranscribeAction::start` spawns a thread that calls `try_start_recording` (blocking), and only calls `show_recording_overlay` upon success.

### Bluetooth Microphone Handling

Bluetooth mics (e.g., AirPods) require special handling due to the A2DP→HFP profile switch delay (~1-2s).

**Pre-warming**: On app startup, if a Bluetooth mic is selected, the system briefly opens the audio stream in the background to trigger the profile switch. This happens before the user presses fn.

**Overlay Behavior**:
- **Internal mics**: Skip "Starting microphone..." → go directly to recording overlay (fast ~100-200ms init)
- **Bluetooth mics**: Show "Starting microphone..." connecting overlay → warmup delay → recording overlay

**Warmup Delays** (Bluetooth only):
- First trigger: 1000ms (in case pre-warm didn't complete)
- Subsequent triggers: 750ms (buffer stabilization)

**Key Files**:
- [audio.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/managers/audio.rs): `prewarm_bluetooth_mic()`, `is_current_device_bluetooth()`
- [lib.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/lib.rs): Calls prewarm on app startup
- [actions.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/actions.rs): Warmup delay logic
 
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

### Overlay Readiness Signal (Flicker Prevention)

To prevent overlay flicker on the first Fn press after app startup, the system uses a **readiness handshake**:

**Problem**: The overlay webview loads asynchronously after app startup. If the user presses Fn before React has registered its event listeners, the `show-overlay` event may be emitted too early, causing a race condition where the native window appears but React hasn't set `opacity: 1` yet.

**Solution**:

```
    App Startup                          React Ready                First Fn Press
         │                                    │                          │
         ▼                                    ▼                          ▼
  ┌─────────────┐    ┌───────────────────────────────────┐    ┌─────────────────┐
  │  Create     │───▶│ React registers event listeners   │───▶│ Check OVERLAY_  │
  │  overlay    │    │ then emits "overlay-ready"        │    │ READY flag      │
  │  panel      │    └───────────────────────────────────┘    └─────────────────┘
  └─────────────┘                   │                                  │
                                    ▼                                  ▼
                         ┌───────────────────┐           If ready: proceed immediately
                         │ Rust marks        │           If not ready: wait up to 500ms
                         │ OVERLAY_READY=true│
                         └───────────────────┘
```

**Show Sequence (Flicker-Free)**:

```
   ┌─────────────────────────────────────────────────────────────────────┐
   │ 1. emit("show-overlay", "recording")   ← Send event to React FIRST │
   │ 2. sleep(20ms)                          ← Let React set opacity: 1  │
   │ 3. window.show()                        ← Then make window visible  │
   └─────────────────────────────────────────────────────────────────────┘
```

**Key Files**:
- [overlay.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/overlay.rs): `OVERLAY_READY` atomic, `mark_overlay_ready()`, show sequence ordering
- [RecordingOverlay.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/overlay/RecordingOverlay.tsx): Emits `overlay-ready` after listeners registered

## Implementation

### Backend Files

| File | Purpose |
|------|---------|
| [fn_key_monitor.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/fn_key_monitor.rs) | CGEventTap-based Fn key detection |
| [shortcut.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/shortcut.rs) | Global shortcut registration |
| [actions.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/actions.rs) | `TranscribeAction` start/stop logic |
| [overlay.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/overlay.rs) | Overlay show/hide, readiness tracking |

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
OVERLAY_READY         // True after React has registered event listeners
```

### Mutual Exclusivity

- When PTT starts → hands-free toggle state is reset to `false`
- When fn+space detected with PTT active → PTT is canceled first

## Reserved Shortcuts (Blocked)

### macOS
- `fn+a`, `fn+c`, `fn+d`, `fn+e`, `fn+f`, `fn+h`, `fn+m`, `fn+n`, `fn+q`
- Standard system shortcuts (Cmd+C, Cmd+V, etc.)

### Customization

Users can change shortcuts in Settings → Shortcuts. The app validates:
1. No conflicts with system shortcuts
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
