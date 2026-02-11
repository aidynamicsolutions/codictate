# Change: Add Aptabase Analytics

## Why
To make informed decisions about product development, we need to understand how users are interacting with Handy. Aptabase provides a privacy-first, anonymous way to gather usage insights (e.g., active versions, feature usage) without compromising user privacy or collecting PII.

## What Changes
- Add `tauri-plugin-aptabase` to the project.
- Implement an **Offline Event Queue** using `tauri-plugin-store` to persist events when the device is offline and flush them when online.
- Instrument key lifecycle events (`app_started`, `app_exited`) and feature usage events (e.g., `transcription_completed`).
- Update documentation with Aptabase setup instructions and privacy details.

## Impact
- **Specs**: updates `observability` spec.
- **Code**:
    - `src-tauri/Cargo.toml`: Add dependencies.
    - `src-tauri/src/lib.rs`: Initialize Aptabase plugin.
    - `src/utils/analytics.ts` (New): valid wrapper for tracking events with offline handling logic.
    - `src-tauri/capabilities/default.json`: Allow `store` and `aptabase` permissions.
