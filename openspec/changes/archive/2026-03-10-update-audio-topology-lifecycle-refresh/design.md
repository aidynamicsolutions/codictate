## Context
The first-pass warm-path work already proved that overlapping microphone stream open with the recording trigger materially improves push-to-talk startup latency. March 10, 2026 runtime validation showed:

- explicit-selection warm-path runs as low as roughly `238 ms` to recording overlay
- default-route warm-path runs as low as roughly `211 ms` to recording overlay
- the slowest observed first-pass run still improved materially from the earlier `~1.17 s` baseline

That first pass solved the main trigger-path regression, but it still leaves one UX gap: after a longer idle period, the app can fall back to `fresh` topology resolution even when the user’s environment has not actually changed. Users experience that as “the first transcription after a break is slower again.”

This follow-up is not about keeping the microphone stream open longer. It is about keeping trustworthy topology knowledge around longer, refreshing it opportunistically before the user presses the trigger, and explaining the microphone-selection tradeoff clearly in the product UI.

## Goals / Non-Goals
### Goals
- Reduce the chance that the first transcription after a long idle period falls back to a user-visible `fresh` topology scan in stable environments.
- Keep default-route safety exactly as strict as the first-pass implementation, even with a much longer cache TTL.
- Refresh topology cache proactively on meaningful lifecycle and routing events without opening the microphone stream.
- Keep the refresh path single-flight, coalesced, and non-blocking so it cannot become a new source of startup latency.
- Improve onboarding and microphone-selection guidance so users understand that a specific built-in/internal microphone is the most consistent low-latency choice, while `Default` prioritizes flexibility.
- Extend log-based verification so the team can prove the refreshed fast path was actually used after long idle and lifecycle transitions.

### Non-Goals
- Re-implement the first-pass warm path; this change builds on it.
- Keep the microphone stream or recorder stream open for hours.
- Relax default-route safety checks to “trust a long TTL.”
- Replace route-generation monitoring with lifecycle events; lifecycle refresh is additive, not a substitute.
- Rework microphone identity persistence or settings schema beyond the copy and cache-lifecycle changes needed here.
- Force users off `Default`; the UX should recommend consistency, not remove flexibility.

## Current Behavior Baseline
The current implementation has three relevant layers:

1. Warm stream reuse
   - `Fn` down or equivalent trigger can begin prearm.
   - The actual recording start can reuse that already-open stream and log `resolution_mode="warm"`.

2. Topology cache
   - Device enumeration results are cached for 10 minutes.
   - Explicit microphone selections can reuse that cache more aggressively than `Default`.
   - Default-route reuse is still gated by route-change state.

3. Fresh fallback
   - If cache is stale, route state is uncertain, the selected device is missing, or cached open fails, the app enumerates devices again on the startup path.

This change extends layer 2 and improves the user guidance around layer 1 vs layer 3. It does not change the fact that layer 1 (`warm`) remains the best-case startup path.

## Decision: Preserve first-pass warm-path ownership and cleanup semantics
The long-idle cache revision extends topology lifetime, not warm-stream lifetime.

### Behavior
- Warm prearm still begins only after a concrete user recording trigger starts.
- Unused warm streams still auto-close after a bounded grace window of no more than 1 second.
- If transcription start is later blocked by maintenance mode, permission denial, or state cancellation, the system must synchronously close any warm stream it owns instead of waiting for the grace timer.
- Background topology refresh must not take ownership of, prolong, reopen, or otherwise depend on warm-stream state.

### Why
- The first-pass warm path depends on tight ownership and cleanup rules.
- The second-pass cache-lifecycle work should not accidentally reintroduce “warm stream left behind” failures while optimizing long-idle starts.

## Decision: Promote the topology cache from short-lived TTL to long-lived event-driven cache
The topology cache should be treated as a long-lived, event-refreshed resource rather than a short-lived startup-only optimization.

### Behavior
- Increase cache TTL from 10 minutes to 24 hours.
- Treat the 24-hour TTL as a backstop, not the primary validity signal.
- Refresh topology cache in the background on meaningful events:
  - app foreground / activation
  - wake from sleep
  - audio-route / default-input change
- Background refresh enumerates devices and updates cache metadata only.
- Background refresh must never open the microphone stream, must never light the mic indicator, and must never block a user-triggered start from proceeding.

