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

*   **macOS**: Uses `NSPanel` via `tauri-nspanel` for "Status" level floating behavior (appears above full-screen apps).
*   **Windows**: Uses Win32 `SetWindowPos` to strictly enforce `HWND_TOPMOST`.
*   **Linux**: Standard Tauri webview with `always_on_top`.

## Event Flow

1.  **Hotkey Pressed**: Global shortcut triggers `fn_key_monitor`.
2.  **State Update**: `show_recording_overlay()` is called.
3.  **UI Sync**: `mic-level` events stream volume data to the overlay.
