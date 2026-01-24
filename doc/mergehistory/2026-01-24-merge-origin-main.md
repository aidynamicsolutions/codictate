# Merge: origin/main → llm Branch

**Date:** 2026-01-24  
**Merged Commits:** 16 commits from `origin/main` (bfbeb32..713d0b3)  
**LLM Branch State:** 41 commits ahead of common ancestor  
**Result:** 2 commits (`0a45fb3` merge, `f1dc4a2` post-merge fixes)

---

## Overview

This document records the resolution decisions made when merging `origin/main` into the `llm` feature branch. The `llm` branch contains extensive work on:
- Onboarding flow redesign
- MLX integration for local AI
- "Codictate" rebranding
- User profile system
- Advanced word correction (Double Metaphone)

Main contained general improvements including new translations, UI refinements, and keybinding enhancements.

---

## Commits Merged from Main

| Commit | Description |
|--------|-------------|
| 713d0b3 | v0.7.0 release |
| various | Czech and Turkish translations |
| various | `immer` for immutable state updates |
| various | Copy last transcript tray action |
| various | Automatic filler word removal |
| various | Keyboard implementation selection (handy_keys) |
| various | Experimental features toggle |
| various | Various bug fixes |

---

## Resolution Decisions

### ✅ Kept from LLM Branch

| Item | Rationale |
|------|-----------|
| **"Codictate" branding** | Active rebrand in progress; kept `productName: "Codictate"` and `identifier: com.codictate.app` |
| **Advanced word correction** | Uses `rphonetic` + Double Metaphone for superior phonetic matching vs main's simpler `natural` + `soundex` approach |
| **Comprehensive onboarding** | More complete onboarding flow with permissions, microphone check, hotkey setup, model download |
| **User profile system** | `user_profile.rs` and related commands for personalization |
| **MLX model manager** | Local AI integration for post-processing |
| **Flag emojis in language selector** | Better visual identification of languages |
| **Frontend logging** | `log_from_frontend` command for unified session-correlated logging |
| **Onboarding paste override** | `set_onboarding_paste_override` to work around WebView Cmd+V issue |
| **Lazy Enigo initialization** | `EnigoState::new()` with lazy init pattern vs main's deferred-only approach |

### ✅ Brought In from Main

| Item | Rationale |
|------|-----------|
| **Version 0.7.0** | Updated version number to match release |
| **Czech + Turkish translations** | New language support |
| **`immer` dependency** | For immutable state updates in frontend |
| **`filter_transcription_output`** | Filler word removal (uh, um, hmm) and stutter collapse - useful feature |
| **`copy_last_transcript` tray action** | Simpler sync implementation with unit tests |
| **`get_latest_entry()` on HistoryManager** | Sync version for tray menu |
| **`natural` and `regex` dependencies** | May be useful for future enhancements |
| **`initializeEnigo` command** | Frontend can explicitly initialize after permissions granted |
| **AccessibilityOnboarding component** | Dedicated permission flow from main |

### ❌ Left Out from Main (Not Compatible)

| Item | Reason Left Out |
|------|-----------------|
| **`keyboard_implementation` setting** | Requires `handy_keys` backend module we don't have |
| **`handy_keys` module** | Alternative keyboard input implementation - would conflict with our approach |
| **`HandyKeysShortcutInput.tsx`** | UI for `handy_keys` feature |
| **`KeyboardImplementationSelector.tsx`** | UI for switching keyboard implementations |
| **`ExperimentalToggle.tsx`** | Requires `experimental_enabled` backend setting |
| **`changeExperimentalEnabledSetting` command** | Not in our settings.rs |
| **`changeKeyboardImplementationSetting` command** | Not in our shortcut/mod.rs |
| **`start_handy_keys_recording` / `stop_handy_keys_recording`** | Not in our codebase |

---

## File-by-File Conflict Resolutions

### Rust Backend

| File | Resolution |
|------|------------|
| `src-tauri/Cargo.toml` | Kept "codictate" name, v0.7.0 version, kept `rphonetic`, added `natural` + `regex` |
| `src-tauri/tauri.conf.json` | Kept "Codictate" productName + identifier, updated to v0.7.0 |
| `src-tauri/src/lib.rs` | Kept our specta commands, added `initializeEnigo`, removed handy_keys references |
| `src-tauri/src/commands/mod.rs` | Merged: kept our `log_from_frontend` + `set_onboarding_paste_override`, added `initializeEnigo` adapted to our lazy-init pattern |
| `src-tauri/src/tray.rs` | Kept conditional has_history menu, adopted main's simpler `copy_last_transcript` with tests, kept `tracing` logging |
| `src-tauri/src/audio_toolkit/text.rs` | Kept our Double Metaphone algorithm, added `filter_transcription_output` from main |
| `src-tauri/src/actions.rs` | Kept ours (more comprehensive flow with permission checks) |
| `src-tauri/src/settings.rs` | Kept ours |
| `src-tauri/src/shortcut/mod.rs` | Kept ours |

### Frontend

| File | Resolution |
|------|------------|
| `package.json` | Combined: kept newer lucide-react + radix-ui, added immer |
| `src/bindings.ts` | Kept ours, manually added `initializeEnigo` command |
| `src/i18n/languages.ts` | Combined: kept flag emojis, added Czech + Turkish with flags |
| `src/i18n/locales/*/translation.json` | Kept ours (comprehensive onboarding), updated `copyLastRecording` → `copyLastTranscript` |
| `src/components/settings/ShortcutInput.tsx` | Simplified to use `HandyShortcut` only (removed handy_keys switching) |
| `src/components/settings/index.ts` | Removed HandyKeysShortcutInput export |
| `src/stores/settingsStore.ts` | Removed `experimental_enabled` updater |
| `src/App.tsx` | Kept ours (comprehensive onboarding flow) |
| `src/main.tsx` | Kept ours (dark mode sync + model store init) |

---

## Future Considerations

1. **Keyboard Implementation**: If we want the `handy_keys` feature in the future, we'd need to port the Rust modules from main.

2. **Experimental Features Toggle**: Could be added if we implement `experimental_enabled` in settings.rs.

3. **Periodic Merges**: This branch should be periodically merged with main to avoid divergence. Recommend monthly merge reviews.

---

## Build Verification

- ✅ `cargo check` passes
- ✅ `bun run build` passes (TypeScript compiles, Vite bundles)
- ⚠️ Large chunk warning (>500KB) - pre-existing, not related to merge

---

## Git Commands Used

```bash
# Initial merge
git fetch origin
git merge origin/main --no-commit

# Conflict resolution (33 files)
# Lock files - took main's versions
git checkout --theirs bun.lock src-tauri/Cargo.lock

# Config files - manual merge
# Translation files - took ours + updated key names
# Rust files - mostly took ours with selective additions

# Final commits
git commit -m "Merge origin/main into llm"
git commit -m "fix: Post-merge build fixes"
```