### Why
- Real users pause for longer than 10 minutes even when their microphone setup does not change.
- A long-lived topology cache is cheap to keep because it stores device metadata, not an active stream.
- Event-driven refresh reduces the need to pay fresh-enumeration cost at trigger time without trusting stale topology blindly.

### Platform scope
- App foreground and activation refresh should use existing Tauri app or window lifecycle signals where available.
- Wake-from-sleep refresh is macOS-only in this change, using native workspace notifications.
- Unlock or session-active refresh is intentionally not part of the final scope; app foreground, wake, and route-change coverage proved adequate without relying on a less reliable session-active notification path.
- Windows and Linux do not gain wake lifecycle refresh in this proposal revision; they should follow the same unsupported-to-skip pattern already used for missing route-monitoring support.
- Audio-route and default-input refresh should inherit the current platform coverage of the existing route-monitoring path rather than introducing a separate platform-specific mechanism here.

## Decision: Keep default-route safety exactly as strict as the current implementation
The proposal revision must not weaken the conservative rules that protect `Default`.

### Behavior
- `Default` may reuse cached topology only when route state proves that the current default input route is unchanged relative to the cached entry.
- Background refresh may update cached route metadata, but it does not override the rule above.
- If route state is changed, missing, unsupported, or otherwise uncertain, `Default` must still resolve from fresh topology on the startup path.
- Lifecycle refresh hooks are additive safety and UX improvements; they do not replace route-generation checks.

### Why
- The default route can change outside the app at any time.
- A longer TTL is safe only if route-change state remains the real authority for `Default`.

## Decision: Let explicit microphone selections benefit most directly from the longer-lived cache
Explicit microphone selection is the path where the cache extension should deliver the clearest UX benefit.

### Behavior
- When a specific microphone is selected, cached topology should be reused for up to 24 hours unless a refresh or invalidation occurs first.
- If the selected device is missing in cached topology, startup must immediately refresh on the startup path before making fallback or persistence decisions.
- If opening from cached topology fails, startup must retry once with a full fresh live device enumeration before surfacing failure.
- Existing Bluetooth avoidance, Continuity Camera filtering, clamshell override behavior, and explicit-device disconnect recovery remain intact.

### Why
- Explicit selection is the lowest-ambiguity input choice.
- Users who care most about consistent startup speed are the users most likely to accept a fixed built-in/internal microphone.

## Decision: Background refresh must be single-flight and non-blocking
The app may receive bursts of lifecycle events. Refresh behavior must be deterministic and low-risk.

### Behavior
- Multiple refresh triggers arriving close together must coalesce into one background enumeration pass.
- If a refresh is already in flight, later compatible refresh requests should log as `coalesced` or `skipped`, not start parallel scans.
- The topology-resolution decision point is the moment immediately before startup would otherwise begin its own live device enumeration or commit a cached topology target for stream-open.
- If a user-triggered start happens while refresh is running, startup should not wait indefinitely for refresh; it should continue with the existing startup rules and only reuse refresh results if they are already available by the topology-resolution decision point.
- Refresh failure must degrade gracefully to the current conservative startup behavior rather than blocking transcription.

### Why
- Wake, foreground, and route-change events often cluster.
- Parallel enumerations would waste work and make logs ambiguous.
- The goal is to move work off the trigger path, not to add a new dependency that can stall it.

## Decision: Onboarding and microphone picker should explain consistency vs flexibility
The UX should make the startup-latency tradeoff understandable without being coercive.

### Product guidance
- Recommend a specific built-in/internal microphone as the most consistent startup-speed option when one is available.
- Explain `Default` as the flexible option that follows macOS input changes automatically.
- Do not claim that `Default` is always slower; the app can still make `Default` fast when route state is confirmed.
- Do not imply that external microphones are wrong; the message should be about predictability and flexibility tradeoffs.

## Lifecycle Model
### Cache state model
- Empty:
  - app has no cached topology yet
  - next start must enumerate unless a background refresh completes first
- Fresh and valid:
  - cached entry is younger than 24 hours
  - startup may reuse it subject to explicit/default rules
- Freshly refreshed:
  - cache was updated by a lifecycle or route event before the user triggered recording
  - startup may reuse it subject to explicit/default rules
