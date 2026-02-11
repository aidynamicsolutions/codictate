# Change: Add Sentry Error Monitoring

## Why
Currently, the application lacks a centralized way to track errors and crashes in production. This makes it difficult to proactively identify and fix issues that users encounter. We need a robust solution that works across both the Rust backend and the frontend while respecting user privacy.

## What Changes
- Add `tauri-plugin-sentry` to capture Rust panics and frontend errors.
- Configure Sentry with a DSN provided at build time or runtime.
- Implement strictly privacy-preserving defaults (no PII, local scrubbing).
- Add offline caching for error reports using Sentry's browser transport.
- Update documentation to include Sentry setup and prerequisite keys.

## Impact
- **Specs**: updates `observability` spec.
- **Code**:
    - `src-tauri/Cargo.toml`: Add dependencies.
    - `src-tauri/src/lib.rs`: Initialize Sentry plugin.
    - `src/App.tsx` (or main entry): Initialize/configure frontend Sentry if custom config is needed beyond plugin defaults.
    - `src-tauri/tauri.conf.json`: permissions.
