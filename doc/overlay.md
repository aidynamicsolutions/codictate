# Overlay Architecture

The recording overlay is a high-performance, always-on-top window that visualizes microphone levels and transcription status.

## Core Strategy: "Always Mapped"

To prevent visual flickering associated with OS window mapping/unmapping (`show()`/`hide()`), we use a **transparency + event pass-through** strategy.

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

To ensure the overlay appears **instantly (0ms delay)** when the shortcut is pressed, we implement several critical optimizations:

#### 1. macOS App Nap Prevention
*   **Problem**: macOS suspends hidden webviews ("App Nap"), causing a ~1s wake-up delay.
*   **Solution**: The `NSPanel` is created with `.visible(true)` in the builder. This forces the OS to treat the window as active from birth, even though it is visually transparent.

#### 2. Audio Stream Consistency
*   **Wait for Ready**: `actions.rs` calls `try_start_recording` (blocking ~100ms) *before* showing the overlay.
*   **Result**: This guarantees that if the user sees the "Recording" UI, the microphone stream is definitively active and capturing audio. The latency is minimal thanks to startup pre-warming.
*   **Bluetooth**: For Bluetooth devices, a "Starting microphone..." state is shown during the longer wakeup phase.

#### 3. Frontend Rendering
*   **Non-Blocking**: Heavy operations (like language sync) are fire-and-forget.
*   **Forced Paint**: `requestAnimationFrame` is used to force a browser reflow immediately upon receiving the `show-overlay` event.

### Visual Polish (Content Reveal)

The overlay uses a two-stage animation to feel "instant" yet "premium":

1.  **Instant Background (80ms)**: The black pill background gets `opacity: 1` almost immediately.
2.  **Content Reveal (300ms)**: The internal content (bars, buttons) slides up (`translateY`) with a springy bezier curve `cubic-bezier(0.16, 1, 0.3, 1)`.

This ensures the user has immediate feedback that the system is responsive, while the animation adds character.

## Platform Implementation

*   **macOS**: Uses `NSPanel` via `tauri-nspanel` for "Status" level floating behavior (appears above full-screen apps), configured as a non-activating panel (`no_activate`, `nonactivating_panel`, cannot become key/main) so overlay interaction does not focus the main app window.
*   **Windows**: Uses Win32 `SetWindowPos` to strictly enforce `HWND_TOPMOST`.
*   **Linux**: Standard Tauri webview with `always_on_top`.

## Event Flow

1.  **Hotkey Pressed**: Global shortcut triggers `fn_key_monitor`.
2.  **State Update**: `show_recording_overlay()` is called.
3.  **UI Sync**: `mic-level` events stream volume data to the overlay.