- Expired:
  - cache age exceeded 24 hours
  - next start must refresh on the startup path if no background refresh occurs first

### Refresh source matrix
| Source | Current platform scope | Expected action | Stream open allowed | Startup fallback if unsupported or missed |
| --- | --- | --- | --- | --- |
| App foreground / activation | Cross-platform where Tauri app or window activation events exist | Schedule a topology-only background refresh | No | Keep existing cache state; startup still falls back to normal cache/fresh rules |
| Wake from sleep | macOS only in this change | Schedule a topology-only background refresh | No | Skip this refresh source and keep existing cache state; startup still falls back to normal cache/fresh rules |
| Audio-route / default-input change | Same platforms and runtime paths as existing route monitoring | Refresh topology and update captured route-generation metadata | No | Treat `Default` conservatively and force fresh startup-path resolution when route state is uncertain |

### Startup decision model
- Best case:
  - warm stream already opened by prearm
  - startup logs `resolution_mode="warm"`
- Next best:
  - no open stream yet, but topology cache is trusted
  - startup logs `resolution_mode="cache"`
- Slow path:
  - topology must be enumerated on the startup path
  - startup logs `resolution_mode="fresh"`

## Observability Requirements
The existing startup logs are necessary but not sufficient once lifecycle refresh is added.

### Required refresh log fields
- refresh source
- refresh outcome
- refresh duration
- device count
- route generation captured at refresh time
- whether cache was updated
- whether a refresh request was coalesced into an existing one

### Required startup interpretation
The logs must let a developer answer:
- Did a background refresh happen before this transcription?
- Did the start reuse that refreshed topology?
- Was this start warm, cache-based, or fresh?
- If it was fresh, was that because the cache expired, route state changed, route state was unknown, or cached open failed?

### Required startup reason field
- When startup logs `resolution_mode="fresh"`, it must also log a structured `fresh_reason` field.
- `fresh_reason` must distinguish at least cache expiry, route changed, route unknown, and cached explicit-device open failure.

## Risks / Trade-offs
- Longer-lived cache increases the blast radius of stale-cache bugs if invalidation coverage is incomplete.
  - Mitigation: keep default-route gating unchanged, keep explicit-device missing/open-failure refresh paths, and log refresh sources and outcomes clearly.
- Background refresh may create extra device scans around wake and foreground.
  - Mitigation: coalesce refresh requests and keep refresh work topology-only.
- Copy that pushes too hard toward explicit internal microphones could confuse users who intentionally rely on `Default`.
  - Mitigation: describe built-in/internal microphones as the most consistent option, and `Default` as the flexible option.

## Edge Cases
- User sleeps the machine with one audio topology and wakes with another.
  - background refresh should update cache before the next likely trigger
- Route-change monitor fires while the app is backgrounded.
  - route generation and cache refresh should update, but `Default` still only reuses if startup sees confirmed current route state
- User connects AirPods while `Default` is selected.
  - refresh should notice the topology change, but existing Bluetooth-avoidance fallback rules still apply when a better non-Bluetooth microphone is available
- User uses `Default` on a platform where route-monitoring support is incomplete.
  - longer TTL must not bypass the existing fresh-if-uncertain rule
- User has no built-in/internal microphone available.
  - copy should still explain the consistency/flexibility tradeoff without referencing an option that is not present
- App foreground, wake, and route-change events all fire in a short burst.
  - refresh work should coalesce rather than enumerate devices three times
- Warm prearm begins, but transcription is then blocked by maintenance mode or permission denial.
  - any owned warm stream should be closed synchronously rather than waiting for the grace timer

## Migration Plan
1. Keep `update-audio-startup-warm-path` as the completed first-pass change.
2. Implement backend support for:
   - 24-hour topology cache TTL
   - single-flight background refresh
  - foreground, wake, and route-change refresh triggers
   - structured refresh logs
3. Preserve the current default-route startup safety contract and existing explicit-device recovery paths while adopting the new cache lifecycle.
4. Update onboarding and microphone-selection copy, including i18n resources.
5. Validate with:
   - automated tests for refresh, reuse, and fallback behavior
   - manual long-idle and route-change runs
   - log-based confirmation that refreshed topology actually reduces fresh startup-path enumerations
