# Merge upstream-main -> main (2026-03-09)

## Context

- Source: `upstream/main` at `fc05e4a`
- Target: `main` at `c53db9b`
- Result: pending merge (resolved locally, not committed)

## Commits Reviewed from Upstream

| SHA | Description | Decision |
| --- | --- | --- |
| `10beb60` | feat: add portable mode to NSIS installer (#807) | **Skip** – Windows-only packaging/runtime change with high merge surface; not worth pulling into a macOS-first fork right now. |
| `f8c74e9` | Fix Italian translation (#973) | **Take (adapted)** – took the corrected Italian wording that fits our current locale structure and kept existing fork-specific phrasing where upstream context no longer matched. |
| `615b3c9` | feat: language-aware filler word removal (#971) | **Partial take** – took the language-aware behavior fix, but adapted it to our counting pipeline and changed the custom-word semantics to additive backend-only extras instead of upstream override semantics. |
| `e354c0a` | update dialog package.json | **Take (adapted)** – aligned frontend `@tauri-apps/plugin-dialog` to `~2.6` and regenerated `bun.lock` while preserving our broader dependency set. |
| `f221f83` | Merge branch 'main' of github.com:cjpais/Handy | **Skip** – merge commit with no standalone value for selective sync. |
| `fc05e4a` | fix: simplify Bun setup for Windows ARM64 (#965) | **Take** – low-risk CI cleanup removing the obsolete special-case Bun bootstrap workaround. |

## Conflict Resolution

| File | Strategy | Notes |
| --- | --- | --- |
| `.github/workflows/build.yml` | **Take upstream** | Kept the upstream Bun setup simplification only. |
| `package.json` | **Hybrid** | Took the intent of upstream `e354c0a` by bumping frontend `@tauri-apps/plugin-dialog` from `~2` to `~2.6`, while keeping our existing Codictate deps and scripts. |
| `bun.lock` | **Regenerated** | Refreshed with `bun install --lockfile-only` so the dialog plugin resolves to `2.6.0`. |
| `src-tauri/src/lib.rs` | **Keep ours** | Rejected portable-mode bootstrap and window-creation changes; preserved current Codictate startup, observability, and macOS behavior. |
| `src-tauri/tauri.conf.json` | **Keep ours** | Rejected portable-mode NSIS template wiring and window config replacement; preserved current Codictate branding and window settings. |
| `src-tauri/src/settings.rs` | **Hybrid** | Kept our settings model and added hidden backend-only `extra_filler_words` with additive semantics. |
| `src/bindings.ts` | **Hybrid** | Added generated type support for `extra_filler_words`; no UI or update command added in this pass. |
| `src-tauri/src/managers/transcription.rs` | **Hybrid** | Kept our filter ordering and filler-removal count path; threaded through `app_language` and `extra_filler_words`. |
| `src-tauri/src/audio_toolkit/text.rs` | **Hybrid** | Replaced static filler list logic with language-aware defaults plus additive extras, but kept our existing counting API and hallucination split. |
| `src/i18n/locales/it/translation.json` | **Hybrid** | Took the upstream Italian fixes that map cleanly onto our current locale file and preserved existing fork-specific labels where upstream wording did not fit the evolved structure. |
| `src-tauri/src/audio_feedback.rs` | **Keep ours** | Rejected portable-aware path resolution. |
| `src-tauri/src/commands/audio.rs` | **Keep ours** | Rejected portable-aware path resolution. |
| `src-tauri/src/commands/mod.rs` | **Keep ours** | Rejected portable-aware app/log directory changes. |
| `src-tauri/src/managers/history.rs` | **Keep ours** | Rejected portable-aware app-data changes. |
| `src-tauri/src/managers/model.rs` | **Keep ours** | Rejected portable-aware model path changes. |
| `src-tauri/src/overlay.rs` | **Keep ours** | Rejected portable-mode WebView data-directory override. |
| `src-tauri/src/portable.rs` | **Delete** | Portable-mode source file deliberately excluded from the merge. |
| `src-tauri/nsis/installer.nsi` | **Delete** | Portable-mode installer template deliberately excluded from the merge. |

## Verification

| Command | Result |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml test_filter_` | ✅ PASS |
| `cargo check --manifest-path src-tauri/Cargo.toml` | ✅ PASS |
| `bun run lint` | ✅ PASS (0 errors, 1 pre-existing warning in `src/components/home/StatsOverview.tsx`) |
| `python3 -m json.tool src/i18n/locales/it/translation.json >/dev/null` | ✅ PASS |
| `git diff --check` | ✅ PASS |

## Final Status

- Merge commit: **not yet committed**
- Safety branch: `codex/safety-main-pre-upstream-merge-2026-03-09`
- Upstream review branch: `upstream-main` updated to `fc05e4a`
- Follow-up tasks:
  - none for this sync
