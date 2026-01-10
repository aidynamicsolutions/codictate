# Design: Unified Tracing Instrumentation

## Context

**Goal**: Single log file for Rust, Python, and Frontend that you can read during development for faster debugging.

## Log Format: Logfmt (Human-Readable Structured)

We use **Logfmt** format — the sweet spot between JSON (too verbose) and plain text (no structure):

```
2026-01-09T14:20:00+07:00 INFO session=abc123 target=transcription duration_ms=890 msg="Transcription completed"
2026-01-09T14:20:01+07:00 ERROR session=abc123 target=audio msg="Failed to open device" error="Device not found"
```

**Why Logfmt:**
- ✅ Human-readable at a glance (no `{` `}` clutter)
- ✅ Grepable: `grep 'session=abc123'` works perfectly
- ✅ Structured: Key-value pairs for filtering
- ✅ Compact: One line per event

### Required Fields

| Field | Example | Purpose |
|-------|---------|---------|
| **timestamp** | `2026-01-09T14:20:00+07:00` | ISO 8601, chronological order |
| **level** | `INFO`, `DEBUG`, `ERROR` | Severity filtering |
| **session** | `abc123` | Correlate logs for one recording |
| **target** | `audio`, `transcription`, `python` | Which component |
| **msg** | `"Recording started"` | Human description |

### Optional Fields

| Field | When to use |
|-------|-------------|
| **duration_ms** | Performance timing |
| **error** | Error details |
| **samples** | Audio sample count |

## Architecture: Single Unified Log File

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Frontend  │    │    Rust     │    │   Python    │
│  (React)    │    │   Backend   │    │   Sidecar   │
└──────┬──────┘    └──────┬──────┘    └──────┬──────┘
       │                  │                  │
       └──────────────────┼──────────────────┘
                          ▼
                ┌─────────────────┐
                │   handy.log     │  ← Single file
                │  (with rotation)│
                └─────────────────┘
```

### How it works:

1. **Rust**: Writes directly to log file via `tracing-appender` using native `tracing::` macros
2. **Python**: Logs to stdout, Rust captures sidecar stdout and writes to same file
3. **Frontend**: Uses browser console (session_id correlation is a future enhancement)

All logs include `session_id` for filtering.

## Session ID Flow

```
Recording Start (Frontend)
    │
    ▼ invoke("start_recording", { session_id: "abc123" })
    │
    ▼ Rust creates span: transcription_session{session_id="abc123"}
    │
    ├── Audio recording logs
    ├── VAD processing logs
    │
    ▼ HTTP to Python: X-Session-Id: abc123
    │
    ├── Python logs with session_id
    │
    ▼ Transcription logs
    │
    ▼ Post-processing logs
    │
    ▼ Paste result logs
```

## Rust Configuration

```rust
tracing_subscriber::fmt()
    .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
    .with_target(true)
    .with_level(true)
    .with_ansi(false)  // No color codes in file
    .init();
```

## Debug Workflow

```bash
# All logs for a specific session
grep 'session=abc123' ~/Library/Logs/com.cjpais.handy/handy.log

# Just errors
grep 'ERROR' handy.log

# Transcription timings
grep 'duration_ms' handy.log
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tracing` | Macros (native, no bridge needed) |
| `tracing-subscriber` | Logfmt output |
| `tracing-appender` | File rotation (7 days) |
| `uuid` | Session IDs |

> **Note**: We fully migrated all Rust code to native `tracing::` macros, eliminating the need for `tracing-log` bridge and `log` crate.
