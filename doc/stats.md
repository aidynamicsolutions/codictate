# Home Statistics

The Home page displays aggregated statistics about the user's transcription activity. These stats are calculated from all transcription history entries.

## Stats Overview

| Stat | Description | Formula |
|------|-------------|---------|
| **Daily Streak** | Consecutive days with at least one transcription | Count of unbroken daily usage |
| **WPM** | Words per minute (average dictation speed) | `total_words / total_duration_minutes` |
| **Total Words** | Total words dictated across all transcriptions | Sum of word counts |
| **Time Saved** | Estimated time saved vs. typing | `(total_words / 40) - total_duration_minutes` |
| **Faster Than Typing %** | How much faster than average typing speed | `((wpm - 40) / 40) × 100` |

## Display Formats

### Daily Streak Card
- **Value**: `X days 🔥` (singular "day" for 1)
- **Subtext**: Tiered encouragement messages:
  | Streak Days | Message |
  |-------------|---------|
  | 0 | "Let's start a new streak!" |
  | 1-3 | "You're off to a great start!" |
  | 4-7 | "Keep the momentum going!" |
  | 8+ | "You're on fire!" |
- **Tooltip**: "Consecutive days with at least one transcription."

### WPM Card
- **Value**: `X 🏆`
- **Subtext**: 
  - Shows "X% faster than typing" when WPM > 40
  - Shows encouragement message ("Start speaking...") when WPM is 0
- **Tooltip**: "Words per minute (average dictation speed). Calculated as total words / total duration."

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
- Simple whitespace-based word splitting (`split_whitespace()`)

### WPM (Words Per Minute)
- Total words divided by total recording duration in minutes
- Threshold: Minimum duration of **0.06 seconds** (0.001 min) required to calculate WPM
- Returns `0.0` if no recordings exist or total duration is below threshold

### Time Saved
- Compares dictation time to estimated typing time
- Assumes average typing speed of **40 WPM**
- Can be negative if dictation is slower than typing

### Streak Days
- Counts consecutive days with at least one transcription
- Streak is considered **active** if user recorded today or yesterday
- Duplicate dates are removed before counting

### Faster Than Typing %
- Only calculated when WPM > 40
- Shows percentage improvement over average typing speed

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
    transcription_dates TEXT -- JSON array of active dates for streak calc
);
```

## Implementation

| Layer | File | Function |
|-------|------|----------|
| Backend | `src-tauri/src/managers/history.rs` | `get_home_stats()`, `save_transcription()` |
| Command | `src-tauri/src/commands/history.rs` | `get_home_stats`, `clear_all_history` |
| Frontend | `src/components/Home.tsx` | `loadData()`, helper functions |
| Translations | `src/i18n/locales/en/translation.json` | `home.stats.*`, `settings.history.*` |
