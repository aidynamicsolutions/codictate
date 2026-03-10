# Change: Update On-Demand Recording Startup Latency

## Why
Manual verification on March 10, 2026 showed that lifecycle-driven topology refresh is working correctly on wake and foreground, but users can still perceive a slow start after pressing `Fn` because recorder startup remains the dominant cost on the on-demand path.

In the wake verification run:
- raw `Fn` press was observed at `2026-03-10T11:47:41.699849Z`
- `startup_trigger_received` logged at `2026-03-10T11:47:41.815470Z`
- `prearm_requested` logged at `2026-03-10T11:47:41.841023Z`
- cached topology was reused at `2026-03-10T11:47:41.842859Z`
- the connecting overlay was shown at `2026-03-10T11:47:42.172885Z`
- audio startup logged `Lock held for 357.764792ms, total init: 436.885666ms`
- `stream_open_ready` logged at `2026-03-10T11:47:42.279486Z`
- `capture_ready_ack` logged at `2026-03-10T11:47:42.461147Z`
- the recording overlay was shown at `2026-03-10T11:47:42.558746Z`

That run shows the bottleneck is no longer topology refresh. The remaining latency is dominated by the on-demand startup path itself: accepted-trigger handling, prearm scheduling, stream open, recorder start acknowledgement, and post-wake overlay presentation. This change addresses that path directly without reframing it as a topology-refresh fix.

## What Changes
- Add a dedicated on-demand startup latency proposal that is separate from lifecycle topology refresh and explicitly treats cached-topology success as the baseline, not the optimization target.
- Replace the current readable delayed “connecting microphone” experience on the on-demand path with an immediate non-text neutral pre-ready overlay shell that appears before capture-ready without violating the bars-equals-ready UX contract.
- Evaluate and specify recorder startup optimizations in the stream-open and capture-ready phases, with safety rules that prevent any ready-to-speak UI from appearing before capture is actually safe.
- Add a post-wake overlay fallback path so transient Accessibility instability does not block early visual feedback or recording overlay presentation longer than necessary.
- Extend structured observability so future validation can attribute startup time across input receipt, trigger acceptance, prearm dispatch, stream open, start acknowledgement, and overlay presentation.
- Require manual verification to report repeated-run measurements per scenario rather than relying on a single anecdotal pass.

## Impact
- Affected specs: `audio-recording`, `observability`
- Related changes:
  - `update-audio-topology-lifecycle-refresh`
  - archived `2026-03-10-update-audio-startup-warm-path`
- Affected code:
  - `src-tauri/src/actions.rs`
  - `src-tauri/src/fn_key_monitor.rs`
  - `src-tauri/src/managers/audio.rs`
  - `src-tauri/src/audio_toolkit/audio/recorder.rs`
  - `src-tauri/src/overlay.rs`
  - `src-tauri/src/accessibility/macos.rs`
