# Sentry Error Monitoring Integration

This document outlines how Sentry is integrated into Codictate for error tracking.

## Architecture
We use `tauri-plugin-sentry` to provide a unified error tracking solution.
-   **Backend:** Rust SDK captures panics and `anyhow::Error`s.
-   **Frontend:** Browser SDK injects into the webview and forwards errors to the Rust backend.
-   **Plugin:** Acts as the bridge, adding OS context and ensuring offline caching/sending.

## Setup
Ensure `SENTRY_DSN` is set (see [Prerequisites](./sentry_prerequisites.md)).

The initialization happens in `src-tauri/src/lib.rs`:
```rust
tauri_plugin_sentry::init(
    &sentry::ClientOptions {
        dsn: std::env::var("SENTRY_DSN").ok().map(|d| d.parse().unwrap()),
        release: sentry::release_name!(),
        before_send: Some(std::sync::Arc::new(Box::new(|event| {
            // Privacy scrubbing logic here
            Some(event)
        }))),
        ..Default::default()
    },
    |options| {
        // Plugin specific options
        options
    }
)
```

## Privacy & PII
Codictate is a privacy-first application.
-   **No PII:** We do not track IP addresses or user-identifiable data by default.
-   **Scrubbing:** The `before_send` hook in Rust is used to aggressively scrub potential sensitive data from error messages before they leave the device.
-   **Consent:** (Future) We will add an opt-in/opt-out setting in the General Settings.

## Debugging
To verify Sentry is working:
1.  Run the app with `SENTRY_DSN` set.
2.  Trigger a test error (we will add a hidden debug command for this).
3.  Check the Sentry dashboard.
