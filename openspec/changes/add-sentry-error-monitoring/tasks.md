## 1. Implementation
- [ ] 1.1 Add `sentry` and `tauri-plugin-sentry` to `src-tauri/Cargo.toml`
- [ ] 1.2 Initialize Sentry in `src-tauri/src/lib.rs` with DSN and `before_send` hook for PII scrubbing
- [ ] 1.3 Configure `tauri-plugin-sentry` in `src-tauri/tauri.conf.json`
- [ ] 1.4 Add `makeBrowserOfflineTransport` configuration in frontend initialization (if custom init is used) or verify plugin's offline handling
- [ ] 1.5 Add `SENTRY_DSN` handling (env var or config)
- [ ] 1.6 Verify Sentry capturing Rust panic
- [ ] 1.7 Verify Sentry capturing Frontend exception
- [ ] 1.8 Verify PII scrubbing (e.g., ensure no personal data in event payload)
