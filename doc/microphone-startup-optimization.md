# Microphone Startup Optimization

This document is the canonical reference for microphone startup-speed behavior.
It describes the combined architecture across the first-pass warm-path change
(`update-audio-startup-warm-path`) and the second-pass lifecycle-refresh change
(`update-audio-topology-lifecycle-refresh`).

## Goals

- Keep push-to-talk startup fast when the user has just pressed the trigger.
- Keep the first post-idle run fast when the microphone topology has not changed.
- Preserve conservative `Default` safety when route state is unknown or changed.
- Avoid background work that opens the recorder stream or lights the mic indicator.

## Layered Startup Model

### 1. Trigger-bound warm path

- `Fn` down and the earliest non-Fn shortcut boundary start a narrow prearm.
- Prearm opens the recorder stream behind a single serialized gate.
- The later recording start reuses the same in-flight or ready stream when possible.
- Unused warm streams auto-close after a short grace window.

### 2. Long-lived topology cache

- Device enumeration metadata is cached for up to 24 hours.
- The cache stores:
  - cache age
  - captured route generation
  - refresh source
  - refresh path (`startup_path` vs `opportunistic`)
- The TTL is only a backstop. Route awareness remains the real safety signal.

### 3. Lifecycle-driven background refresh

- The app refreshes topology metadata in the background on:
  - app foreground / focus
  - macOS wake from sleep
  - audio-route change
- Refresh is single-flight and coalesced.
- Refresh enumerates devices only. It never opens the recorder stream.
- Startup never waits on an in-flight refresh; it only reuses refresh results if they
  are already available when startup resolves topology.

### 4. Fresh startup-path fallback

- Startup still performs a fresh enumeration when:
  - the cache is missing
  - the cache is older than 24 hours
  - `Default` route state is changed or unknown
  - an explicit cached device is missing
  - opening a cached explicit device fails

## Explicit vs Default Selection

### Explicit microphone

- This is the most predictable low-latency path.
- A valid explicit device can reuse cached topology for up to 24 hours.
- If the cached explicit target fails to open, startup retries once with a full fresh
  live enumeration before surfacing failure.

### `Default`

- `Default` remains the flexible option because it follows macOS input changes.
- Cached topology is reused only when the route monitor confirms the captured
  generation still matches the current generation.
- If route monitoring is unavailable or route state is unknown, startup falls back to
  a fresh enumeration.
- Bluetooth avoidance remains intact: when `Default` would land on a Bluetooth mic,
  the app still prefers a better non-Bluetooth alternative when one exists.

## Lifecycle Source Matrix

| Source | Scope | Refresh path | Startup safety impact |
| --- | --- | --- | --- |
| App foreground | Cross-platform window focus | Opportunistic | Improves post-idle cache reuse |
| Wake | macOS native workspace notifications | Opportunistic | Improves post-sleep cache reuse |
| Audio-route change | Existing CoreAudio route monitor path | Opportunistic | Updates cached topology while default-route generation checks remain authoritative |
| Startup path | All platforms | Startup | Used only when cache reuse is unsafe or unavailable |

## Structured Logs

Inspect these log streams during verification:

### Refresh logs

- `event_code="topology_cache_refresh"`
- Key fields:
  - `refresh_source`
  - `refresh_path`
  - `outcome`
  - `duration_ms`
  - `device_count`
  - `route_generation`
  - `cache_updated`
  - `skip_reason`

### Startup logs

- `event_code="topology_resolution"`
- Key fields:
  - `resolution_mode`
  - `resolution_reason`
  - `fresh_reason`
  - `cache_refresh_source`
  - `cache_refresh_path`
  - `cache_age_ms`
  - `cache_route_generation`

## Verification Workflow

For each manual pass, capture:

1. the lifecycle trigger log
2. the refresh outcome log
3. the next startup-resolution log

Recommended passes:

- background to foreground, then push-to-talk
- sleep to wake, then push-to-talk
- route change while idle, then push-to-talk

Stable-setup expectations:

- explicit built-in/internal microphone after a long idle shorter than 24 hours should
  usually reuse cache
- `Default` after a long idle shorter than 24 hours should reuse cache only when route
  generation is confirmed unchanged
- route changes should force conservative fresh resolution for `Default`
