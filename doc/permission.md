# Permission Architecture

This document describes macOS permission handling in Codictate.

## Permissions Required

| Permission | Purpose | Check API |
|------------|---------|-----------| 
| **Accessibility** | Fn/Globe key capture, paste via Cmd+V, direct text input | `AXIsProcessTrusted()` |
| **Microphone** | Audio recording | `AVCaptureDevice.authorizationStatus` |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Action                              │
│                    (press Fn to record)                         │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│               fn_key_monitor.rs (Event Tap)                     │
│  • CGEventTap at HID level captures Fn key                      │
│  • Requires Accessibility permission                            │
│  • TapDisabledByTimeout → permission revoked                    │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│           actions.rs → TranscribeAction::start()                │
│  • Calls check_microphone_permission() BEFORE recording         │
│  • Denied → emit event + notification, abort start              │
│  • Authorized → calls audio.rs::try_start_recording()           │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                   permissions.rs                                │
│  • check_accessibility_permission() → AXIsProcessTrusted        │
│  • check_microphone_permission() → AVCaptureDevice via objc2    │
│  • open_accessibility_settings() → deep-link                    │
│  • open_microphone_settings() → deep-link                       │
└─────────────────────────────────────────────────────────────────┘
```

## Files

| File | Responsibility |
|------|----------------|
| `permissions.rs` | Low-level permission check APIs + open settings commands |
| `fn_key_monitor.rs` | Accessibility permission handling + TapDisabled recovery |
| `actions.rs` | Microphone permission gate before recording starts |
| `audio.rs` | Recording state machine, stream lifecycle, and capture-ready start coordination |
| `tray.rs` | Tray icon state management (Idle, Recording, Transcribing) |
| `input.rs` | Enigo initialization (lazy if no accessibility permission) |
| `PermissionBanner.tsx` | Generic component: modal + banner + focus re-check + event listener |
| `AccessibilityPermissions.tsx` | Wrapper using PermissionBanner, retries Enigo + shortcut init + Fn monitor on grant |
| `MicrophonePermissions.tsx` | Wrapper using PermissionBanner, waits for accessibility first |
| `PermissionModal.tsx` | Unified modal dialog for permission requests |

## Startup Flow (No Permissions)

```
App Launch
    │
    ├─► input.rs: EnigoState::new()
    │   └─► check_accessibility_permission()
    │       ├─► GRANTED: Initialize Enigo immediately
    │       └─► NOT GRANTED: Skip (no system dialog triggered)
    │
    ├─► shortcut bootstrap: initialize_shortcuts_with_source("backend_startup")
    │   └─► check_accessibility_permission()
    │       ├─► GRANTED: Register global shortcuts immediately
    │       └─► NOT GRANTED: Defer init (retry later via frontend recovery)
    │
    ├─► fn_key_monitor.rs: start_fn_key_monitor()
    │   └─► check_accessibility_permission()
    │       └─► NOT GRANTED: Return error (no system dialog)
    │
    └─► Frontend: AccessibilityPermissions.tsx
        └─► checkAccessibilityPermission()
            └─► NOT GRANTED: Show custom modal (not system dialog)
```

## Revocation Handling

### Accessibility (Fn key stops working)

1. **Primary**: 500ms polling of `AXIsProcessTrusted()` (very cheap call)
2. **Backup**: `TapDisabledByTimeout` event (can be delayed 5-10s by macOS)
3. On detection:
   - Reset flags, stop run loop
   - Emit `accessibility-permission-lost` event
   - Show a throttled native notification (no forced focus steal)
   - `PermissionBanner` shows modal with "Open Settings" button

### Microphone (recording fails)

1. Canonical start gate is in `actions.rs` (`TranscribeAction::start`) before recording/overlay transitions.
2. Fn-specific fast path in `fn_key_monitor.rs` also checks before toggling PTT/hands-free state.
3. Both paths use `AVCaptureDevice.authorizationStatus(for: .audio)` via objc2.
4. Denied → emit permission event + show throttled native notification (no forced focus steal).

## Permission Revocation Flow

```
 ┌──────────────────────┐     ┌──────────────────────┐
 │   macOS Revokes      │     │   User Presses Fn    │
 │   Accessibility      │     │   (Mic Denied)       │
 └──────────┬───────────┘     └──────────┬───────────┘
            │                            │
            ▼                            ▼
 ┌──────────────────────┐     ┌──────────────────────┐
 │ fn_key_monitor.rs    │     │ fn_key_monitor.rs    │
 │ (500ms poll detects) │     │ (check_microphone_   │
 │                      │     │  permission())       │
 └──────────┬───────────┘     └──────────┬───────────┘
            │                            │
            ├────────────────────────────┤
            ▼                            ▼
 ┌────────────────────────────────────────────────┐
 │  Emit Event + Native Notification              │
 │  "accessibility-permission-lost" /             │
 │  "microphone-permission-denied"                │
 └───────────────────────┬────────────────────────┘
                         │
         ┌───────────────┴────────────────┐
         ▼                                ▼
 ┌──────────────┐                  ┌──────────────┐
 │ User clicks  │                  │ Main window  │
 │ notification │                  │ already open │
 └──────┬───────┘                  └──────┬───────┘
        │                                  │
        ▼                                  ▼
 ┌──────────────┐                  ┌──────────────────────────┐
 │ macOS Reopen │                  │ PermissionBanner modal/  │
 │ event shows  │                  │ banner guides user       │
 │ main window  │                  └─────────────┬────────────┘
 └──────┬───────┘                                │
        │                                         ▼
        ▼                                ┌──────────────┐
 ┌──────────────┐                        │ Deep-link to │
 │ Permission   │                        │ System Prefs │
 │ UI appears   │                        └──────────────┘
 └──────────────┘
```

## Key Learnings

- **No system notification API**: Apple provides no way to observe permission revocation
- **macOS delays TapDisabled**: Tap is disabled immediately, but event sent seconds later
- **500ms polling chosen**: Prioritizes minimal keyboard lockup over minimal CPU usage
- **cpal can't detect mic denial**: macOS opens devices with silenced audio
- **objc2 for FFI**: Raw `objc_msgSend` crashes on ARM64; use `msg_send!` macro
- **"soun" media type**: AVMediaTypeAudio = `NSString::from_str("soun")`
- **Callback must be fast**: Heavy work in TapDisabled callback causes keyboard lockup
- **Accessibility for Enigo**: Paste simulation (Cmd+V) requires accessibility permission
- **Avoid system dialog**: Don't call `prompt_accessibility_permission()` or `Enigo::new()` without checking permission first
- **Graceful error handling**: Use `match` not `expect()` in tray icon code to avoid panics
- **Focus-based re-check**: Use window focus event to detect permission grant (user returns from Settings)
- **Permission-grant recovery path**: On accessibility grant, retry `initializeEnigo()`, `initializeShortcuts()`, and `startFnKeyMonitor(true)`
- **DRY with PermissionBanner**: Shared component handles modal, banner, event, and focus for both permission types
- **Accessibility-first priority**: Microphone UI only renders when accessibility is already granted
- **No hotkey focus theft**: Permission failures from transcription hotkeys notify the user without bringing the app window to front
- **Reopen-driven recovery**: On macOS, app reopen events (dock/notification click) are used to surface the main UI when hidden
