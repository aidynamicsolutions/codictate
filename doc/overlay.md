# Overlay Architecture

The recording overlay is a high-performance, always-on-top window that visualizes microphone levels and transcription status.

## Core Strategy: "Always Mapped"

To prevent visual flickering associated with OS window mapping/unmapping (`show()`/`hide()`), we use a **transparency + event pass-through** strategy.

On macOS, "always mapped" is now paired with an explicit **present** step on every visible transition. The panel staying mapped keeps the webview warm, but it is **not** treated as proof that the user can currently see it. Native presentation and final placement are handled separately.

### Visibility States

1.  **Hidden (Standby)**
    *   **Window**: Remains `visible` (mapped) but fully transparent.
    *   **Interaction**: `set_ignore_cursor_events(true)` (Click-through enabled).
    *   **Frontend**: Opacity `0` (CSS).

2.  **Visible (Active)**
    *   **Modes**: Recording, Processing, Transcribing, Connecting.
    *   **Window**: Remains `visible`.
    *   **Interaction**: `set_ignore_cursor_events(false)` (Interactive).
    *   **UI**: Opacity `1`. The "Cancel" button remains clickable in all these states to allow aborting the operation.

### Cancellation & State Locking

When the user initiates a cancellation:
1.  The overlay state is immediately set to `Cancelling`.
2.  This acts as a **State Lock**.
3.  Any concurrent background tasks (like "stop recording cleanup") that try to hide the overlay will check this state and **abort** their hide request.
4.  Only the dedicated cancellation cleanup task (which runs after a 600ms delay) has the authority to transition from `Cancelling` to `Hidden`.

This ensures the "Cancelling..." feedback is never prematurely clobbered by race conditions.

### Cancellation Foreground Behavior

Cancel actions are designed to be **non-foregrounding**:
1.  Clicking overlay Cancel must not bring the main app window to front.
2.  This applies across cancel entry points that route through centralized cancellation.
3.  On macOS, a short reopen suppression window (2s) blocks accidental app foregrounding during cancel races, then normal reopen behavior resumes.

### Unified Message Lane (Undo Feedback)

Undo feedback and discoverability hints share the same center message lane.

1.  **Single Presentation Model**
    *   Undo feedback (`Undo applied`, `Nothing to undo`, `Undo expired`) and discoverability use the same lane used by transcribing/processing text.

2.  **Overflow Handling**
    *   Text stays centered/static when it fits.
    *   Marquee scrolling activates only when text overflows available width.
    *   `prefers-reduced-motion` disables marquee animation.

3.  **Operation Priority**
    *   Operational overlay states (`Recording`, `Transcribing`, `Processing`, `Connecting`, `Cancelling`) always preempt undo feedback/discoverability cards.
    *   If an undo hint card is visible and recording/transcription starts, the undo card is dismissed immediately and the operation UI is shown.
    *   Pending undo-card timers are cleared when operation states are shown so they cannot hide the overlay mid-session.

### Hover Hit-Testing Coordinate Space

On Retina displays, hover hit-testing can fail if coordinate spaces are mixed.

1.  `outer_position()` is in **physical pixels**.
2.  Enigo `location()` and `getBoundingClientRect()` are in **logical points**.
3.  Overlay hover regions must normalize to one space (logical points) before `contains()` checks.

### Zero-Latency Optimizations

To ensure the overlay appears **instantly** when any activation shortcut is pressed, we implement several critical optimizations:

#### 1. macOS App Nap Prevention
*   **Problem**: macOS suspends hidden webviews ("App Nap"), causing a ~1s wake-up delay.
*   **Solution**: The `NSPanel` is created with `.visible(true)` in the builder. This forces the OS to treat the window as active from birth, even though it is visually transparent.
*   **Guardrail**: no activation path is allowed to reintroduce blocking readiness waits, remounts, or synchronous post-move verification before the overlay becomes visible.

#### 2. Audio Stream Consistency
*   **Wait for Ready**: `actions.rs` calls `try_start_recording` and waits for capture-ready acknowledgement *before* showing the recording overlay.
*   **Result**: If the user sees the "Recording" UI, the stream is definitively active. Typical latency is ~100-200ms with pre-warming. Start can fail in two windows: stream-open/data-flow startup (up to ~3s `open()` timeout) and capture-ready acknowledgement (500ms timeout after start command).
*   **Connecting Feedback**: A "Starting microphone..." state is shown immediately for known Bluetooth devices, and shown for other devices only when startup exceeds a 220ms threshold. This suppresses unreadable flashes on fast internal-mic starts while preserving explicit feedback for genuinely slow startup.
*   **Model Warm-up**: ASR model loading/decode warm-up continues in the background and no longer blocks the recording bars. Users get the recording UI as soon as the microphone is safe to speak into; the stop/transcription path still waits for model readiness if needed.
*   **Robust Cleanup**: commit-mismatch cleanup is ownership-aware (`owner=self|other`) via per-attempt `prepare_token` tracking, and tray/overlay cleanup is owner-gated so stale starts (including same-binding re-triggers) never stop or visually clobber a newer active recording.
*   **Serialized Prepare Cancellation**: stopping during `Preparing` now transitions through a transient `CancellingPrepare` state before returning to `Idle`, and on-demand stream teardown happens inside that serialized window so new prepare attempts cannot race in before cleanup finishes.
*   **Supersession Semantics**: superseded pending starts are reported as cancellation-class outcomes (`SupersededByNewStart` / `StartAbandonedDueToSupersededState`), not recorder-worker failures.
*   **Silent-Device Safety**: recorder control commands are polled on a short timeout, so `Stop`/`Shutdown` stay responsive even when no audio packets are flowing.

