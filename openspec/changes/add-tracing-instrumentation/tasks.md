# Tasks: Add Unified Tracing Instrumentation

## Phase 1: Core Tracing (Essential)

### 1. Rust Dependencies
- [x] 1.1 Add `tracing` and `tracing-subscriber` to Cargo.toml
- [x] 1.2 Add `tracing-appender` for file output with rotation
- [x] 1.3 ~~Add `tracing-log` to capture `log` crate messages from dependencies~~ (Removed - fully migrated to native tracing)
- [x] 1.4 Add `uuid` crate for session ID generation
- [x] 1.5 Remove `log` crate dependency (migrated all code to native `tracing::` macros)
- [x] 1.6 Remove `tauri-plugin-log` dependency (no longer needed)

### 2. Initialize Tracing Subscriber
- [x] 2.1 Configure tracing-subscriber with Logfmt format (timestamp, level, target, msg)
- [x] 2.2 Set up dual output: stdout (with color) + file (no color)
- [x] 2.3 Set file rotation to keep 7 log files
- [x] 2.4 ~~Add `tracing_log::LogTracer` to capture `log` crate messages~~ (Removed - fully migrated)

### 3. Add Session Correlation
- [x] 3.1 Generate session UUID when recording starts
- [x] 3.2 Create session span that wraps the entire recording → transcription cycle
- [x] 3.3 Ensure all logs within a session include the session field

### 4. Migrate All Rust Code to Native Tracing
- [x] 4.1 Replace `log` macros in `transcription.rs` with `tracing` macros
- [x] 4.2 Replace `log` macros in `audio.rs` with `tracing` macros
- [x] 4.3 Replace `log` macros in `actions.rs` with `tracing` macros
- [x] 4.4 Add `#[instrument]` to `TranscriptionManager::transcribe()`
- [x] 4.5 Migrate all remaining files from `log::` to `tracing::` (20 files total)

## Phase 2: Unified Log File (Cross-Component)

### 5. Python Sidecar → Same Log File
- [x] 5.1 Configure Python to log to stdout in Logfmt format
- [x] 5.2 Accept `X-Session-Id` header from Rust
- [x] 5.3 Include session= field in Python log output

### 6. Frontend Logging
- [x] 6.1 Create frontend logging utility that routes to Rust backend
- [x] 6.2 Include session_id in frontend log calls via `logging.ts`

## Phase 3: Documentation

### 7. Documentation & Skill
- [x] 7.1 Read skills documentation at https://code.claude.com/docs/en/skills.md
- [x] 7.2 Create debugging skill at `.claude/skills/debugging-with-tracing/SKILL.md`

## Phase 4: Verification

### 8. Verification
- [x] 8.1 Run `cargo build` to verify compilation
- [ ] 8.2 Perform a recording and check unified log file
- [x] 8.3 Verify Logfmt output: `TIMESTAMP LEVEL session=ID target=COMPONENT msg="..."`
- [x] 8.4 Filter logs by session: `grep 'session=abc123' handy.log`
- [x] 8.5 Verify Python and Frontend logs appear in same file with session correlation

