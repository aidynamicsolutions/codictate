# Merge origin/main → llm (2026-02-11)

## Commits from `origin/main`

This merge brings in the latest changes from `origin/main` since the 2026-02-10 merge. Key features integrated: auto-submit, show tray icon setting, unload model tray item.

## Conflict Resolution

| File | Strategy |
|------|----------|
| `lib.rs` | **Hybrid** – kept our imports/handlers + added `unload_model` tray handler, `show_tray_icon`/`auto_submit` commands in both specta blocks |
| `settings.rs` | **Hybrid** – kept our filler/hallucination filter fields + added `show_tray_icon`, `auto_submit`, `auto_submit_key` from main. Removed unused `experimental_enabled`/`keyboard_implementation` |
| `clipboard.rs` | **Keep ours** + added `AutoSubmitKey` and `enigo` imports from main |
| `tray.rs` | **Keep ours** + added `unload_model` tray item (made opt-in) and `set_tray_visibility` function |
| `shortcut/mod.rs` | **Keep ours** + added `AutoSubmitKey` import |
| `Cargo.toml` | **Hybrid** – kept Codictate branding, updated version to `0.7.4` |
| `tauri.conf.json` | **Hybrid** – kept Codictate branding, updated version to `0.7.4` |
| `Cargo.lock` | **Regenerated** – removed duplicate `handy` entry, kept `codictate` |
| `bindings.ts` | **Hybrid** – kept our types + added `show_tray_icon`, `auto_submit`, `auto_submit_key`, `AutoSubmitKey`. Fixed duplicate `changeAutoSubmitSetting`/`changeAutoSubmitKeySetting` commands |
| `settingsStore.ts` | **Hybrid** – kept our updaters + added `show_tray_icon` updater |
| 11 i18n translation files | **Took theirs** for `copyLastTranscript`/`unloadModel` keys, removed duplicates |

## Additional Changes (Post-Merge)

| Change | Description |
|--------|-------------|
| `show_unload_model_in_tray` setting | New opt-in setting (default `false`) to control visibility of "Unload Model" in tray menu |
| `ShowUnloadModelInTray.tsx` | New toggle component placed under Model Unload Timeout in Advanced Settings |
| `tray.rs` refactor | Idle menu now dynamically builds items list based on `has_history` and `show_unload_model_in_tray` |
| `doc/model.md` | Updated with tray menu documentation |

## Build Fixes

| Error | Fix |
|-------|-----|
| `update_tray_menu` not found | Renamed to `update_tray_menu_sync` (function was renamed in main) |
| Unused `debug` import in `settings.rs` | Removed |
| `&mut enigo` double borrow in `clipboard.rs` | Removed `&mut` (already a mutable borrow from `guard.as_mut()`) |

## Verification

- `cargo check` — pending
- `bun run lint` — pending

## Notes

- "Codictate" branding preserved throughout
- Version updated to `0.7.4` to stay in sync with main
- "Unload Model" tray item made opt-in (was always visible in main)
- Changes left **uncommitted** for manual review
