# Change: Update Audio Topology Lifecycle Refresh

## Why
The first-pass warm-path change already restored user-triggered prearm and materially improved push-to-talk startup latency, but long-idle sessions can still regress to `fresh` topology resolution even when the user’s microphone environment has not changed. That leaves a noticeable UX gap: the first transcription after a break can feel slower again despite the warm-path fix being correct.

This follow-up change improves long-idle startup consistency by keeping topology knowledge current before the user presses the trigger, while preserving the existing default-route safety contract and explaining the microphone-selection tradeoff more clearly in the product UI. The final shipped scope keeps foreground, wake, and audio-route refresh, and intentionally drops the lock-to-unlock refresh path after runtime verification showed it was not reliable enough to justify continued complexity.

## What Changes
- Add event-driven topology-cache refresh hooks for meaningful lifecycle changes such as app foreground, wake, and audio-route/default-input change.
- Raise the topology cache TTL from 10 minutes to 24 hours while keeping lifecycle and route invalidation as the primary correctness mechanism rather than trusting the longer TTL alone.
- Require background refresh to stay topology-only, single-flight, coalesced, and non-blocking so it never opens the microphone stream or delays recording start.
- Preserve default-route safety exactly as it works after the first-pass warm-path change: cached topology may be reused only when route state proves the route is unchanged.
- Let explicit microphone selections benefit most directly from the longer-lived cache, including retrying once with fresh topology if cached open fails.
- Add structured refresh lifecycle logs so developers can verify whether long-idle starts reused refreshed cached topology or still fell back to fresh startup-path enumeration.
- Update onboarding and microphone-picker copy to recommend a specific built-in/internal microphone as the most consistent startup-speed option while explaining that `Default` follows macOS input changes automatically.

## Impact
- Depends on: `update-audio-startup-warm-path`
- Affected specs: `audio-recording`, `observability`
- Affected code: `src-tauri/src/managers/audio.rs`, lifecycle wiring under `src-tauri/src/lib.rs` or a new `src-tauri/src/` lifecycle module, existing route-change monitoring paths, and onboarding/microphone-selection UI copy in `src/`
