# History Feature

The History feature in Codictate provides a secure, local archive of all your dictations, optimized for handling large datasets (tested up to 100,000+ entries) without performance degradation.

## Storage
- **Local Only:** All data (audio files and transcriptions) is stored locally on your device. Nothing is uploaded to the cloud.
- **Location:**
  - **Audio:** Saved as `.wav` files in the `recordings` folder within the app data directory.
  - **Metadata:** Transcriptions, timestamps, and durations are stored in a local SQLite database (`history.db`).
- **Capacity:** The system supports efficient storage and retrieval of millions of entries.

## Performance Architecture
To ensure a smooth user experience even with massive history logs, the feature implements:
- **Virtualization:** The specific list component (`HistoryList`) uses `react-virtuoso` to render only the visible items in the DOM.
- **Backend Pagination:** Data is fetched in chunks (pages) from the SQLite database using `LIMIT` and `OFFSET`, preventing memory bloat.
- **Server-Side Search:** Search operations are performed directly via SQL queries (`LIKE`) on the backend for speed and efficiency.
- **Synchronous Asset Resolution:** Audio file paths are resolved on the backend, allowing the frontend to generate asset URLs synchronously (`convertFileSrc`), eliminating async waterfalls during scrolling.

## Features
- **Playback:** Listen to the original audio recording for any entry.
- **Transcription:** View and copy the transcribed text.
- **Search:** Instantly filter history by transcription content.
- **Organize:**
  - **Timeline:** Entries are grouped by date.
  - **Star:** Mark important recordings as "Saved" to prevent accidental deletion during pruning.
- **Filter:** Narrow results via a dropdown integrated with the search bar (see below).

## Filtering

A unified dropdown sits inline with the search input, offering:

| Option | Backend Behavior |
|---|---|
| All Time (default) | No filter applied |
| ⭐ Starred | `WHERE saved = 1` |
| Today | `WHERE timestamp >= <start of today>` |
| This Week | `WHERE timestamp >= <Monday of current week>` |
| This Month | `WHERE timestamp >= <1st of current month>` |
| This Year | `WHERE timestamp >= <Jan 1 of current year>` |

**Key details:**
- **Server-side filtering:** Filters are SQL `WHERE` clauses, composing with search via `AND`. This ensures pagination stays correct regardless of filter.
- **Type:** `HistoryFilter` — a single union type (`"all" | "starred" | "today" | "this_week" | "this_month" | "this_year"`) managed in `useHistory.ts`.
- **Timezone:** Cutoff timestamps are computed client-side using local timezone (`new Date()`), then passed as Unix seconds to the backend.
- **Optimistic unstar:** When a user unstars an entry while the "Starred" filter is active, the entry is immediately removed from the list.
- **Clear filter:** An `X` button appears next to the dropdown when any filter is active.

## Storage Management
To manage disk space, navigate to **Settings > History**:
- **Storage Usage:** View real-time disk usage and total recording count.
- **Pruning:** Bulk delete unsaved recordings older than specific thresholds (3 days, 1 week, etc.).
  *Note: "Saved" (starred) entries are never automatically pruned.*
- **Clear All:** Permanently delete all history and audio files.
