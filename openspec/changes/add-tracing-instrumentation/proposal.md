# Change: Add Unified Tracing for Development Debugging

## Why

Debugging the transcription pipeline is slow because:
1. Logs are fragmented across Rust, Python, and Frontend
2. No way to correlate logs from a single recording session
3. Only 1 log file retained (issues disappear on rotation)

## What Changes

- Add `tracing` crate for structured Rust logging with session spans
- Route Python sidecar logs to same file as Rust
- Add session_id to all logs for easy filtering
- Keep 7 log files for better retention

## Outcome

**Single file** with all logs:
```bash
# Debug a specific recording session
grep 'session=abc123' ~/Library/Logs/com.cjpais.handy/handy.log
```

## Impact

- Affected code: `lib.rs`, `transcription.rs`, `audio.rs`, `actions.rs`, `server.py`
- New dependencies: `tracing`, `tracing-subscriber`, `tracing-appender`, `uuid`
