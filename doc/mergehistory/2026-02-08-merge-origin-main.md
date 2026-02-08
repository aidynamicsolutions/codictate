# Merge History: origin/main -> llm (v0.7.2)

**Date:** 2026-02-08
**Versions Merged:** `origin/main` (v0.7.2) into `llm` (v0.7.2)

## Summary
Successfully merged `origin/main` into `llm`, bringing in the new "Models" settings page, updated Release workflows, and dependencies, while preserving the custom "Codictate" branding and onboarding flow.

## Conflict Resolution Details

### Branding & UI (Preserved from `llm`)
- **Product Name**: Kept as "Codictate" in `tauri.conf.json` and `src-tauri/Cargo.toml`.
- **Sidebar**:
  - Preserved "Codictate" logo (`CodictateLogo`).
  - Preserved `Dictionary` section.
  - Added new `Models` section from `origin/main`.
  - Omitted `Advanced` section (not present/needed in this branch).
- **Onboarding**: Kept custom multi-step onboarding flow (`src/components/onboarding/Onboarding.tsx`).
- **Shortcuts UI**: Adopted `SettingContainer` from `origin/main` for `GlobalShortcutInput.tsx` but kept custom logic.

### Settings & Features
- **General Settings**: Preserved `llm` structure (Shadcn UI components, Language Modal) while noting new model settings availability.
- **History**: Kept `llm` specific `HistorySettings.tsx` dialog logic.
- **Translation**:
  - Merged new keys for `models` and `shortcuts`.
  - Resolved `pasteDelay` duplicates.
  - Reset non-English locales to `HEAD` to avoid build breakage (will need future updates).

### Backend & Dependencies
- **Dependencies**:
  - Added `@tauri-apps/plugin-dialog` (from `origin/main`).
  - Added `tauri-plugin-dialog` crate.
  - Updated `package.json` and `Cargo.toml` to v0.7.2.
  - Regenerated `Cargo.lock` and `bun.lock` to resolve conflicts.
- **Capabilities**: Merged `dialog:default` permission.

## Verification
- `bun run lint`: Passed.
- `cargo check`: Passed.
- `Sidebar` structure verified manually via code review.

## Cleanup & Refactoring
- **Removed Legacy UI Components**:
  - `src/components/ui/Input.tsx` (Unused)
  - `src/components/ui/Button.tsx` (Unused)
  - `src/components/ui/TextDisplay.tsx` (Unused)
  - `src/components/ui/Badge.tsx` (Unused)
  - `src/components/ui/ResetButton.tsx` (Refactored to Shadcn)
  - `src/components/ui/Textarea.tsx` (Refactored to Shadcn)
  - `src/components/ui/PathDisplay.tsx` (Inlined/Refactored)
  - `src/components/settings/advanced/AdvancedSettings.tsx` (Dead code; Advanced settings are now integrated into `GeneralSettings.tsx`).
- **Migrated to Shadcn UI**:
  - `ModelCard.tsx`: Replaced legacy `Badge` and `Button` with Shadcn equivalents.
  - `PostProcessingSettings.tsx`: Replaced legacy `Textarea` and `ResetButton` with Shadcn `Textarea` and `Button` (ghost, icon).
  - `ClamshellMicrophoneSelector.tsx`: Replaced `ResetButton` with Shadcn `Button` (ghost, icon).
  - `AppDataDirectory.tsx` & `LogDirectory.tsx`: Inlined `PathDisplay` logic using Shadcn `Button`.
- **Build Fixes**:
  - Deleted `src/components/settings/HandyKeysShortcutInput.tsx` (Unused, caused type errors due to missing backend commands).
- **Backend**:
  - Restored `get_recommended_first_model` in `lib.rs` for macOS aarch64 to fix unused code warning.
