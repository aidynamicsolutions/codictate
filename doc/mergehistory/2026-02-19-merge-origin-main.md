# Merge origin/main → llm (2026-02-19)

## Commits from `origin/main`

| SHA | Description | Decision |
|-----|-------------|----------|
| `3c0fb95` | fix: drain audio buffer before recording | **Take** – auto-merged cleanly |
| `cbc8080` | chore: update nix hash | **Take** – auto-merged cleanly |
| `58b95c5` | chore: release v0.7.6 | **Partial take** – version bump to 0.7.6, kept Codictate branding |
| `133c50c` | feat: CLI parameters for Linux | **Partial take** – added `cli.rs`, `--start-hidden`, `--no-tray`, `--debug` |
| `0cb8ab2` | feat: structured outputs for LLM post-processing | **Take** – JSON schema via `send_chat_completion_with_schema`, `TRANSCRIPTION_FIELD` |
| `83e6f5c` | feat: add Moonshine V2 streaming models | **Take** – tiny/small/medium variants, kept Parakeet V3 as default |
| `203ba1d` | feat: SIGUSR1/SIGUSR2 signal handling for Linux | **Take** – `signal_handle.rs` routes through coordinator |
| `7fdde63` | fix: transcription lock-up via TranscriptionCoordinator | **Partial take** – ported coordinator + FinishGuard, kept our shortcut routing |
| `dc20774` | fix: add back language selection | **Take** – auto-merged cleanly |
| `e2a8008` | fix: push lock handling | **Take** – auto-merged cleanly |

## Conflict Resolution

| File | Strategy |
|------|----------|
| `actions.rs` | **Keep ours** + added `FinishGuard`, `build_system_prompt`, `strip_invisible_chars`, structured output logic, `TRANSCRIPTION_FIELD` |
| `lib.rs` | **Hybrid** – kept our managers/imports + added `TranscriptionCoordinator`, `CliArgs`, tray icon settings |
| `main.rs` | **Hybrid** – kept `codictate_app_lib` + added `CliArgs::parse()` |
| `transcription.rs` | **Hybrid** – kept our session management + added `catch_unwind`, `lock_engine()`, `MoonshineStreaming` variant |
| `utils.rs` | **Keep ours** + added coordinator `notify_cancel` in `cancel_current_operation` |
| `signal_handle.rs` | **Took theirs** – coordinator-based signal routing (Linux only) |
| `Cargo.toml` | **Hybrid** – kept Codictate branding, added `clap`, `signal-hook`, updated to v0.7.6 |
| `tauri.conf.json` | **Hybrid** – kept Codictate branding, updated to v0.7.6 |
| `Cargo.lock` | **Regenerated** |
| `bindings.ts` | **Hybrid** – kept our types + added new commands/types from main |
| 12 i18n translation files | **Hybrid** – kept ours + added Moonshine V2 model keys, tray icon/unload strings |

### New Files

| File | Source |
|------|--------|
| `cli.rs` | From main – CLI argument parsing via `clap` |
| `transcription_coordinator.rs` | From main – transcription lifecycle serialization |

## Architecture Decision: Two Input Routing Systems

This merge introduces a second transcription input routing system from main. Both coexist by design.

| | `ManagedToggleState` (ours) | `TranscriptionCoordinator` (main) |
|---|---|---|
| Purpose | Toggle state tracking for shortcuts | Serialize concurrent signal inputs |
| Used by | `shortcut/mod.rs`, `fn_key_monitor.rs` | `signal_handle.rs` (Linux SIGUSR) |
| Platform | macOS | Linux |
| Mechanism | `HashMap<String, bool>` | MPSC channel + background thread + state machine |

**Rationale:** On macOS, keyboard events are already serialized by `CGEventTap` and Tauri's global shortcut API. The coordinator is unnecessary for this path. On Linux, SIGUSR signals are asynchronous and need serialization to prevent race conditions. Since we target macOS only, the coordinator is effectively inert but kept for future Linux support.

**Known limitation:** The coordinator's `start()` checks `is_recording()` immediately after calling `action.start()`, but our `TranscribeAction::start` is async (spawns a thread). This means Linux signal-based toggle will desync. Deferred since macOS-only.

## Build Fixes

| Error | Fix |
|-------|-----|
| `log` crate in `transcription_coordinator.rs` | Changed to `tracing` for consistency with codebase |

## Verification

- `cargo check` — pending
- `bun run lint` — pending

## Notes

- "Codictate" branding preserved throughout
- Parakeet V3 kept as default model (Moonshine V2 is English-only)
- `cli.rs` command name still says `"handy"` — cosmetic, only affects `--help`
- Changes left **uncommitted** for manual review
