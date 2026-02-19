---
name: log
description: Analyze logs and add new logging to the Codictate application. Use when the user asks to "check logs", "debug", "find errors", "trace a session", or "add logs".
---

# Log Skill

This skill helps you analyze existing logs and add new logging to the Codictate application.

## 1. Analyze Logs

Logs are located at: `~/Library/Logs/com.pais.codictate/`

### Smart Search (Recommended)
Searches the latest log while filtering out common noise (like accessibility checks).

```bash
# Syntax: grep params... SEARCH_TERM $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1) | grep -v "Accessibility permission check"
grep -E "SEARCH_TERM" $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1) | grep -v "Accessibility permission check"
```

### Trace a Specific Session
To follow a single recording from start to finish:

1.  **Find a session ID** (e.g. from a transcription or error):
    ```bash
    grep "Transcription result:" $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)
    # Output example: ... session{session=d7cd03a7} ...
    ```

2.  **Filter by that ID**:
    ```bash
    grep "session=d7cd03a7" $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1) | grep -v "Accessibility permission check"
    ```

### Performance Analysis
Find slow operations (transcription, startup, etc.):
```bash
grep "completed in" $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)
```

### Reference: Common Patterns

| Context | Pattern | Command Example |
|---------|---------|-----------------|
| **Errors** | `ERROR` | `grep -E "ERROR" ...` |
| **Warnings** | `WARN` | `grep -E "WARN" ...` |
| **Transcriptions** | `Transcription result:` | `grep "Transcription result:" ...` |
| **Session** | `session=ID` | `grep -E "session=abc12345" ...` |
| **Frontend** | `target=fe` | `grep -E "target=fe" ...` |
| **Backend** | `codictate_app_lib` | `grep -E "codictate_app_lib" ...` |

---

## 2. Add Logs

Use these patterns to add structured logs that will appear in the unified log file.

### Rust (Backend)
File: `src-tauri/src/**/*.rs`

```rust
use tracing::{info, debug, error, warn};

// Standard logging
info!("Recording started");
debug!(samples = 1024, "Processing audio");
error!("Failed to load model: {}", err);

// With session context (if available)
info!(session = session_id, "Processing chunk");
```

### TypeScript (Frontend)
File: `src/**/*.tsx` or `src/**/*.ts`

```typescript
import { logInfo, logError, logDebug } from "@/utils/logging";

// Usage: logLevel(message, target_component)
logInfo("History updated", "fe-history");
logError(`Failed to load: ${error}`, "fe-updater");
```

### Python (MLX Sidecar)
File: `sidecar/**/*.py`

```python
logger.info("Model loaded", extra={'session': session_id})
logger.error(f"Inference failed: {e}")
```

### Swift (Native macOS FFI)
File: `src-tauri/swift/*.swift`

**Do NOT add logging in Swift.** Swift code is compiled as a static library and called via FFI from Rust. Return codes (-1/0/1) communicate status; Rust wrappers log outcomes.

If debugging Swift during development:
1. Use Xcode debugger with breakpoints
2. Temporarily add `print()` (remove before commit)
3. Check Rust logs for function outcomes
