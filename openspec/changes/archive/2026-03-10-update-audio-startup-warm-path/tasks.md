## 1. Implementation
- [x] 1.1 Add `audio-recording` spec deltas for on-demand microphone warm path behavior, default-route fast-path reuse, and cache-safety rules.
- [x] 1.2 Add `observability` spec deltas for session-correlated audio startup timing logs and verification coverage.
- [x] 1.3 Implement a narrow user-triggered prearm path for on-demand recording that overlaps stream opening with `Fn` down or equivalent shortcut start, dedupes concurrent open work, and auto-closes unused warm streams.
- [x] 1.4 After `1.3`, implement the positive default-route fast path so unchanged system-default input starts reuse warm or cached topology instead of always fresh-enumerating.
- [x] 1.5 After `1.3`, implement route-aware cache invalidation for default-route starts on macOS and require conservative fresh-resolution fallback when route state is unavailable or unsupported.
- [x] 1.6 Add a startup `trigger_id` correlation key that is emitted before session creation and propagated into later session-bound startup logs.
- [x] 1.7 Preserve existing explicit-device, Bluetooth avoidance, Continuity Camera filtering, permission denial, hands-free overlap, and maintenance-mode behaviors while applying the warm-path changes.
- [x] 1.8 Add structured audio-startup timing logs for trigger receipt, prearm lifecycle, topology resolution mode, stream-open readiness, capture-ready acknowledgement, connecting overlay readiness, and recording overlay readiness.
- [x] 1.9 Document stable explicit microphone identity migration as follow-up work rather than implementing settings/API schema changes in this change.
- [x] 1.10 Update maintained developer-facing architecture docs that describe shortcut/audio startup flow, including `GEMINI.md` and `doc/hotkeyshortcut.md`, to reflect the warm path and trigger-to-session logging behavior.

## 2. Verification
- [x] 2.1 Add or update focused tests for warm-path state transitions, auto-close behavior, duplicate-open dedupe, default-route fast-path reuse, route-aware invalidation decisions, and trigger-to-session correlation behavior.
- [x] 2.2 Run targeted backend tests for the audio manager changes.
- [x] 2.3 Perform manual log verification from the unified log file by comparing at least two push-to-talk sessions and confirming the timing fields needed to inspect `fn_press -> connecting_overlay -> stream_open_ready -> recording_overlay`.
- [x] 2.4 Document verification findings in the change notes or PR summary, including whether the positive default-route fast path was exercised and any remaining platform-specific limitations.

## Verification Notes
- `cargo test --lib` passed on the final implementation tree in `src-tauri/`.
- Focused coverage was added for default-route invalidation decisions, prearm auto-close ownership, and trigger metadata parsing.
- `openspec validate update-audio-startup-warm-path --strict` passed.
- Manual unified-log verification on `2026-03-10` from `/Users/tiger/Library/Logs/com.pais.codictate/codictate.2026-03-10.log` captured two live push-to-talk sessions with full `trigger_id -> session` correlation:
  - `trigger_id=9f055869`, `session=9fc09b29`: `startup_trigger_received` at `06:31:54.565282Z`, `overlay_connecting_shown` at `06:31:54.895328Z` (~330 ms), `stream_open_ready` at `06:31:55.293249Z` (~728 ms), `overlay_recording_shown` at `06:31:55.430278Z` (~865 ms).
  - `trigger_id=2d3e22a4`, `session=4708e9ec`: `startup_trigger_received` at `06:32:02.246454Z`, no connecting overlay needed, `stream_open_ready` at `06:32:02.414596Z` (~168 ms), `overlay_recording_shown` at `06:32:02.484768Z` (~238 ms).
- Those live runs improved on the prior baseline (`~1.17 s` slow / `~0.32 s` fast) and confirmed warm-path reuse via `resolution_mode="warm"` with `resolution_reason="reused_open_stream"`.
- The live sessions did **not** exercise the positive default-route fast path yet: both `prearm_completed` events logged `resolution_reason="explicit_selection_cache_hit"`, so unchanged system-default reuse still needs a dedicated runtime validation pass.
