# Local Patch Notes: tauri-plugin-aptabase

This vendored plugin includes a local runtime-safety patch for Codictate.

## Why this patch exists

- Upstream `tauri-plugin-aptabase` v1.0.0 calls `tokio::spawn(...)` in plugin setup.
- In this app context, that can panic at startup with:
  `there is no reactor running, must be called from the context of a Tokio 1.x runtime`.

## Patch applied

- File: `src/client.rs`
- Change: `tokio::spawn(...)` -> `tauri::async_runtime::spawn(...)`

## Upgrade/removal

Before upgrading/removing this vendored patch, follow:

- `doc/analytic_error_monitoring/aptabase_plugin_patch_maintenance.md`

