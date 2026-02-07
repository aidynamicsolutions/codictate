# Microphone System Architecture

This document explains the microphone detection, selection, and failover logic in Handy.

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/managers/audio.rs` | Core audio manager, failover logic |
| `src-tauri/src/commands/audio.rs` | Tauri commands for frontend communication |
| `src-tauri/src/audio_device_info.rs` | Device type detection (Bluetooth, virtual, built-in) |
| `src/components/shared/MicrophoneModal.tsx` | Device selection UI |
| `src/stores/settingsStore.ts` | Frontend settings + updaters |

## Selection Flow

```
User clicks device in MicrophoneModal
       ↓
updateSetting("selected_microphone", device.name)  ← Always saves explicit name
       ↓
settingsStore calls commands.setSelectedMicrophone()
       ↓
Backend stores in settings, calls rm.update_selected_device()
       ↓
get_effective_device_from_list() determines actual device to use
```

## Device Resolution Logic

`get_effective_device_from_list()` in `audio.rs` handles device selection:

```
1. If clamshell mode + clamshell_microphone set → use clamshell mic
2. If selected_microphone is Some(name):
   a. If name is in blocklist → find fallback
   b. Otherwise → return that device (respects user intent)
3. If selected_microphone is None ("Default"):
   a. If system default is Bluetooth → fallback to built-in mic
   b. Otherwise → use system default
```

**Critical:** When user explicitly selects a Bluetooth device, we store the name (not `None`), so step 2b runs and Bluetooth-avoidance is bypassed.

## Clamshell Mode

**What is it?** When a MacBook lid is closed but the machine is running with an external display.

**Detection:** Uses `ioreg -r -k AppleClamshellState` to query macOS IORegistry.

**Purpose:** The built-in microphone is muffled when the lid is closed. Users typically have an external microphone (USB, webcam) that they prefer to use.

**How it works:**
1. User sets a "Clamshell Microphone" in Settings → General
2. On each recording start, app checks `is_clamshell()`
3. If true AND clamshell_microphone is set → use that mic instead of regular selection

**Key files:**
- `src-tauri/src/helpers/clamshell.rs` — Detection logic
- `src/components/settings/ClamshellMicrophoneSelector.tsx` — UI (only shown on laptops)

## Dead Device Detection

Monitors for "dead" microphones (connected but producing no audio):

```
Audio callback runs every ~10ms
       ↓
If all samples are zero → increment zero_level_count
       ↓
After 50 consecutive zero readings (~500ms) → emit "audio-device-dead" event
       ↓
Dead device listener triggers failover
```

**Key state:** `zero_level_count: Arc<Mutex<u32>>`

## Failover Logic

When a device is detected as dead:

1. Add failed device to `blocked_devices` blocklist
2. Find fallback via `find_fallback_device_from_list()`:
   - Prefers built-in mic
   - Excludes blocked devices
   - Excludes virtual devices
3. Save fallback to settings + emit `audio-device-auto-switched` event
4. Stop current stream, restart with new device

## Grace Period (Prevents Cascade)

**Problem solved:** After failover, the new device might produce 0 samples briefly during transition, causing a secondary false failover.

**Solution:** 10-second grace period using timestamp:

```rust
failover_timestamp: Arc<Mutex<Option<Instant>>>
```

- Set to `Some(Instant::now())` when failover occurs
- 0-sample check in `stop_recording()` skips blocklist addition if within grace period
- Never reset on good audio (unlike the old boolean flag)

## Blocklist Management

`blocked_devices: Arc<Mutex<HashSet<String>>>`

- **Added:** When dead device detected (during failover)
- **Cleared:** When user explicitly selects a device (fresh start)
- **Checked:** In `get_effective_device_from_list()` to skip dead devices

## Frontend-Backend Sync

Device auto-switch flow:

```
Backend emits "audio-device-auto-switched" event
       ↓
Frontend SettingsEventHandler receives it
       ↓
Updates settings.selected_microphone store
       ↓
UI re-renders with new selection
       ↓
Toast notification shown to user
```

## Important Invariants

1. **Explicit selection always wins:** If `selected_microphone` is `Some(name)`, use that device (unless blocked)
2. **Bluetooth-avoidance only for Default:** Only applies when `selected_microphone` is `None`
3. **Grace period protects transitions:** 10 seconds after failover, don't add new device to blocklist
4. **User selection clears blocklist:** Fresh start on explicit device selection

## Debugging

Check logs for:
```bash
grep -E "failover|blocklist|Dead device|Switching|grace period" ~/Library/Logs/com.pais.codictate/*.log | tail -50
```

Key log patterns:
- `Dead device detected: 50 consecutive zero-level readings` → Device failed
- `Skipping blocklist addition - within 10s grace period` → Protection working
- `Immediate failover: Switching from X to Y` → Failover in progress
- `Added 'X' to dead device blocklist` → Device blocked
