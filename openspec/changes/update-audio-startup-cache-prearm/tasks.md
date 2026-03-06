## 1. Input Device Cache Strategy
- [x] 1.1 Increase input-device cache TTL from 5 seconds to 10 minutes
- [x] 1.2 Add cache dirty/in-flight/throttle state to avoid redundant scans
- [x] 1.3 Implement async cache refresh API with `IfStaleOrDirty` and `Force` policies
- [x] 1.4 Add cache dirty + forced refresh trigger on selected/clamshell microphone changes
- [x] 1.5 Add cache refresh trigger on main-window focus and fn-key down
- [x] 1.6 Add cache refresh trigger on generic shortcut/signal transcription starts

## 2. On-Demand Pre-Arm
- [x] 2.1 Add `kickoff_on_demand_prearm` API with source tagging
- [x] 2.2 Add pre-arm in-flight guard and stream lifecycle serialization lock shared by startup prewarm/start/stop paths
- [x] 2.3 Add 900ms pre-arm grace timeout with safe auto-close when no recording commit
- [x] 2.4 Invoke pre-arm from fn-key press flow and generic transcribe start flow
- [x] 2.5 Gate early Fn pre-arm behind maintenance-mode checks to avoid blocked-start mic side effects
- [x] 2.6 Add ownership guard so pre-arm timeout cleanup only closes streams opened by pre-arm
- [x] 2.7 Route Bluetooth startup prewarm through serialized stream lifecycle APIs with ownership-safe auto-close
- [x] 2.8 Add stream-epoch + pre-arm owner-token guards so stale workers cannot close newer stream instances

## 3. Observability
- [x] 3.1 Emit structured cache lifecycle logs (`cache_hit`, `cache_stale`, `cache_dirty`, refresh events)
- [x] 3.2 Emit structured pre-arm lifecycle logs (`prearm_started`, `prearm_stream_ready`, `prearm_timeout_autoclose`, skip/cancel paths)

## 4. Validation
- [x] 4.1 Add/extend unit tests for cache decisions and ownership-aware pre-arm auto-close logic
- [x] 4.1a Add regression coverage for epoch mismatch ownership checks in pre-arm/prewarm auto-close predicates
- [x] 4.2 Run `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 4.3 Run `cargo test --manifest-path src-tauri/Cargo.toml managers::audio::tests -- --nocapture`

## 5. Route Change Safety + Spec Deltas
- [x] 5.1 Add native input-route change monitoring and force-refresh cache bypass for default-route starts when needed
- [x] 5.1a Replace consume/reset route-change counters with monotonic generation tracking so concurrent starts cannot clear each other's route-change signal before refresh is applied
- [x] 5.2 Prevent startup prewarm stream opens from persisting fallback microphone auto-switch settings
- [x] 5.3 Add OpenSpec delta files for `shortcut-settings` and `observability`
