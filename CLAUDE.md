<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Prerequisites:** [Rust](https://rustup.rs/) (latest stable), [Bun](https://bun.sh/)

```bash
# Install dependencies
bun install

# Run in development mode (different log levels)
bun run tauri:dev          # Handy logs only (quietest, recommended)
bun run tauri:dev:debug    # Handy DEBUG + suppress noisy deps
bun run tauri:dev:verbose  # All logs including deps (noisiest)

# Legacy command (uses default RUST_LOG)
bun run tauri dev

# If cmake error on macOS:
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri:dev

# Build for production
bun run tauri build

# Linting and formatting (run before committing)
bun run lint              # ESLint for frontend
bun run lint:fix          # ESLint with auto-fix
bun run format            # Prettier + cargo fmt
bun run format:check      # Check formatting without changes
```

**Model Setup (Required for Development):**

```bash
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

## Architecture Overview

Handy is a cross-platform desktop speech-to-text app built with Tauri 2.x (Rust backend + React/TypeScript frontend).

### Backend Structure (src-tauri/src/)

- `lib.rs` - Main entry point, Tauri setup, manager initialization
- `managers/` - Core business logic:
  - `audio.rs` - Audio recording and device management
  - `model.rs` - Model downloading and management
  - `transcription.rs` - Speech-to-text processing pipeline
  - `history.rs` - Transcription history storage
- `audio_toolkit/` - Low-level audio processing:
  - `audio/` - Device enumeration, recording, resampling
  - `vad/` - Voice Activity Detection (Silero VAD)
- `commands/` - Tauri command handlers for frontend communication
- `shortcut.rs` - Global keyboard shortcut handling
- `fn_key_monitor.rs` - Native macOS Fn/Globe key detection (cannot use Tauri global shortcuts for fn)
- `settings.rs` - Application settings management

### Keyboard Shortcuts

**Fn key on macOS**: The Fn/Globe key cannot be captured via Tauri's global shortcut API. It's handled natively in `fn_key_monitor.rs` using `CGEventTap`, emitting `fn-key-down`/`fn-key-up` Tauri events.

**Reserved shortcuts**: Common system shortcuts are blocked in `shortcut.rs` to prevent users from accidentally overriding copy/paste etc. Includes:
- macOS: `fn+a/c/d/e/f/h/m/n/q`, `cmd+c/v/x/z/a/s/tab/space/q`
- Windows/Linux: `ctrl+c/v/x/z/a/s`, `alt+tab/f4`, `super+l/d`

**Shortcut recording**: Use the shared `useShortcutRecorder` hook (`src/hooks/useShortcutRecorder.ts`) for recording shortcuts. Key patterns:
- Uses refs (`isRecordingRef`, `recordedKeysRef`, `saveInProgress`) for synchronous access in async callbacks
- Avoid calling async functions from within `setState` updaters
- Use `resetBindings` (plural) to atomically reset multiple shortcuts

### Frontend Structure (src/)

- `App.tsx` - Main component with onboarding flow
- `components/settings/` - Settings UI (35+ files)
- `components/model-selector/` - Model management interface
- `components/onboarding/` - First-run experience
- `hooks/useSettings.ts`, `useModels.ts` - State management hooks
- `stores/settingsStore.ts` - Zustand store for settings
- `bindings.ts` - Auto-generated Tauri type bindings (via tauri-specta)
- `overlay/` - Recording overlay window code

### Key Patterns

**Manager Pattern:** Core functionality organized into managers (Audio, Model, Transcription) initialized at startup and managed via Tauri state.

**Command-Event Architecture:** Frontend → Backend via Tauri commands; Backend → Frontend via events.

**Pipeline Processing:** Audio → VAD → Whisper/Parakeet → Text output → Clipboard/Paste

**State Flow:** Zustand → Tauri Command → Rust State → Persistence (tauri-plugin-store)

## Internationalization (i18n)

All user-facing strings must use i18next translations. ESLint enforces this (no hardcoded strings in JSX).

**Adding new text:**

1. Add key to `src/i18n/locales/en/translation.json`
2. Use in component: `const { t } = useTranslation(); t('key.path')`

**File structure:**

```
src/i18n/
├── index.ts           # i18n setup
├── languages.ts       # Language metadata
└── locales/
    ├── en/translation.json  # English (source)
    ├── es/translation.json  # Spanish
    ├── fr/translation.json  # French
    └── vi/translation.json  # Vietnamese
```

## Code Style

**Rust:**

- Run `cargo fmt` and `cargo clippy` before committing
- Handle errors explicitly (avoid unwrap in production)
- Use descriptive names, add doc comments for public APIs

**TypeScript/React:**

- Strict TypeScript, avoid `any` types
- Functional components with hooks
- Tailwind CSS v4 + shadcn/ui for styling
- Path aliases: `@/` → `./src/`

## shadcn/ui

UI components use [shadcn/ui](https://ui.shadcn.com/) with Tailwind CSS v4.

**Adding components:**

```bash
bunx shadcn@latest add button
bunx shadcn@latest add card
```

**Key files:**

- `src/App.css` - Tailwind + shadcn color variables (OKLCH), brand colors
- `src/lib/utils.ts` - `cn()` utility for class merging
- `components.json` - shadcn CLI config (style: radix-maia, RSC: false)

**Dark mode:** Auto-syncs with system preference via `.dark` class on `<html>` (set in `src/main.tsx`).

**Protected files:** Do NOT edit files in `src/components/shared/ui/` — these are auto-generated by shadcn CLI. To customize, create wrapper components elsewhere.

## Commit Guidelines

Use conventional commits:

- `feat:` new features
- `fix:` bug fixes
- `docs:` documentation
- `refactor:` code refactoring
- `chore:` maintenance

## Debug Mode

Access debug features: `Cmd+Shift+D` (macOS) or `Ctrl+Shift+D` (Windows/Linux)

## Platform Notes

- **macOS**: Metal acceleration, accessibility permissions required
- **Windows**: Vulkan acceleration, code signing
- **Linux**: OpenBLAS + Vulkan, limited Wayland support, overlay disabled by default

## macOS Permissions

See `doc/permission.md` for full architecture. Key learnings:

- **Microphone check**: Use `objc2` crate with `msg_send!` macro. Raw `objc_msgSend` FFI crashes on ARM64.
- **cpal limitation**: The audio library cannot detect permission denial—macOS still opens devices but silences audio.
- **TapDisabled callbacks**: Keep fast. Heavy work (notifications, i18n) causes keyboard lockup. Spawn a thread instead.

## Window Configuration

Main window size is configured in `src-tauri/tauri.conf.json`:

- **Default size:** 1280×800 pixels (fills ~74% of 16" MBP, ~89% of 13" MacBook)
- **Minimum size:** 1100×700 pixels (allows minor resizing without layout redesign)
- **Maximizable:** false (window cannot be maximized)
- **Centered:** macOS centers the window by default on launch

These dimensions follow macOS HIG best practices, targeting 80% screen coverage on the most common MacBook displays (13" Air/Pro at 1440×900 logical resolution).
