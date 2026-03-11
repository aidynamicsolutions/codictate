# Update Recording Startup Latency via Stream Pause/Resume

## Why
Currently, stopping a recording fully tears down the `cpal` stream and the underlying CoreAudio `AudioUnit`. Rebuilding it on the next user trigger takes 300–800ms. We discovered that keeping the stream alive and using `stream.pause()` (which maps to `AudioOutputUnitStop` on macOS) effectively pauses the hardware graph and turns off the macOS privacy indicator, while dropping the resume latency to ~50ms. 

## What Changes
- Retain the `cpal::Stream` instance across recording sessions instead of dropping it.
- Use `stream.pause()` when a recording stops and `stream.play()` when a new recording starts.
- Update `AudioRecorder`'s consumer thread to gracefully idle or be decoupled from the stream lifetime instead of exiting when the stream drops.

## Impact
- Affected specs: `audio-recording`
- Affected code:
  - `src-tauri/src/audio_toolkit/audio/recorder.rs`
  - `src-tauri/src/managers/audio.rs`
