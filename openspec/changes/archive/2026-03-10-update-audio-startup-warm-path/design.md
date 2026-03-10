## Context
The current hot path keeps only a short-lived startup-primed device cache and recorder warmup. Logs from March 9, 2026 show that slower push-to-talk starts are dominated by audio startup:

- slow session: device enumeration `204.8 ms`, total mic init `583.4 ms`
- fast session: device enumeration `85.0 ms`, total mic init `196.9 ms`

The reverted commit `44a885b` addressed this by combining:
- async topology refresh
- route-change tracking
- prearm stream opening on `Fn` down
- extra stream ownership/epoch state

That package improved latency, but the implementation surface was broad enough that it was later reverted. We want to recover the latency benefit without reintroducing the full state complexity.

## Goals / Non-Goals
- Goals:
  - Reduce user-triggered microphone init time for on-demand recording.
  - Require the fast path to apply to unchanged default-route starts, since that is where the measured regression appeared.
  - Preserve correctness for default-route changes, explicit-device disconnects, and cancelled triggers.
  - Add structured timing logs that let developers verify the optimization from a single session timeline.
- Non-Goals:
  - Rebuild the entire reverted audio lifecycle state machine.
  - Keep the microphone stream open indefinitely in on-demand mode.
  - Change always-on microphone semantics.
  - Introduce stable explicit microphone identity persistence in this first-pass change.
  - Redesign settings or audio-selection APIs.

## Decisions

### Decision: Restore only user-triggered prearm, not the full reverted feature set
The implementation will restore a narrow warm path:
- begin prearm only after a concrete user start gesture begins
- dedupe prearm and recording-open work behind a shared open gate
- auto-close the prearmed stream if recording does not commit within a short timeout of no more than 1 second

It will not restore:
- long-lived prearm ownership/epoch machinery unless required by the minimal design
- unconditional cache refresh on every trigger
- broad fallback persistence logic tied to cached topology

Why:
- the main user-visible benefit came from overlapping stream open with the user press
- the larger reverted design was harder to reason about and made cache correctness more fragile

### Decision: Treat explicit-device and default-device starts differently
Explicit device selection and system default routing have different safety properties:

- Default route:
  - must reuse warm or cached topology when route state is current and unchanged
  - must not trust a stale cached topology snapshot unless route-change state confirms it is still current
  - uses fresh resolution after route-change events or when route state is unknown
- Explicit selection:
  - remains functionally correct under the warm-path changes
  - continues using the existing resolution and recovery path in this first implementation phase

Why:
- the system default input can change without the selected display name changing
- the measured latency regression was on the default/internal microphone path, so the positive default-route fast path must be part of the contract

### Decision: Defer explicit microphone identity migration to a follow-up change
Stable explicit microphone identity remains a worthwhile follow-up because display-name matching still requires scans and is ambiguous for duplicate names, but it is not necessary to fix the measured default-route startup regression.

Why:
- the current regression can be addressed by restoring a safe default-route warm path plus better observability
- identity migration would add settings/API scope that is not required to prove the immediate latency fix
- phasing reduces rollout risk and lets developers validate the warm-path improvement independently

### Decision: Add route-aware invalidation on macOS, with safe fallback elsewhere
On macOS, the audio startup path should respond to Core Audio route/default-input changes so default-route starts know when cached topology is no longer trustworthy.

Behavior:
- when default input or device topology changes, invalidate warm topology state for default-route starts
- if the route monitor is unavailable or unsupported, default-route starts skip warm topology reuse and resolve from fresh topology

Why:
- TTL-only cache freshness is insufficient for default-route correctness
- invalidation from real route-change signals avoids both stale opens and unnecessary full rescans

### Decision: Verification is log-first and session-correlated
The change will emit structured timing logs with explicit event codes and a trigger correlation key so developers can compare startup stages without adding ad hoc instrumentation.

Required timing points:
- trigger received with `trigger_id`
- prearm requested / skipped / completed / autoclosed
- topology lookup mode (`warm`, `cache`, `fresh`)
- stream open ready
- capture-ready acknowledgement
- connecting overlay shown
- recording overlay shown

Correlation rules:
- pre-session trigger logs must include `trigger_id`
- once a session id exists, startup logs must include both `trigger_id` and `session`

Why:
- this change is performance-sensitive and regressions are easiest to diagnose from timelines
- the repo already uses unified structured logs as the main debugging surface

## Risks / Trade-offs
- Prearm adds more concurrency to the recording start path.
  - Mitigation: require a single deduped stream-open gate and auto-close unused warm streams quickly.
- Route monitoring can be platform-specific.
  - Mitigation: require fresh default-route resolution when route state is unavailable instead of guessing.
- Short prearm windows could still surface microphone indicator flicker.
  - Mitigation: keep grace windows short, limit prearm to concrete user gestures, and skip prearm in always-on mode.

## Edge Cases
- User presses `Fn`, prearm starts, then releases before recording commit.
  - warm stream must auto-close and no stale ownership may block the next trigger
- Hands-free mode is already active when `Fn` is pressed.
  - prearm must not start a parallel push-to-talk session
- Backup/restore maintenance mode blocks transcription start.
  - prearm must not leave a warm stream behind
- Microphone permission is denied.
  - prearm failure must be non-fatal and must not delay the existing denial flow
- Explicit microphone was unplugged between save and trigger.
  - warm-path changes must not break the existing fallback logic
- Default input changes between startup cache prime and first trigger.
  - route invalidation must force a fresh default-route resolution
- Bluetooth or Continuity Camera devices appear/disappear during warm path.
  - fallback selection and cache invalidation must still respect existing device-quality heuristics
- Trigger log occurs before session creation.
  - startup verification must rely on `trigger_id` bridging pre-session and session-bound logs

## Migration Plan
1. Add spec coverage for the warm path and timing logs.
2. Implement Phase 1 narrow warm path:
   - user-triggered prearm
   - deduped stream opening
   - positive default-route warm reuse when route state is current
   - route-aware default-route invalidation with conservative fallback
   - timing logs with trigger-to-session correlation
3. Validate using unified logs against repeated push-to-talk runs.
4. Create a follow-up change for explicit microphone identity migration after the warm-path fix is validated.
