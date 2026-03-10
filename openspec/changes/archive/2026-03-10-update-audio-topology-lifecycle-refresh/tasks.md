## Scope Note
This follow-up change builds on the completed first-pass warm-path work in `update-audio-startup-warm-path`. The checklist below covers only the second-pass scope for long-lived event-driven cache refresh, unchanged default-route safety, and onboarding/microphone-picker guidance.

## 1. Spec and Design
- [x] 1.1 Add `audio-recording` spec deltas for:
  - a 24-hour topology cache TTL
  - event-driven background refresh on wake, app foreground, and audio-route change
  - the rule that background refresh must never open the microphone stream
  - unchanged default-route safety requirements despite the longer-lived cache
  - user-facing guidance for explicit built-in/internal microphone selection vs `Default`
- [x] 1.2 Add `observability` spec deltas for:
  - structured cache-refresh lifecycle logs
  - machine-readable refresh source and outcome fields
  - manual verification expectations for long-idle and finalized lifecycle-refresh scenarios
- [x] 1.3 Finalize `design.md` with a concrete lifecycle model, refresh-source matrix, invalidation rules, failure handling, copy strategy, and verification plan before coding begins.

## 2. Backend Cache Lifecycle
- [x] 2.1 Extend the topology cache from 10 minutes to 24 hours and record enough metadata to distinguish:
  - cache age
  - refresh source
  - route generation captured at refresh time
  - whether the cache was refreshed opportunistically or on the startup path
- [x] 2.2 Add a single-flight background refresh path that enumerates devices and refreshes topology cache without opening the recorder stream or lighting the microphone indicator.
- [x] 2.3 Wire app foreground or activation refresh using existing Tauri app or window lifecycle signals.
- [x] 2.4 Wire wake-from-sleep refresh on macOS via native workspace notifications, and keep unsupported platforms on the explicit skip path rather than synthesizing wake state.
- [x] 2.5 Remove unlock or session-active refresh from the final implementation and spec after runtime verification showed the notification path was not reliable enough to keep.
- [x] 2.6 Wire audio-route or default-input change refresh adjacent to the existing route-monitoring path instead of scattering it across unrelated files.
- [x] 2.7 Coalesce duplicate lifecycle refresh requests so bursts of wake, foreground, and route events do not spawn parallel device enumerations.
- [x] 2.8 Ensure refresh failures never block transcription start; if refresh fails, the next start must continue to use the existing conservative startup fallback rules.

## 3. Startup Resolution Rules
- [x] 3.1 Preserve the existing warm-path prearm behavior and ownership safety while allowing refreshed topology to seed later starts after long idle periods.
- [x] 3.2 Keep default-route startup safety exactly as strict as it is now:
  - reuse cached topology only when route state is current and unchanged
  - force fresh resolution when route state is changed or unknown
  - keep unsupported or unavailable route-monitoring behavior conservative
- [x] 3.3 Allow explicit microphone selections to benefit from the 24-hour cache when the selected device remains valid, while preserving:
  - explicit-device disconnect recovery
  - cached-open fresh retry
  - Bluetooth avoidance heuristics
  - Continuity Camera filtering
  - clamshell microphone override behavior
- [x] 3.4 Ensure lifecycle refresh does not weaken the existing policy of preferring a better non-Bluetooth alternative over the macOS default Bluetooth microphone when possible.

## 4. Observability and Verification
- [x] 4.1 Add structured logs for every cache refresh attempt with fields for:
  - refresh source
  - outcome (`started`, `completed`, `skipped`, `failed`, `coalesced`)
  - duration
  - device count
  - captured route generation
  - whether the refresh updated the shared topology cache
  - skip reason when a lifecycle hook is unsupported or intentionally not registered
- [x] 4.2 Add structured startup logs that make it clear whether a fast start reused:
  - warm stream state
  - event-refreshed cached topology
  - a fresh startup-path enumeration
  - the structured `fresh_reason` when startup must resolve from fresh topology
- [x] 4.3 Add or update focused tests for:
  - long-lived cache TTL behavior
  - lifecycle-refresh dedupe and coalescing
  - unsupported wake hook behavior on non-macOS paths
  - refresh-without-stream-open behavior
  - default-route reuse after unchanged wake or foreground refresh
  - explicit-device reuse after long idle
  - external interface power-cycle recovery through fresh live re-enumeration
  - conservative fallback when refresh metadata is missing or stale
- [x] 4.4 Run backend tests for the audio manager and any lifecycle wiring added for refresh triggers.
- [x] 4.5 Run frontend validation for the onboarding and microphone-copy changes, including i18n coverage for new user-facing strings.
- [x] 4.6 Perform manual runtime verification for the finalized lifecycle scope and startup provenance:
  - app foreground refresh completed and the following startup reused refreshed cache with preserved provenance
  - wake bridge delivery and refresh completed; the recorded wake run stayed cache-based, though the subsequent startup reused a later route-change refresh that landed after wake
  - idle route change refreshed cache and the following startup reused refreshed route-change topology with preserved provenance
- [x] 4.7 Have a developer exercise lifecycle transitions directly in the app and capture unified-log evidence for each pass:
  - app background to foreground, then push-to-talk
  - macOS sleep to wake, then push-to-talk
  - idle route change such as unplug, replug, or AirPods connect or disconnect, then push-to-talk
  - each pass should capture the lifecycle trigger log, refresh outcome log, and the following startup resolution log
- [x] 4.8 Record post-idle lifecycle results and confirm whether the first post-idle run avoids fresh enumeration in the expected stable-setup cases.
  - foreground pass reused refreshed cache with `cache_refresh_source="app_foreground"`
  - idle route-change pass reused refreshed cache with `cache_refresh_source="audio_route_change"` and no `startup_path` refresh for the final verified run
  - wake pass confirmed bridge delivery and refresh observability, with cache-based startup preserved even though the captured run attributed reuse to a later route-change refresh
- [x] 4.9 After the developer collected the runtime logs from the lifecycle exercise passes, hand those logs back for an independent verification pass to confirm the implementation matches the finalized spec and the expected fast path was actually used.

## 5. Product Copy
- [x] 5.1 Update onboarding copy to recommend choosing a specific built-in or internal microphone for the most consistent startup speed when one is available.
- [x] 5.2 Update microphone-picker copy to explain that:
  - a specific built-in or internal microphone gives the most predictable startup behavior
  - `Default` follows macOS input changes automatically
  - `Default` is the more flexible choice, not necessarily the most consistent one
- [x] 5.3 Route all new user-facing text through i18n resources and keep the wording neutral for users who intentionally prefer `Default` or external-device workflows.

## 6. Documentation
- [x] 6.1 Update existing microphone-startup architecture docs, including `doc/hotkeyshortcut.md` and `GEMINI.md`, so they describe how this follow-up lifecycle-refresh work builds on `update-audio-startup-warm-path`.
- [x] 6.2 Document the end-to-end microphone speed optimization architecture across both related changes, including:
  - first-pass trigger-bound warm path and route-aware default reuse
  - second-pass lifecycle refresh, long-lived topology cache, and post-idle verification workflow
  - the structured logs developers should inspect during manual verification
- [x] 6.3 If the existing docs cannot clearly hold that combined architecture, add a dedicated document under `doc/` that becomes the canonical reference for microphone startup optimization behavior and verification.
