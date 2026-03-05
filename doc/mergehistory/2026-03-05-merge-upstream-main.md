# Merge upstream-main → main (2026-03-05)

## Context

- Source: `upstream/main` at `5ff3c89`
- Target: `main` at `6e15296`
- Result: pending commit (changes staged, not committed)

## Commits Reviewed from Upstream

| SHA | Description | Decision |
| --- | --- | --- |
| `0c83588` | use blocking for handy keys (#881) | **Skip** – we don't use `handy-keys` crate; macOS Fn key uses native `CGEventTap` |
| `551ad6c` | Add BoltAI as a sponsor! (image) | **Skip** – upstream README sponsor |
| `a832280` | Add BoltAI as a sponsor! (text fix) | **Skip** – upstream README sponsor |
| `4e4bc61` | please add logs when submitting bugs | **Already taken** – cherry-picked in `6ba7794` |
| `52047ae` | release v0.7.8 | **Superseded** – by v0.7.9 below |
| `f705a49` | Update README.md for improved description | **Skip** – upstream README; we have our own |
| `f1516d9` | fix: auto-refresh model list when switching post-processing providers (#854) | **Take** – clears stale model cache on provider/base-URL switch |
| `1d4d682` | [macOS] Fix tray icon disabled + start hidden → permanently invisible (#903) | **Take** – critical macOS fix: guards activation policy with `tray_visible`, adds `RunEvent::Reopen` handler |
| `8626357` | keyboardImplementation translations (FR, PT, ES, TR) (#910) | **Take** – i18n corrections for 13 locales |
| `2361b85` | fix(i18n): tray menu shows Simplified Chinese when Traditional Chinese selected (#901) | **Take** – `tray_i18n.rs` now tries full locale before language prefix |
| `17d34a9` | fix: upgrade tauri-plugin-updater to v2.10.0 (#876) | **Take** – fixes Windows updater duplicate registry entries; upgrades tauri 2.9→2.10, dialog, and `patch.crates-io` branches |
| `ff86122` | feat: add GigaAM v3 for Russian speech recognition (#913) | **Take** – additive engine type in `model.rs`/`transcription.rs` + i18n keys |
| `b0aa234` | Update translation.json (fr) | **Take** – minor French translation fix |
| `eade87a` | upgrade to handy keys 0.2.2 (#926) | **Skip** – we don't use `handy-keys` |
| `f403cb1` | update transcribe-rs | **Take** – needed for GigaAM + general fixes |
| `998449d` | release v0.7.9 | **Take** – version bumps to 0.7.9 with Codictate branding |
| `a6b5c32` | move to tauri dialog 2.6 | **Take** – `tauri-plugin-dialog` dep bump |
| `a50c59c` | docs: add Linux install steps to BUILD.md (#951) | **Take** – documentation only |
| `5ff3c89` | Update translation.json (it) (#961) | **Take** – one-line Italian translation fix |

## Conflict Resolution

| File | Strategy | Notes |
| --- | --- | --- |
| `Cargo.toml` | **Hybrid** | Codictate branding + v0.7.9, tauri 2.10.2, `handy-2.10.2` patch branches, kept our sentry/aptabase/tracing/backup deps |
| `tauri.conf.json` | **Hybrid** | Codictate branding + `com.pais.codictate` + v0.7.9 |
| `lib.rs` | **Hybrid** | Took upstream `tray_visible` guard in `CloseRequested` (used `tracing::error!` not `log::error!`), kept our `RunEvent` handler (overlay suppression, analytics, MLX sidecar), kept our tests module |
| `package.json` | **Hybrid** | Kept our `@tauri-apps/api` 2.10.1 (higher), our extra deps (canvas-confetti, cva, clsx, cmdk), `plugin-store` 2.4.2; took upstream `~2.10.0` tilde range for updater |
| `bindings.ts` | **Hybrid** | Kept our `CustomWordEntry`, richer `HistoryEntry`, `HistoryStats`, `HomeStats`; added `GigaAM` to `EngineType`; skipped `ImplementationChangeResult`/`KeyboardImplementation` (handy-keys) |
| `BUILD.md` | **Hybrid** | Kept our Tailwind source scope section; took upstream production build, Linux install, and AppImage troubleshooting docs |
| `README.md` | **Keep ours** | Codictate README |
| `handy_keys.rs` | **Delete** | modify/delete conflict; we don't use handy-keys |
| 11 i18n translation files | **Hybrid** | Kept our Smart Insertion labels (superior to upstream "Append trailing space"); added upstream `keyboardImplementation` translation keys |
| `Cargo.lock` | **Took theirs** + regenerated via `cargo check` |
| `bun.lock` | **Took theirs** + regenerated via `bun install` |

### Auto-merged Files (no conflicts)

| File | Source |
| --- | --- |
| `src-tauri/src/managers/model.rs` | GigaAM v3 model info + `EngineType::GigaAM` variant |
| `src-tauri/src/managers/transcription.rs` | GigaAM engine loading + transcription path |
| `src-tauri/src/tray_i18n.rs` | zh-TW locale lookup fix |
| `src/stores/settingsStore.ts` | Post-process provider auto-refresh fix |
| `src/components/settings/PostProcessingSettingsApi/usePostProcessProviderState.ts` | Auto-fetch models on provider switch |
| `.github/ISSUE_TEMPLATE/bug_report.md` | Logs section for bug reports |
| `sponsor-images/boltai.jpg` | Upstream sponsor image (auto-merged) |
| 6 i18n files (ar, cs, ko, tr, zh-TW) | Auto-merged translation keys |

## Verification

| Command | Result |
| --- | --- |
| `cargo check` | ✅ PASS |
| `bun run lint` | ✅ PASS (0 errors, 1 pre-existing warning) |
| `bun run test` | ✅ PASS (105 tests, 13 files) |
| JSON validation (17 i18n files) | ✅ PASS |
| Conflict marker check | ✅ No remaining markers |

## Final Status

- Merge commit: **not yet committed** (staged for manual review)
- Safety branch: `codex/safety-main-pre-upstream-merge-2026-03-05`
- Follow-up tasks: none
