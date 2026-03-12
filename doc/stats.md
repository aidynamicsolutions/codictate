# Home Statistics

The Home page displays aggregated statistics about the user's transcription activity. These stats are calculated from all transcription history entries.

## Stats Overview

| Stat | Description | Formula |
|------|-------------|---------|
| **Daily Streak** | Consecutive days with at least one transcription | Count of unbroken daily usage |
| **WPM** | Words per minute (average dictation speed) | `total_words / total_speech_duration_minutes` |
| **Total Words** | Total words dictated across all transcriptions | Sum of word counts |
| **Time Saved** | Estimated time saved vs. typing | `(total_words / 40) - total_recording_duration_minutes` |
| **Faster Than Typing %** | How much faster than average typing speed | `((wpm - 40) / 40) × 100` |
| **Filler Words Removed** | Total filler words (uh, um, etc.) automatically removed | Count of matched filler patterns |

## Display Formats

### Smart Stats Tile (Rotating)
This dynamic tile alternates between **Daily Streak** and **Filler Words Removed** to provide more insights in less space.

**Behavior:**
1. **Initial Load:** Shows Streak for ~2.5s, then animates to Filler Stats (if applicable).
2. **On Focus:** Re-triggers rotation sequence.
3. **Smart Locking:** Locks to **Streak face** (no rotation) if:
    - Filler word filter is disabled.
    - No filler words have been removed yet.
    - Streak is broken (0 days) — prioritizes encouragement.
    - Streak milestone reached (≥ 3 days) — prioritizes celebration.

**Streak Face:**
- **Value**: `X days 🔥` (singular "day" for 1)
- **Subtext**: Tiered encouragement messages (same as before).
- **Tooltip**: "Consecutive days with at least one transcription."

**Filler Words Face:**
- **Value**: `X ✨`
- **Subtext**: "X cleaner words" or "Your speech, polished <U+2728>"
- **Tooltip**: "Total filler words (uh, um, etc.) automatically removed from your transcriptions."

### WPM Card
- **Value**: `X.X 🏆`
- **Subtext**: 
  - Shows "X% faster than typing on average" when WPM > 40
  - Shows encouragement message ("Start speaking...") when WPM is 0
- **Tooltip**: "Lifetime average words per minute based on speech-active duration. This can move slightly up or down as new dictations are added."

### Total Words Card
- **Value**: `X 🚀` (formatted with locale separators)
- **Subtext**: Tiered word equivalents:
  | Word Count | Equivalent | Unit Size |
  |------------|------------|-----------|
  | 50+ | Tweets | ~50 words |
  | 800+ | Blog posts | ~800 words |
  | 2,000+ | Articles | ~2,000 words |
  | 7,500+ | Short stories | ~7,500 words |
  | 30,000+ | Novellas | ~30,000 words |
  | 80,000+ | Novels | ~80,000 words |
- **Tooltip**: "Total words dictated across all transcriptions."

### Time Saved Card
- **Value**: Smart time formatting:
  | Duration | Format | Example |
  |----------|--------|---------|
  | Duration | Format | Example | Animation |
  |----------|--------|---------|-----------|
  | < 1 min | X secs ⏱️ | "45 secs ⏱️" | Standard (2s) |
  | 1-59 mins | Xm Ys ⏱️ | "3m 10s ⏱️" | Standard (2s) |
  | 1-24 hours | Xh Ym ⏱️ | "2h 30m ⏱️" | Slow (3s) |
  | 24+ hours | Xd Yh Zm | "320d 2h 5m" | Slow (3s) |

- **Subtext**: Tiered time equivalents:
  | Minutes Saved | Message | Unit Size |
  |---------------|---------|-----------|
  | 0 | "Start dictating to save time!" | — |
  | 1-4 | "Every minute counts!" | — |
  | 5-7 | "Almost a coffee break!" | — |
  | 8+ | Showers | ~8 mins |
  | 10+ | Coffee breaks | ~10 mins |
  | 22+ | TV episodes | ~22 mins |
  | 25+ | Commutes | ~25 mins |
  | 120+ | Movies | ~120 mins |
  | 480+ | Workdays | ~480 mins |
- **Tooltip**: "Estimated time saved vs. typing at 40 WPM. Formula: (Total Words / 40) - Total Duration."

## Calculation Details

### Word Count
- Uses **post-processed text** if available, otherwise raw transcription text
- Unicode-aware segmentation (`unicode_words()`)

### WPM (Words Per Minute)
- Total words divided by total **speech-active** duration in minutes
- This is a **lifetime average**, so it is not monotonic and may go down after a slower-than-average dictation
- Threshold: Minimum duration of **0.06 seconds** (0.001 min) required to calculate WPM
- Returns `0.0` if no recordings exist or total duration is below threshold

