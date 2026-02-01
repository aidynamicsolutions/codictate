# History Feature

The History feature in Codictate provides a secure, local archive of all your dictations.

## Storage
- **Local Only:** All data (audio files and transcriptions) is stored locally on your device. Nothing is uploaded to the cloud.
- **Location:**
  - **Audio:** Saved as `.wav` files in the `recordings` folder within the app data directory.
  - **Metadata:** Transcriptions, timestamps, and durations are stored in a local SQLite database (`history.db`).
- **Capacity:** The system supports efficient storage of up to 1,000,000 entries.

## Features
- **Playback:** Listen to the original audio recording for any entry.
- **Transcription:** View and copy the transcribed text.
- **Search & Organize:**
  - **Timeline:** Entries are grouped by date.
  - **Star:** Mark important recordings as "Saved" to prevent accidental deletion during pruning.

## Storage Management
To manage disk space, navigate to **Settings > History**:
- **Storage Usage:** View real-time disk usage and total recording count.
- **Pruning:** bulk delete unsaved recordings older than:
  - 3 days
  - 7 days
  - 30 days
  - 3 months
  - 1 year
  *Note: "Saved" (starred) entries are never automatically pruned.*
- **Clear All:** Permanently delete all history and audio files.
