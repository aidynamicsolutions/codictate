# Change: Update Audio Startup Warm Path

## Why
Push-to-talk startup latency is currently dominated by microphone topology enumeration and stream opening on the user-trigger path. Recent log analysis showed startup variance between back-to-back recordings because the app no longer overlaps microphone preparation with the `Fn` press, especially on the system-default/internal microphone path where the user expects startup to feel immediate.

The previously reverted optimization reduced latency but bundled together several lifecycle and cache-management behaviors that increased complexity and made stale-topology handling harder to reason about. This change restores only the safe, measurable parts of that idea and makes the verification contract explicit enough to prove the fast path is actually used from logs alone.

## What Changes
- Add a scoped warm-path optimization for on-demand recording that begins microphone prearm only after a user trigger begins (`Fn` down or equivalent shortcut press), dedupes concurrent stream-open work, and auto-closes unused warm streams after a short grace window.
- Require a positive default-route fast path so unchanged system-default input starts reuse warm or cached topology instead of always fresh-enumerating.
- Tighten cache-safety rules so default-route starts do not trust stale topology snapshots without route-change confirmation.
- Add structured audio-startup timing logs and trigger-to-session correlation so developers can compare `fn_press -> connecting_overlay -> stream_open_ready -> recording_overlay` across sessions from the unified log alone.
- Defer stable explicit microphone identity migration to a follow-up change after the first-pass warm-path fix is validated.

## Impact
- Affected specs: `audio-recording`, `observability`
- Affected code: `src-tauri/src/managers/audio.rs`, `src-tauri/src/fn_key_monitor.rs`, `src-tauri/src/actions.rs`
