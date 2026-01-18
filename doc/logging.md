# Unified Logging & Tracing

Handy uses a unified tracing system where all components (Rust, Python, Frontend) write to a single log file.

## Quick Start

```bash
# Watch logs in real-time
bun run logs

# Search logs for patterns
bun run logs:grep "ERROR"
bun run logs:grep "session=abc12345"
```

## Log Location

| Platform | Path |
|----------|------|
| macOS | `~/Library/Logs/com.pais.codictate/codictate.YYYY-MM-DD.log` |
| Windows | `%APPDATA%\com.pais.codictate\logs\codictate.YYYY-MM-DD.log` |
| Linux | `~/.local/share/com.pais.codictate/logs/codictate.YYYY-MM-DD.log` |

## Log Format

```
TIMESTAMP LEVEL TARGET: MESSAGE session="ID" target="COMPONENT"
```

Example:
```
2026-01-09T16:05:07Z INFO codictate_app_lib::tracing_config: Session started session="27d68929" target="fe"
2026-01-09T16:05:25Z DEBUG codictate_app_lib::managers::mlx: Loading model...
```

## Component Targets

| Target | Component |
|--------|-----------|
| `codictate_app_lib::*` | Rust backend |
| `fe` | Frontend (default) |
| `fe-history` | Frontend history UI |
| `fe-updater` | Frontend update checker |
| `mlx-sidecar` | Python MLX inference server |

## Session Correlation

Each recording gets a unique 8-character session ID. Filter by session to trace a single recording flow:

```bash
bun run logs:grep "session=27d68929"
```

## Adding Logs

### Rust (Backend)
```rust
use tracing::{info, debug, error, warn};

info!("Recording started");
debug!(samples = 1024, "Processing audio");
error!("Failed to load model: {}", err);
```

### TypeScript (Frontend)
```typescript
import { logInfo, logError, logDebug } from "@/utils/logging";

logInfo("History updated", "fe-history");
logError(`Failed to load: ${error}`, "fe-updater");
```

### Python (MLX Sidecar)
```python
logger.info("Model loaded", extra={'session': session_id})
```

## Log Levels

| Level | Use |
|-------|-----|
| `ERROR` | Failures requiring attention |
| `WARN` | Potential issues |
| `INFO` | Key events (recording start/stop) |
| `DEBUG` | Detailed debugging |
| `TRACE` | Verbose tracing |

## Dynamic Log Level (Developer Only)

The file log level can be changed at runtime:

- **Console**: Set `RUST_LOG` env var before starting
- **File**: Defaults to DEBUG level

This is a developer feature — no user-facing UI exists.

## Dev Run Modes

```bash
bun run tauri:dev          # INFO level (recommended)
bun run tauri:dev:debug    # DEBUG level, noisy deps suppressed
bun run tauri:dev:verbose  # TRACE level (very noisy)
```

## Log Rotation

- **Rotation**: Daily (new file each day)
- **Retention**: 7 days (auto-deleted)
- **Non-blocking**: Writes don't slow the UI

## Common Debug Patterns

```bash
# Recording issues
bun run logs:grep "Recording|vad"

# Transcription issues
bun run logs:grep "Transcription|model"

# Post-processing issues
bun run logs:grep "mlx-sidecar|post.?process"

# All errors
bun run logs:grep "ERROR"

# Frontend only
bun run logs:grep "target=fe"
```

## Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Frontend  │    │    Rust     │    │   Python    │
│  (React)    │    │   Backend   │    │   Sidecar   │
└──────┬──────┘    └──────┬──────┘    └──────┬──────┘
       │                  │                  │
       │ invoke()         │                  │ stdout
       └─────────────────►│◄─────────────────┘
                          │
                          ▼
                  codictate.YYYY-MM-DD.log
```
