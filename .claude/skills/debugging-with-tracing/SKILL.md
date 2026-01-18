---
name: debugging-with-tracing
description: Debug Handy using unified tracing logs. Use when investigating recording, transcription, or post-processing issues. Shows how to find logs and filter by session ID.
---

# Debugging with Tracing

Handy uses a unified tracing system that logs to a single file for easy debugging.
All components (Rust backend, Python MLX sidecar, React frontend) write to the same log file.

## Quick Start: Log Viewing Scripts

```bash
# Watch logs in real-time (live tail)
bun run logs

# Search for specific patterns in logs
bun run logs:grep "ERROR"

# Filter by session ID (see "Finding Session IDs" below)
bun run logs:grep "session=abc12345"
```

### When to Use Each Script

| Script | Use Case |
|--------|----------|
| `bun run logs` | **Active debugging** — watching logs as you interact with the app. Use when reproducing a bug or monitoring behavior in real-time. Press `Ctrl+C` to stop. |
| `bun run logs:grep "pattern"` | **Post-mortem analysis** — searching past logs for specific patterns. Use when looking for errors, filtering by session ID, or finding specific events after they happened. |

## Log File Location

```bash
# macOS
~/Library/Logs/com.pais.codictate/codictate.log

# Windows
%APPDATA%\com.pais.codictate\logs\codictate.log

# Linux
~/.local/share/com.pais.codictate/logs/codictate.log
```

## Log Targets (Component Prefixes)

Each log line includes a `target` field identifying the source component:

| Target Pattern | Component |
|----------------|-----------|
| `codictate_app_lib::*` | Rust backend (e.g., `codictate_app_lib::managers::audio`) |
| `fe` | Frontend (default) |
| `fe-history` | Frontend history component |
| `fe-updater` | Frontend update checker |
| `mlx-sidecar` | Python MLX inference server |

### Filtering by Component

```bash
# All frontend logs
bun run logs:grep "target=fe"

# All backend audio logs
bun run logs:grep "codictate_app_lib::managers::audio"

# All MLX sidecar logs
bun run logs:grep "target=mlx-sidecar"
```

## Instructions

1. Run the app with one of these commands:
   ```bash
   bun run tauri:dev          # Handy logs only (quietest, recommended)
   bun run tauri:dev:debug    # Handy DEBUG + suppress noisy deps  
   bun run tauri:dev:verbose  # All logs including deps (noisiest)
   ```
2. In a separate terminal, run `bun run logs` to watch logs
3. Perform a recording to generate logs
4. Use session IDs to correlate logs across components

## Finding Session IDs

Session IDs are **8-character hex codes** generated when a recording starts. To find a session ID:

**Step 1: Look at the logs** — Session IDs appear when recordings start:
```
2026-01-09T16:05:07Z INFO ... Session started session="27d68929" target="fe"
```

**Step 2: Copy the session ID** — In the example above, the session ID is `27d68929`

**Step 3: Filter by that session** — Now trace all logs for that recording:
```bash
bun run logs:grep "session=27d68929"
```

This shows all logs (Rust, Python, Frontend) for that single recording flow.

## Session Correlation

Every recording session is assigned a unique session ID (8-character UUID prefix).
All logs from Rust, Python, and Frontend include this session ID.

### Filtering Logs by Session

```bash
# Find all logs for a session (replace abc12345 with actual ID from logs)
bun run logs:grep "session=abc12345"

# Find errors only for a session
bun run logs:grep "ERROR.*session=abc12345"

# Watch for session logs in real-time
bun run logs | grep 'session='
```

## Common Debug Patterns

### Recording Issues
```bash
bun run logs:grep "Recording (started|stopped)"
bun run logs:grep "vad"  # Voice Activity Detection
```

### Transcription Issues
```bash
bun run logs:grep "Transcription completed"
bun run logs:grep "model.*load|load.*model"
```

### Post-Processing Issues
```bash
bun run logs:grep "post.?process|MLX|Apple Intelligence"
bun run logs:grep "target=mlx-sidecar"
```

### Frontend Issues
```bash
bun run logs:grep "target=fe"
```

## Log Retention

Logs are rotated daily and kept for 7 days.

## Log Levels

| Level | Use |
|-------|-----|
| ERROR | Problems that need attention |
| WARN | Potential issues |
| INFO | Key events (recording start/stop, results) |
| DEBUG | Detailed debugging info |
| TRACE | Verbose tracing |

### Dynamic Log Level (Developer Only)

The file log level can be changed at runtime for debugging:

- **Console output**: Set `RUST_LOG` env var (e.g., `RUST_LOG=debug bun run tauri dev`)
- **File output**: Defaults to DEBUG, changes apply immediately via internal `set_log_level` command
- **Default: DEBUG** — captures detailed info for debugging

This is a developer feature — there is no user-facing Settings UI for log level.