### Time Saved
- Compares dictation **recording** time to estimated typing time
- Assumes average typing speed of **40 WPM**
- Can be negative if dictation is slower than typing

### Streak Days
- Counts consecutive days with at least one transcription
- Streak is considered **active** if user recorded today or yesterday
- Duplicate dates are removed before counting

### Faster Than Typing %
- Only calculated when WPM > 40
- Shows percentage improvement over average typing speed

### Filler Words
- Counts words matching filler patterns (e.g., "um", "uh", "like", "you know")
- Count matches exactly what is removed from the text output
- Only counts fillers when the **Filler Word Filter** is enabled

## Data Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Home.tsx      │────▶│  getHomeStats()  │────▶│  user_stats     │
│   (Frontend)    │     │  (Rust command)  │     │  (SQLite)       │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                        │
        │  listen("history-      │  emit("history-
        │  updated")             │  updated")
        ◀────────────────────────┘
```

### Live Updates
- Backend emits `history-updated` event after each transcription save
- Frontend listens for this event and refetches stats
- Updates also trigger on: clear all (though stats persist)

## Duration Semantics Migration

- New entries store both:
  - `duration_ms`: recording elapsed duration (wall-clock during hold-to-talk)
  - `speech_duration_ms`: VAD-retained unpadded speech duration
- WPM uses `total_speech_duration_ms`
- Time Saved uses `total_duration_ms`
- Existing installs run a one-time best-effort backfill that preserves lifetime totals while correcting duration semantics for available history rows.
- A one-time visible shift in WPM/Time Saved is expected after this migration.

## Stats Backup and Recovery

- Current backups export canonical aggregate stats in `history/user_stats.json`.
- Restore applies canonical stats when that payload is present and valid.
- Legacy backups (or malformed optional stats payloads) fall back to recomputing stats from history rows and emit recoverable warnings.
- Fallback recompute semantics:
  - word totals follow runtime save semantics (`post_processed_text` fallback to `transcription_text`, Unicode-aware word count)
  - speech duration falls back to recording duration when historical rows are missing speech values (`speech_duration_ms = 0`)

- Before duration backfill mutates stats, the app writes:
  - Filesystem snapshot: `app_data/stats-backups/user_stats-pre-duration-v1-<timestamp>.json`
  - Database snapshot row: `user_stats_migration_backup`
- If migration fails, transaction rollback preserves pre-mutation DB state and both backups remain available for recovery.

### One-Time Known Repair Signature (March 3, 2026)

For the known restore-regression signature:

- bad: `words=46208`, `duration=22697871`, `speech=3202691`, `semantics=1`
- target: `words=43604`, `duration=22720677`, `speech=22618064`, `semantics=1`

apply the guarded SQL runbook documented in [backup-restore.md](backup-restore.md).

## History Management

### Recording Retention
The default `recording_retention_period` is set to **`Never`** — entries are never auto-deleted. 
However, even if history IS deleted (auto or manual), **Stats persist forever** in the `user_stats` table.

Available retention options:
| Option | Behavior |
|--------|----------|
| `Never` | No auto-deletion (default) |
| `PreserveLimit` | Keep only the latest N entries |
| `Days3` | Delete entries older than 3 days |
| `Weeks2` | Delete entries older than 2 weeks |
| `Months3` | Delete entries older than 3 months |

### Clear All History
Users can manually clear all history via the **"Clear All History"** button in Settings → History:
- Shows confirmation dialog with entry count
- Deletes all transcription records and audio files
- **Does NOT reset lifetime stats** (Total Words, Duration, Streak, etc.)
- Triggers `history-updated` event

## Database Schema

Stats are maintained in a dedicated `user_stats` table (singleton row, ID=1):

```sql
CREATE TABLE user_stats (
    id INTEGER PRIMARY KEY DEFAULT 1,
    total_words INTEGER DEFAULT 0,
    total_duration_ms INTEGER DEFAULT 0,
    total_transcriptions INTEGER DEFAULT 0,
    first_transcription_date INTEGER,
    last_transcription_date INTEGER,
    transcription_dates TEXT, -- JSON array of active dates for streak calc
    total_filler_words_removed INTEGER DEFAULT 0
);
```

## Implementation

| Layer | File | Function |
|-------|------|----------|
| Backend | `src-tauri/src/managers/history.rs` | `get_home_stats()`, `save_transcription()` |
| Command | `src-tauri/src/commands/history.rs` | `get_home_stats`, `clear_all_history` |
| Frontend | `src/components/home/StatsOverview.tsx`, `SmartStatTile` | `renderSmartTile()` |
| Translations | `src/i18n/locales/en/translation.json` | `home.stats.*` (updated) |
