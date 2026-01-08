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
