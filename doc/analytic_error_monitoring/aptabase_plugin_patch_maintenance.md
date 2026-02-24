# Aptabase Plugin Patch Maintenance

This document explains the temporary local patch applied to `tauri-plugin-aptabase` and how to handle future upgrades safely.

## Scope

- App: Codictate (`src-tauri`)
- Plugin: `tauri-plugin-aptabase` v1.0.0
- Local override: `[patch.crates-io] tauri-plugin-aptabase = { path = "vendor/tauri-plugin-aptabase" }`

## Why we patched

In this app startup context, upstream v1.0.0 can panic during launch:

`there is no reactor running, must be called from the context of a Tokio 1.x runtime`

Root cause:

- Upstream plugin starts its polling loop with `tokio::spawn(...)` in setup.
- If a Tokio reactor is not entered for that call site, startup panics.

## What was changed locally

- Vendored plugin path: `src-tauri/vendor/tauri-plugin-aptabase`
- File patched: `src-tauri/vendor/tauri-plugin-aptabase/src/client.rs`
- One-line change:
  - from `tokio::spawn(...)`
  - to `tauri::async_runtime::spawn(...)`
- Rust deprecation compatibility patch:
  - file: `src-tauri/vendor/tauri-plugin-aptabase/src/lib.rs`
  - updated panic hook type usage from `std::panic::PanicInfo` to `std::panic::PanicHookInfo` (Rust 1.81+ warning cleanup)

This keeps behavior equivalent (background polling) while using Tauri's runtime wrapper that safely enters runtime context.

## Upgrade Checklist (Required)

When upgrading `tauri-plugin-aptabase`, follow this sequence:

1. Check upstream release notes/source for equivalent fix.
2. Compare upstream `src/client.rs` polling spawn behavior.
3. If fixed upstream:
   - remove local patch override in `src-tauri/Cargo.toml`
   - remove `src-tauri/vendor/tauri-plugin-aptabase`
   - run `cargo update -p tauri-plugin-aptabase --manifest-path src-tauri/Cargo.toml`
4. If not fixed upstream:
   - keep vendored override and re-apply minimal patch on the new version.
   - re-check `src/lib.rs` panic hook types (`PanicInfo` vs `PanicHookInfo`) and keep the non-deprecated type.
5. Validate before merge:
   - `bun run tauri:dev` starts without Tokio reactor panic
   - startup logs show analytics enabled/disabled status correctly
   - Aptabase receives `app_started` in debug run
   - graceful quit sends `app_exited`

## Fast Verification Commands

```bash
# Confirm which aptabase crate is resolved
cargo tree -p tauri-plugin-aptabase --manifest-path src-tauri/Cargo.toml

# Check startup analytics status in latest app log
latest=$(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)
rg -n "Analytics enabled|Analytics disabled" "$latest" | tail -n 5
```

## Ownership Note

This patch should be treated as temporary technical debt. Prefer removing it once upstream includes a stable fix.
