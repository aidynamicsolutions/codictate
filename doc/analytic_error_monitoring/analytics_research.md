# Analytics & Error Tracking Research Report

## Executive Summary
For a privacy-focused, open-source Tauri application like Codictate, we recommend a split-stack approach:
-   **Error Tracking:** **Sentry** (Industry standard, robust Rust/JS support).
-   **Usage Analytics:** **Aptabase** (Privacy-first, anonymous, lightweight).

This combination ensures you get deep debugging capabilities when things go wrong, while maintaining strict user privacy for general usage statistics.

---

## 1. Error Tracking: Sentry
[Sentry](https://sentry.io/welcome/) is the gold standard for error tracking. It is essential for "knowing if the user encounters errors so I can help them fix."

### Why Sentry?
-   **Full Stack Coverage:** Captures errors from both the **Rust** backend (panics, `Result::Err`) and the **TypeScript** frontend (React error boundaries, unhandled promise rejections).
-   **Context:** Provides strict stack traces, breadcrumbs (actions leading up to error), and device info (OS version, app version).
-   **Tauri Support:** The `tauri-plugin-sentry` (v2) enables seamless integration.

### Privacy Considerations
-   Sentry allows data scrubbing (removing PII like IPs, usernames).
-   You can host it yourself (via generic Sentry protocols) or use their cloud tier.
-   Be sure to configure `beforeSend` callbacks to strip any sensitive data from the payload.

### Implementation Strategy
1.  Add `tauri-plugin-sentry` to `src-tauri/Cargo.toml`.
2.  Initialize Sentry in `src-tauri/src/lib.rs` (early in startup to catch crash-on-launch issues).
3.  Wraps frontend with Sentry's React SDK for better component stack traces.

---

## 2. Anonymous Usage Analytics: Aptabase
[Aptabase](https://aptabase.com/) is a privacy-first analytics platform built specifically for mobile and desktop apps (Swift, Kotlin, Tauri).

### Deep Dive Validation: Why Aptabase?

#### Developer Experience (Dev UX)
-   **Simple Instrumentation:** The `tauri-plugin-aptabase` is official and well-maintained. Tracking an event is as simple as `track_event("app_started", None)`.
-   **Minimal Friction:** Unlike Google Analytics or Mixpanel which require complex session management and user identification setups, Aptabase works out of the box for "fire and forget" event tracking.
-   **Tauri Native:** It is built by developers in the Tauri ecosystem. The dashboard provides exactly the metrics a desktop app developer needs: App Version adoption (crucial for auto-update tracking) and OS distribution.

#### Privacy Mechanics
-   **No Fingerprinting:** It does not use cookies, local storage identifiers, or persistent device IDs.
-   **Session Isolation:** it uses a daily-rotating salt to hash the IP + User Agent. This means you can track a "session" within a day, but you cannot link a user's activity from Day 1 to Day 2. This mathematically guarantees anonymity over time.
-   **Compliance:** GDPR, CCPA, and PECR compliant by design (no PII collected, no consent banner needed typically).

#### Comparison vs Alternatives
-   **vs Plausible:** Plausible is excellent but web-focused. It lacks a native Tauri plugin for capturing desktop-specific context (like App Version).
-   **vs PostHog:** PostHog is powerful but heavier. It includes feature flags and session replays which might be overkill and introduce privacy complexity. Aptabase is stricter on privacy (no session replay).

### Implementation Strategy
1.  Add `tauri-plugin-aptabase` to `src-tauri/Cargo.toml`.
2.  Add `use_aptabase` to `ActivePlugins` in `lib.rs`.
3.  Instrument key actions (e.g., `track_event("transcription_complete", props)`).

---

## 3. Implementation Plan

To proceed with implementation, you will need to:
1.  **Sign up for Sentry:** Create a project and get a **DSN** (Data Source Name).
2.  **Sign up for Aptabase:** Create a project and get an **App Key**.

### Proposed Changes

#### Backend (`src-tauri/Cargo.toml`)
-   Add `tauri-plugin-sentry`
-   Add `tauri-plugin-aptabase`

#### Backend (`src-tauri/src/lib.rs`)
-   Initialize Sentry integration.
-   Initialize Aptabase plugin.
-   Add a `track_event` command (or use the plugin's direct command) to allow frontend usage.

#### Frontend
-   Add generic error boundary to catch React render errors.
-   Add tracking hooks to key events:
    -   App Launch (handled automatically by Aptabase)
    -   Transcription Success/Failure
    -   Model Download
    -   Settings Changes (anonymous aggregations, e.g., "Language Switched")

## Next Steps
If you approve this plan, please providing the **Sentry DSN** and **Aptabase App Key**.
(Alternatively, I can mock these keys for now, and you can replace them in `.env` or `tauri.conf.json` later).