#### 3. Frontend Rendering
*   **Non-Blocking**: Heavy operations (like language sync) are fire-and-forget.
*   **Forced Paint**: `requestAnimationFrame` is used to force a browser reflow immediately upon receiving the `show-overlay` event.
*   **GPU Layer Pre-Promotion**: CSS `will-change` hints on `.recording-overlay-wrapper` (opacity), `.recording-overlay-inner` (opacity, transform), `.countdown-border` (contents), and `.shimmer-border-dash` (stroke-dashoffset) keep GPU compositor layers pre-allocated even during standby. Without these, the first activation after a long idle triggers layer promotion and animation in the same frame, resulting in visible stutter.
*   **Cold-Start Compositor Settle**: The first overlay activation defers the `fade-in` CSS class by two `requestAnimationFrame` callbacks (~33ms at 60fps), giving the compositor a paint cycle to set up layers before animations begin. Subsequent activations apply the class immediately (tracked by `compositedRef`). This delay is imperceptible to humans.

### Visual Polish (Content Reveal)

The overlay uses a two-stage animation to feel "instant" yet "premium":

1.  **Instant Background (80ms)**: The black pill background gets `opacity: 1` almost immediately.
2.  **Content Reveal (300ms)**: The internal content (bars, buttons) slides up (`translateY`) with a springy bezier curve `cubic-bezier(0.16, 1, 0.3, 1)`.

This ensures the user has immediate feedback that the system is responsive, while the animation adds character.

## Platform Implementation

*   **macOS**: Uses `NSPanel` via `tauri-nspanel` at `PopUpMenu` level, configured as a non-activating panel (`no_activate`, `nonactivating_panel`, cannot become key/main) so overlay interaction does not focus the main app window. Every visible overlay state now routes through a unified `present_overlay_panel` path that:
    1. resolves a target point from active text focus
    2. resolves the final `NSScreen.visibleFrame` in AppKit coordinates
    3. applies the frame directly to the `NSPanel`
    4. reapplies panel level / collection behavior
    5. orders the panel front
*   **macOS collection behavior**: prefer `moveToActiveSpace + fullScreenAuxiliary` for a cursor-following HUD. The panel remains always mapped for performance, but AppKit presentation is still performed on every visible transition.
*   **Windows**: Uses Win32 `SetWindowPos` to strictly enforce `HWND_TOPMOST`.
*   **Linux**: Standard Tauri webview with `always_on_top`.
*   **Multi-monitor placement**: Overlay placement is active-text-focus first. The resolution chain is:
    1. Active insertion caret (AX `AXBoundsForRange`, strict 80ms timeout) — normalizes AX geometry into AppKit coordinates, rejects degenerate caret rects
    2. Focused AX window frame (AX `AXPosition` + `AXSize`, strict 80ms timeout)
    3. Cached last-successful target screen center (instant, no I/O)
    4. Mouse cursor position
    5. Onboarding activation target (main window's monitor, only when Learn step is active)
    6. Primary monitor center (final fallback)
    On macOS, display selection is done from `NSScreen.frame` so bottom-edge inputs still target the correct monitor, while final overlay coordinates are clamped into that screen's `NSScreen.visibleFrame`.
*   **macOS top/bottom semantics**: AppKit overlay placement uses `visibleFrame` directly, so `Top` is derived from `visibleFrame.maxY` and `Bottom` from `visibleFrame.minY`. This keeps the Settings choice aligned with where the overlay actually appears on screen.
*   **Deferred verification**: final panel-frame verification is diagnostic and asynchronous. It updates logs and cache state after presentation, but never blocks any activation shortcut.

### Architecture: macOS Presentation Flow

```
present_overlay_panel(source)
│
├─ resolve_overlay_target_point(include_cached=true)
│  ├─ 1. AX caret probe → normalize to AppKit
│  ├─ 2. AX focused window → normalize to AppKit
│  ├─ 3. Cached last-successful screen center
│  ├─ 4. Mouse cursor
│  ├─ 5. Onboarding main window monitor
│  └─ 6. Primary monitor center
│
├─ dispatch_overlay_panel_presentation(target, seq)
│  │  (run_on_main_thread)
│  ├─ resolve_screen_target_for_point(NSScreen)
│  │  ├─ strict frame containment
│  │  ├─ tolerant frame containment (±2pt)
│  │  └─ nearest frame by distance
│  ├─ compute_overlay_position_for_visible_frame
│  ├─ setFrame_display → orderFrontRegardless
│  └─ schedule_overlay_presentation_sample (async diagnostic)
│
└─ [if fallback] schedule_overlay_target_refresh
   └─ 80ms delay → re-resolve → dispatch if changed
```

## Event Flow

1.  **Hotkey Pressed**: Global shortcut triggers `fn_key_monitor`.
2.  **State Update**: `show_recording_overlay()` is called.
3.  **UI Sync**: `mic-level` events stream volume data to the overlay.
