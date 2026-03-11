## 1. Stream Lifecycle Changes
- [x] 1.1 Update `AudioRecorder` to hold the `cpal::Stream` persistently instead of dropping it on stop.
- [x] 1.2 Implement a paused state in `AudioRecorder` using `stream.pause()`.
- [x] 1.3 Update `start_microphone_stream` (or equivalent) in `AudioRecorder` to optionally just call `stream.play()` if a stream is already built for the correct device.

## 2. Audio Manager Updates 
- [x] 2.1 Update `AudioRecordingManager::stop_microphone_stream` to trigger a pause instead of a full drop.
- [x] 2.2 Verify `kickoff_on_demand_prearm` and start logic correctly leverage the paused stream for low-latency resumes.

## 3. Worker/Consumer Thread Updates
- [x] 3.1 Update the `run_consumer` thread logic to avoid spinning or crashing when the producer stream is paused.
- [x] 3.2 Ensure the consumer resumes processing immediately when the stream is unpaused.

## 4. Documentation and Logging
- [x] 4.1 Update system architecture documentation (e.g., `doc/microphone-startup-optimization.md`) to document the pause/resume lifecycle and its expected latency.
- [x] 4.2 Review and update any relevant frontend or backend logs to ensure the latency and core phases of `stream_open_subphase` correctly reflect the new "resume" operations vs full "rebuilds".

## 5. Verification and Validation
- [x] 5.1 Verify existing `AudioRecordingManager` tests pass with the new lifecycle.
- [x] 5.2 Manually verify the macOS orange privacy indicator disappears within a few seconds of stopping a recording.
- [x] 5.3 Conduct empirical log-based verification: Start a recording, stop it, then start it again. Confirm from the `stream_open_subphase` logs that the second startup hits the `< 50ms` latency band.
