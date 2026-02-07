# Merge: origin/main → llm Branch

**Date:** 2026-02-07
**Merged Commits:** Updates from `origin/main` (approx. 14 commits)
**LLM Branch State:** Updates merged, conflicts resolved.
**Result:** Successful merge with significant manual conflict resolution.

---

## Overview

This merge integrates latest features from `main` (N-gram word correction, RTL support, new languages) into the `llm` branch, while preserving our specific branding ("Codictate") and advanced features (Double Metaphone phonetic matching).

- **Main**: Introduced N-gram based custom word matching, RTL support (Arabic/Hebrew), Korean language, and various settings updates.
- **LLM Branch**: Maintained "Best of Both Worlds" word correction, custom Fn-key handling, and specific UI branding.

## Resolution Decisions

### ✅ Kept from LLM Branch (Ours)

| Item | Rationale |
|------|-----------|
| **Phonetic Matching** | We kept `rphonetic` (Double Metaphone) and `strsim` (Jaro-Winkler) for superior word correction accuracy. |
| **Branding** | Preserved "Codictate" product name and logo in `tauri.conf.json` and `Sidebar.tsx`. |
| **Fn Key Handling** | Rejected `main`'s `keyboard_implementation` in favor of our custom `fn_key_monitor` logic. |
| **Settings UI** | Maintained our Shadcn-based UI components in `Sidebar.tsx`. |

### ✅ Brought In from Main (Theirs)

| Item | Rationale |
|------|-----------|
| **N-gram Logic** | Adopted the N-gram sliding window structure to enable multi-word phrase matching (e.g. "Chat G P T"). |
| **RTL Support** | Integrated `rtl.ts` and UI direction changes for Arabic/Hebrew support. |
| **New Languages** | Added Korean (`ko`) and improved Arabic (`ar`) translations. |
| **Post-Process Settings** | Merged new post-processing configuration options and UI. |

### 🛠️ Manual Merges & Fixes

| File | Resolution |
|------|------------|
| `src-tauri/src/audio_toolkit/text.rs` | **Best of Both Worlds**: Implemented hybrid logic using `main`'s N-gram loop with `llm`'s Double Metaphone matching. |
| `src-tauri/src/actions.rs` | Resolved duplicate transcription logic blocks and fixed missing `post_process_transcription` definitions. |
| `src-tauri/src/shortcut/mod.rs` | Resolved conflicts, registered missing commands, and removed unused `experimental_enabled` setting. |
| `src/App.tsx` | Merged RTL initialization and new language support while keeping existing onboarding flow. |
| `src/components/Sidebar.tsx` | Merged new settings links while ensuring "Codictate" logo and branding remained. |

---

## Verification

- **Word Correction**: Verified "Best of Both Worlds" logic (N-gram + Phonetic) compiles and follows design.
- **Branding**: Checked `tauri.conf.json`, `Sidebar.tsx`, and `App.tsx` for correct "Codictate" usage.
- **Build**: 
    - Frontend: `bun run build` - **PASSED**
    - Backend: `cargo check` - **PASSED** (all warnings resolved)
- **Cleanup**: Removed unused `experimental_enabled` setting from backend and frontend.
