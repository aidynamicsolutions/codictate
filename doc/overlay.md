# Overlay Architecture

The recording overlay is a high-performance, always-on-top window that visualizes microphone levels and transcription status.

## core Strategy: "Always Mapped"

To prevent visual flickering associated with OS window mapping/unmapping (`show()`/`hide()`), we use a **transparency + event pass-through** strategy.

### Visibility States

1.  **Hidden (Standby)**
    *   **Window**: Remains `visible` (mapped) but fully transparent.
    *   **Interaction**: `set_ignore_cursor_events(true)` (Click-through enabled).
    *   **Frontend**: Opacity `0`.

2.  **Visible (Active)**
    *   **Modes**: Recording, Processing, Transcribing.
    *   **Window**: Remains `visible`.
    *   **Interaction**: `set_ignore_cursor_events(false)` (Interactive).
    *   **Frontend**: Opacity `1` (via CSS transition).

### State Transitions

*   **Show**:
    1.  Backend: `set_ignore_cursor_events(false)`.
    2.  Backend: Emits `show-overlay`.
    3.  Frontend: Sets React state to visible (fade-in).
*   **Hide**:
    1.  Backend: Emits `hide-overlay`.
    2.  Frontend: Sets React state to hidden (fade-out).
    3.  Backend: `set_ignore_cursor_events(true)` (after short delay or immediately, relying on frontend transparency).

*Note: `window.show()` is only called on the very first activation or if the window was forcefully hidden.*

## Platform Implementation

*   **macOS**: Uses `NSPanel` via `tauri-nspanel` for "Status" level floating behavior (appears above full-screen apps).
*   **Windows**: Uses Win32 `SetWindowPos` to strictly enforce `HWND_TOPMOST`.
*   **Linux**: Standard Tauri webview with `always_on_top`.

## Event Flow

1.  **Hotkey Pressed**: Global shortcut triggers `fn_key_monitor`.
2.  **State Update**: `show_recording_overlay()` is called.
3.  **UI Sync**: `mic-level` events stream volume data to the overlay.
