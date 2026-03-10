## 1. Spec and Design
- [x] 1.1 Finalize `proposal.md` so it explicitly frames this work as on-demand startup latency optimization, not as a topology-refresh fix.
- [x] 1.2 Finalize `design.md` with:
  - the neutral pre-ready shell decision
  - the bars-equals-ready invariant
  - stream-open and capture-ready optimization boundaries
  - post-wake overlay fallback behavior
  - phase-level validation guidance anchored to the March 10, 2026 baseline
- [x] 1.3 Add `audio-recording` and `observability` spec deltas that separate perceived latency improvements from actual capture-ready latency improvements.

## 2. On-Demand Feedback UX
- [x] 2.1 Replace the delayed readable connecting-start policy on the on-demand path with a neutral non-text pre-ready shell that appears immediately after accepted on-demand trigger handling when overlay feedback is enabled.
- [x] 2.2 Remove the fixed non-Bluetooth `220 ms` readability holdback as the gating policy for early feedback.
- [x] 2.3 Ensure the pre-ready shell:
  - never shows bars
  - never implies the microphone is ready
  - replaces the current text-based connecting state for on-demand startup
  - cleans up correctly on cancellation, failure, and permission denial
- [x] 2.4 Preserve recording bars as the first ready-to-speak indicator.
- [x] 2.5 Add or update frontend overlay coverage so the pre-ready shell and its transition into recording bars are testable and visually distinct.

## 3. Audio Startup Path
- [x] 3.1 Add or update observability so the earliest input callback boundary and the later `startup_trigger_received` boundary are both measurable.
- [x] 3.2 Investigate and reduce avoidable delay between accepted trigger handling, prearm dispatch, and actual stream-open work.
- [ ] 3.3 Restructure or narrow stream-open coordination so recorder creation, device open, and first-packet wait are optimized without breaking single-flight safety.
- [x] 3.4 Investigate and reduce the `stream_open_ready -> capture_ready_ack` gap without weakening capture-readiness correctness.
- [x] 3.5 Preserve current safety for:
  - cancellation during prepare or start
  - maintenance-mode blocking
  - permission denial
  - warm-path ownership and cleanup

## 4. Overlay Wake Fallback
- [x] 4.1 Add a wake-aware or AX-instability-aware fallback path for overlay positioning so slow authoritative AX lookups do not block the first visible overlay state longer than necessary.
- [x] 4.2 Implement a concrete fallback trigger policy for startup presentation that uses:
  - a 5-second recency window after wake or unlock
  - transient AX failure or timeout conditions during startup presentation
- [x] 4.3 Refine fallback positioning asynchronously once authoritative AX state stabilizes.
- [x] 4.4 Keep fallback limited to presentation and positioning semantics, not recording readiness semantics.

## 5. Observability
- [x] 5.1 Add structured startup logs for:
  - earliest input receipt
  - trigger acceptance
  - prepare and prearm dispatch
  - stream-open subphases
  - recorder start acknowledgement subphases
  - pre-ready shell shown
  - recording bars shown
- [x] 5.2 Add structured logs that distinguish:
  - perceived-latency milestones
  - actual audio-readiness milestones
  - overlay fallback reason and refinement outcome
- [x] 5.3 Preserve trigger correlation across pre-session and session-bound startup logs so early-input, startup, and overlay events can be attributed to the same run.
- [x] 5.4 Ensure cache-hit runs remain diagnosable so developers can prove that topology refresh was not the dominant bottleneck in a given session.

## 6. Validation
- [ ] 6.1 Add or update focused backend tests for stream-open coordination, capture-ready acknowledgement ordering, and cleanup under cancellation or failure.
- [x] 6.2 Add or update focused tests for overlay fallback behavior after wake or transient AX failure, plus UI-state tests that bars never appear before capture-ready.
- [ ] 6.3 Perform manual runtime verification for:
  - steady-state cached internal microphone
  - app foreground then start
  - wake then start
  - long idle then start
  - wake with transient AX instability
- [ ] 6.4 Run at least 5 repetitions per manual scenario and record the median and slowest pass for:
  - trigger accepted -> first visible feedback
  - trigger accepted -> stream open ready
  - trigger accepted -> capture-ready ack
  - capture-ready ack -> recording bars shown
- [ ] 6.5 Confirm from logs that bars never appear before capture-ready acknowledgement and that perceived-latency wins are not being reported as actual readiness wins.
