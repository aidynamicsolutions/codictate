# Aptabase Analytics Integration

This document outlines how Aptabase is integrated into Handy for anonymous usage tracking.

## Architecture
-   **Plugin:** `tauri-plugin-aptabase` handles the communication with Aptabase servers.
-   **Offline Handling:** A custom wrapper in `src/utils/analytics.ts` uses `tauri-plugin-store` to queue events when the device is offline and flushes them when functionality is restored.
-   **Privacy:** All data is anonymous. We use Aptabase's default privacy-first settings.

## Setup
Ensure `APTABASE_APP_KEY` is set (see [Prerequisites](./aptabase_prerequisites.md)).

The initialization happens in `src-tauri/src/lib.rs`:
```rust
tauri_plugin_aptabase::Builder::new(
    std::env::var("APTABASE_APP_KEY").expect("APTABASE_APP_KEY must be set")
).build();
```

## Usage
In the frontend, import the analytics wrapper:
```typescript
import { trackEvent } from '@/utils/analytics';

// Simple event
trackEvent('app_started');

// Event with properties
trackEvent('transcription_completed', {
    duration_sec: 120,
    language: 'en'
});
```

## Privacy & Compliance
-   **GDPR/CCPA:** Aptabase is compliant by design. No cookie banner is required for this specific analytics usage as no PII is collected and no persistent trackers are used.
-   **Data Retention:** Data is retained for strict windows and then discarded/aggregated.
-   **Opt-Out:** (Future) We will add a "Share usage statistics" toggle in Settings.
