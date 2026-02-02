# Merge: origin/main → llm Branch

**Date:** 2026-02-02
**Merged Commits:** 3 commits from `origin/main` (5ba5f77..90bfa73)
**LLM Branch State:** Ahead of common ancestor
**Result:** Merge commit with manual conflict resolution

---

## Overview

This merge integrates recent fixes and features from `main` into the `llm` branch.
- **Main**: Added Arabic support, Paste Delay setting, and basic history fixes (Linux audio support).
- **LLM Branch**: Contains extensive history refactoring (virtualization, search).

## Resolution Decisions

### ✅ Kept from LLM Branch (Ours)

| Item | Rationale |
|------|-----------|
| **History Architecture** | We kept our virtualized `HistoryList` and `useHistory` hook instead of reverting to `main`'s simpler fix. |
| **HistorySettings.tsx** | Kept our refactored settings page which uses the new history components. |
| **Product Name** | Kept "Codictate" branding over "Handy". |

### ✅ Brought In from Main (Theirs)

| Item | Rationale |
|------|-----------|
| **AudioPlayer Lazy Loading** | Adopted `onLoadRequest` support in `AudioPlayer` to enable lazy loading of audio blobs on Linux. |
| **Paste Delay Setting** | Added `paste_delay_ms` to `AppSettings` and `clipboard.rs` to support slow target applications. |
| **Arabic Translations** | Added new `ar` locale. |
| **Linux Audio Fix** | Integrated lazy loading logic into `HistoryList.tsx` to prevent UI freezes on Linux. |

### 🛠️ Manual Merges

| File | Resolution |
|------|------------|
| `src-tauri/src/clipboard.rs` | Combined `onboarding_override` (ours) with `paste_delay_ms` (theirs). |
| `src-tauri/src/settings.rs` | Merged `keyboard_implementation` and `paste_delay_ms` fields while keeping our comments. |
| `src/components/shared/HistoryList.tsx` | Manually updated to use `AudioPlayer`'s new `onLoadRequest` API for Linux support. |

---

## Verification

- **Audio Playback**: Verified `AudioPlayer` works with both `src` (Mac/Win) and `onLoadRequest` (Linux simulation).
- **History List**: confirmed virtualization and search still function.
- **Build**: valid `cargo check` and `bun run build`.
