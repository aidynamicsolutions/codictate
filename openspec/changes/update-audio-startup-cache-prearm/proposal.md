# Change: Reduce First-Word Clipping with Hybrid Cache Refresh and Early Pre-Arm

## Why
On-demand recording still has a cold-start window where users can begin speaking before the stream is capture-ready, causing first-word clipping in short utterances. Current device-cache TTL is also short, so startup frequently pays a full device enumeration cost after brief idle periods.

## What Changes
- Increase input-device cache TTL to 10 minutes.
- Add hybrid cache invalidation/refresh behavior:
  - startup prime
  - fn key down, generic shortcut/signal triggers, and main-window focus refresh with `IfStaleOrDirty` policy
  - selected/clamshell microphone updates mark cache dirty and request forced refresh
  - refresh throttle and in-flight guard to prevent noisy duplicate scans
- Add on-demand pre-arm API and behavior:
  - idempotent non-blocking pre-arm kickoff for transcription start paths
  - shared stream open/close serialization path across pre-arm, startup Bluetooth prewarm, and start/stop operations
  - pre-arm grace timeout (900ms) with ownership-safe auto-close only when pre-arm opened the stream, ownership token still matches, stream epoch still matches, and recording does not commit
  - monotonic stream-epoch identity and pre-arm owner tokens to prevent stale timeout workers from closing newer stream instances
- Add system input-route change monitoring (macOS) and cache-bypass safeguards so default-route starts never trust stale cached topology.
  - use monotonic route-change generation tracking (instead of consume/reset counters) so concurrent starts cannot drop route-change signals before refresh is applied
- Restrict startup prewarm stream opens from persisting fallback microphone auto-switch decisions.
- Add structured observability for cache and pre-arm lifecycle events.
- Add unit tests for cache decision logic and ownership-aware pre-arm auto-close behavior.

## Impact
- Affected specs:
  - `shortcut-settings`
  - `observability`
- Affected code:
  - `src-tauri/src/managers/audio.rs`
  - `src-tauri/src/actions.rs`
  - `src-tauri/src/fn_key_monitor.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/audio.rs`
