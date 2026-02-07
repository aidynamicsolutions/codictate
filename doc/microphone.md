# Microphone System Architecture

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/managers/audio.rs` | Core audio manager, device selection |
| `src-tauri/src/commands/audio.rs` | Tauri commands for frontend |
| `src-tauri/src/audio_device_info.rs` | Device type detection (Bluetooth, virtual, built-in) |
| `src/components/shared/MicrophoneModal.tsx` | Device selection UI |

## Selection Flow

```
User clicks device in MicrophoneModal
       ↓
settingsStore calls commands.setSelectedMicrophone()
       ↓
Backend stores in settings, calls rm.update_selected_device()
       ↓
get_effective_device_from_list() determines actual device to use
```

## Device Resolution Logic

`get_effective_device_from_list()` in `audio.rs`:

1. If clamshell mode + clamshell_microphone set → use clamshell mic
2. If selected_microphone is Some(name) → use that device
3. If selected_microphone is None ("Default"):
   - If system default is Bluetooth → fallback to built-in mic
   - Otherwise → use system default

**Note:** When user explicitly selects a Bluetooth device, we store the name (not `None`), so Bluetooth-avoidance is bypassed.

## Clamshell Mode

When a MacBook lid is closed but running with an external display, the built-in mic is muffled.

**Detection:** Uses `ioreg -r -k AppleClamshellState` to query macOS IORegistry.

**How it works:**
1. User sets "Clamshell Microphone" in Settings → Sound
2. On recording start, app checks `is_clamshell()`
3. If true AND clamshell_microphone is set → use that mic

**Files:** `src-tauri/src/helpers/clamshell.rs`, `src/components/settings/ClamshellMicrophoneSelector.tsx`

## Device Filtering

### Continuity Camera

iPhone Continuity Camera mics are filtered from device selection (unreliable for speech-to-text).

**Detection:** CoreAudio transport types `kAudioDeviceTransportTypeContinuityCaptureWired/Wireless`

**Files:** `swift/audio_device_info.swift`, `src/audio_device_info.rs`

### Virtual Devices

Virtual/phantom audio devices are excluded from fallback candidates.

## Device Existence Check

Before opening the recorder, the app checks if the selected device exists in the available device list:

1. If device exists → proceed normally
2. If device is missing (disconnected):
   - Find fallback via `find_fallback_device_from_list()` (prefers built-in, excludes virtual)
   - Update settings to persist fallback
   - Emit `audio-device-auto-switched` event
   - Show notification to user
   - Open with fallback device

This ensures the UI stays in sync when a previously selected mic is disconnected.

## No Automatic Failover During Recording

**There is no automatic failover during recording.** If a microphone stops working mid-recording:
- User will see no audio movement in the audio visualizer
- User should manually switch to a different microphone via settings

This design choice simplifies the codebase and avoids unexpected device switching.

## Frontend-Backend Sync

```
Backend emits "audio-device-auto-switched" event (on initial retry only)
       ↓
Frontend SettingsEventHandler updates settings.selected_microphone
       ↓
UI re-renders + toast notification shown
```

## Debugging

```bash
grep -E "fallback|Switching|audio-device" ~/Library/Logs/com.pais.codictate/*.log | tail -50
```
