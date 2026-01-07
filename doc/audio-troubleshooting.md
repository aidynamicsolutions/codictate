# Audio Recording Troubleshooting

## Empty First Transcription

**Issue**: First transcription after app startup returns empty, but subsequent ones work.

**Cause**: Microphone initialization can produce unstable audio frames that VAD (Voice Activity Detection) misclassifies as noise.

**Solution**: The audio recorder discards the first 3 frames (~90ms) after recording starts to allow the microphone and VAD to stabilize. This follows industry best practice for handling initialization noise.

**Technical Details**:
- Location: `src-tauri/src/audio_toolkit/audio/recorder.rs`
- Warmup frames: 3 (configurable via `WARMUP_FRAMES` constant)
- Duration: ~90ms at 30ms/frame

## Microphone Not Detected

- Check system permissions (macOS: System Preferences → Privacy → Microphone)
- Try a different USB port for external mics
- Restart the application

## Audio Quality Issues

- Ensure microphone is close to sound source
- Check for electrical interference from nearby cables
- Adjust system input volume
