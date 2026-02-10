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
User picks "Default" or a specific device in MicrophoneModal
       ↓
Frontend saves "default" (for Default) or explicit device name
       ↓
Backend stores in settings, calls rm.update_selected_device()
       ↓
get_effective_device_from_list() determines actual device to use
```

## Device Resolution Logic

`get_effective_device_from_list()` in `audio.rs`:

1. If clamshell mode + clamshell_microphone set → use clamshell mic
2. If selected_microphone is `None`, `"default"`, or `"Default"` → treated as "Default" mode:
   - If system default is Bluetooth → multi-tier fallback (see below)
   - Otherwise → use system default
3. If selected_microphone is any other `Some(name)` → use that device strictly

**Note:** When user explicitly selects a Bluetooth device, we store the device name, so Bluetooth-avoidance is bypassed.

## Bluetooth Avoidance

When "Default" is selected and the system default mic is Bluetooth (e.g., AirPods), the app avoids it to prevent low-quality audio (BT uses HFP/SCO profile for mic input). The fallback priority is:

1. **Built-in microphone** (e.g., MacBook Pro Microphone)
2. **Any non-Bluetooth, non-virtual device** (e.g., USB mic)
3. **Bluetooth default** (last resort, only if no alternatives exist)

This also applies in `start_microphone_stream()` as a safety net: if `get_effective_device_from_list()` returns `None`, the stream-open code resolves a device using the same priority instead of passing `None` to `cpal` (which would use the system default and could be Bluetooth).

**Bluetooth Pre-warm:** `prewarm_bluetooth_mic()` only triggers the A2DP→HFP profile switch at startup if the user has **explicitly** selected a Bluetooth device. When "Default" is selected, no pre-warm occurs since the app will use a built-in mic instead.

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

## Microphone Modal UI

The modal (`MicrophoneModal.tsx`) presents two types of options:

1. **"Default" option** — always shown at the top. Saves `"default"` to settings, which triggers backend BT-avoidance logic. Subtitle explains "Automatically selects the best available microphone".
2. **Individual devices** — listed below Default. Bluetooth devices are sorted to the bottom and display an amber "Bluetooth" badge with a tooltip warning about reduced quality.

**Display label** in settings row (`MicrophoneSelector.tsx`): When Default is selected, shows `"Default (MacBook Pro Microphone)"` (resolved via shared `resolveDefaultMicName()` utility). This is display-only — the backend makes the authoritative device choice.

**Shared utilities** (`src/utils/microphoneUtils.ts`): `isDefaultMicSetting()` and `resolveDefaultMicName()` consolidate the default-detection and display-name-resolution logic used by both components.

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
