# Audio Recording Troubleshooting

## Short Recordings Return Empty

**Issue**: Very short recordings (1-2 seconds) return empty transcription.

**Cause**: Speaking too quickly after pressing record, before the microphone stream is fully initialized (~85ms).

**Solution**: The SmoothedVad implements a pre-roll buffer (15 frames, ~450ms) that continuously captures audio and prepends it to the detected speech. This ensures early speech is not lost.

**Technical Details**:
- VAD Location: `src-tauri/src/managers/audio.rs` → `SmoothedVad::new(..., 15, 15, 2)`
- Pre-roll buffer: 15 frames (~450ms) captures audio before speech onset
- Onset detection: 2 consecutive voice frames required to trigger speech mode
- Hangover: 15 frames (~450ms) continues capturing after speech ends

## Microphone Not Detected

- Check system permissions (macOS: System Preferences → Privacy → Microphone)
- Try a different USB port for external mics
- Restart the application

## Audio Quality Issues

- Ensure microphone is close to sound source
- Check for electrical interference from nearby cables
- Adjust system input volume

## Bluetooth Microphone Quality

**Issue**: When AirPods or other Bluetooth headphones connect, macOS can switch the default input to the Bluetooth mic, which uses the HFP/SCO profile (significantly lower quality than built-in mics).

**How the app handles it**: When "Default" is selected, the app automatically avoids Bluetooth microphones and prefers the built-in mic. No user action is needed.

**If you explicitly want to use a Bluetooth mic**: Select it by name in Settings → Sound → Microphone. The app will use it and apply a warmup delay for Bluetooth profile stabilization.

**If quality is still poor with Bluetooth**:
- Use the built-in MacBook microphone or a USB microphone instead
- Bluetooth mics are limited by the HFP profile (~8kHz bandwidth, "AM radio" quality)
- This is a Bluetooth protocol limitation, not an app issue
