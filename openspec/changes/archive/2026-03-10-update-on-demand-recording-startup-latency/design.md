## Context
The archived warm-path change materially improved on-demand startup by overlapping some stream-open work with the user trigger. The active lifecycle-refresh change then addressed the long-idle regression where topology cache reuse became stale after foreground, wake, or unlock.

March 10, 2026 verification showed a new, separate bottleneck: even when topology refresh worked and cached topology was reused successfully, the user could still feel a delayed startup after pressing `Fn`.

Observed wake-path timeline:
- raw `Fn` press: `2026-03-10T11:47:41.699849Z`
- `startup_trigger_received`: `2026-03-10T11:47:41.815470Z`
- `prearm_requested`: `2026-03-10T11:47:41.841023Z`
- cache hit with enumeration skipped: `2026-03-10T11:47:41.842859Z`
- connecting overlay shown: `2026-03-10T11:47:42.172885Z`
- stream-open path: `Lock held for 357.764792ms, total init: 436.885666ms`
- `stream_open_ready`: `2026-03-10T11:47:42.279486Z`
- `capture_ready_ack`: `2026-03-10T11:47:42.461147Z`
- recording overlay shown: `2026-03-10T11:47:42.558746Z`
- total trigger-to-recording-overlay time: about `765 ms`

The timeline implies:
- topology refresh was not the dominant factor in that run
- actual recorder open and start acknowledgement are the main remaining latency sources
- the current UI intentionally hides early feedback for up to `220 ms` on non-Bluetooth starts
- wake-related Accessibility instability can still delay overlay presentation after audio is already progressing

## Goals / Non-Goals
### Goals
- Reduce perceived latency between accepted on-demand trigger and first visible feedback.
- Reduce actual latency between accepted on-demand trigger and capture-ready acknowledgement.
- Preserve the rule that audio bars are the first ready-to-speak indicator.
- Keep all startup optimizations safe under cancellation, maintenance-mode blocking, permission denial, and wake-related platform instability.
- Make the startup timeline diagnosable from logs alone, including the gap before `startup_trigger_received`.

### Non-Goals
- Re-open or redesign lifecycle topology refresh.
- Open or keep the microphone stream alive outside a concrete user trigger.
- Show recording bars before capture-ready is confirmed.
- Change always-on microphone semantics.
- Promise a device-independent hard millisecond SLA in the proposal.

## Decisions
### Decision: Separate perceived responsiveness from capture readiness
The proposal should separate “the trigger was accepted” from “the microphone is ready.”

### Behavior
- On accepted on-demand starts, the app should immediately show a neutral pre-ready shell whenever overlay feedback is enabled.
- The pre-ready shell replaces the current text-based connecting experience for the on-demand path.
- The pre-ready shell is visual acknowledgement only, must not contain recording bars, and should not rely on readable text.
- Recording bars remain exclusive to the capture-ready state.
- Fast starts must not wait behind a fixed readability timer before showing early feedback.

### Why
- The current readable connecting state exists largely to avoid unreadable flashes.
- A neutral capsule shell with a soft pulse or glow does not need a minimum readable duration.
- This preserves the product contract that bars mean “safe to speak.”

### Decision: Do not make bars-only UX the initial proposal baseline
The proposal should not assume complete removal of all pre-ready UI on day one.

### Why
- The measured wake path still leaves roughly `700+ ms` between trigger and recording bars.
- Removing all pre-ready feedback would expose that dead gap to users and make missed-trigger vs slow-start failures harder to distinguish.
- A later follow-up can remove the neutral shell entirely if actual startup becomes consistently fast enough.

### Decision: Treat stream-open work as the primary actual-latency target
Once topology resolution is a cache hit, the remaining startup work should be optimized as its own path.

### Behavior
- The proposal should treat recorder initialization, device open, stream play, and first-packet wait as separate phases.
- Single-flight safety between prearm and user-triggered start must remain intact.
- Blocking coordination should be narrowed where possible so the app does not hold the main startup gate across more work than necessary.
- The proposal must not weaken cancellation or ownership rules already introduced by the warm path.

### Why
- Current logs show `357 ms` lock hold and `437 ms` total init on the stream-open path alone.
- That is larger than the overlay threshold and larger than the capture-ready wait.

### Decision: Reduce capture-ready handshake overhead without weakening the ready contract
The proposal should target the `stream_open_ready -> capture_ready_ack` gap directly.

### Behavior
- The recorder start handshake may be restructured or instrumented more finely.
- Any optimization must preserve the rule that ready UI is shown only after capture can safely begin.
- The proposal should explicitly measure:
  - start command sent
  - worker applied start
  - first post-start frame or equivalent readiness evidence
  - acknowledgement emitted

### Why
- The observed wake run still spent roughly `182 ms` after stream open before `capture_ready_ack`.
- Current implementation likely pays both open-time flow confirmation and post-start flow confirmation.

### Decision: Add a wake-aware overlay fallback path
The proposal should improve overlay responsiveness after wake or unlock when AX is slow or unstable.

### Behavior
- When startup overlay presentation occurs within 5 seconds after a wake or unlock event, or when authoritative AX lookup fails with a transient error such as `AXError -25204` or times out, overlay presentation should use cached or coarse positioning first.
- The overlay should refine its position asynchronously once authoritative AX state stabilizes.
- Overlay fallback affects placement only, not capture-readiness semantics.

### Why
- Current overlay routing attempts authoritative AX queries before fallback, which can delay visible feedback after wake.
- The wake run already showed `focused_lookup_elapsed_ms=221`, which is large enough to affect perception.

### Decision: Observability must distinguish perceived latency from actual readiness
The proposal should make it possible to answer both “when did the user first see feedback?” and “when was capture actually ready?”

### Required timeline buckets
- raw input received or earliest callback entry
- trigger accepted
- prepare state entered
- prearm dispatched and started
- topology resolved
- stream open subphases
- stream open ready
- recorder start command and acknowledgement subphases
- pre-ready shell shown
- recording bars shown
- overlay fallback reason and refinement result

## Alternatives Considered
### Keep the existing text connecting state and only tune the threshold
- Pros:
  - smallest UI change
  - preserves an explicit loading explanation
- Cons:
  - still ties feedback to readability rather than responsiveness
  - still creates text flashes or delayed first feedback

### Remove the connecting state and any pre-ready feedback entirely
- Pros:
  - cleanest end-state UX
  - bars remain the only meaningful signal
- Cons:
  - exposes silent `400-800 ms` gaps on slower starts today
  - makes startup failures feel like missed shortcuts

### Neutral pre-ready shell
- Pros:
  - gives instant acknowledgement without implying readiness
  - can appear immediately without a readability delay
  - preserves bars-only ready contract
- Cons:
  - adds one transitional overlay state
  - still depends on overlay routing responsiveness after wake

## Risks / Trade-offs
- Narrowing lock scope or restructuring startup coordination can introduce new races between prearm, cancellation, and user-triggered start.
  - Mitigation: keep single-flight ownership explicit and preserve warm-path cleanup semantics.
- Earlier visual feedback can be mistaken for readiness if the UI is too similar to recording bars.
  - Mitigation: the pre-ready shell must be visually distinct and must never display levels.
- Wake-aware fallback can place the overlay on a less precise monitor or position briefly.
  - Mitigation: refine asynchronously once authoritative AX state stabilizes and log the fallback reason.

## Validation Plan
1. Capture before and after logs for:
   - steady-state cached internal microphone start
   - app foreground then start
   - wake then start
   - long idle then start
   - wake with transient AX instability
2. Run at least 5 repetitions per scenario and report at minimum the median and slowest pass for:
   - trigger accepted -> first visible feedback
   - trigger accepted -> stream open ready
   - trigger accepted -> capture-ready ack
   - capture-ready ack -> recording bars shown
3. Confirm the following invariants in every run:
   - bars never appear before capture-ready ack
   - no eager microphone open occurs without a user trigger
   - overlay fallback changes placement only, not readiness semantics
4. Use logs alone to determine whether the bottleneck was:
   - input delivery before `startup_trigger_received`
   - prearm scheduling
   - stream open
   - recorder start acknowledgement
   - overlay presentation or AX fallback
