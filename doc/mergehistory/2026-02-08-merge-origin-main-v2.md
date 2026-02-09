# Merge History: origin/main -> llm (v0.7.2)

**Date:** 2026-02-08
**Versions Merged:** `origin/main` (v0.7.2) into `llm` (v0.7.2)

## Summary
Successfully merged `origin/main` into `llm`, bringing in the new "Models" settings page, updated Release workflows, and dependencies, while preserving the custom "Codictate" branding and onboarding flow.

## Conflict Resolution Details

### Branding & UI (Preserved from `llm`)
- **Product Name**: Kept as "Codictate" in `tauri.conf.json` and `src-tauri/Cargo.toml`.
- **src/components/onboarding/ModelCard.tsx**:
    - Preserved Shadcn UI styling for the delete button (`text-destructive`) instead of the upstream `text-logo-primary`.
- **Sidebar**:
  - Preserved "Codictate" logo (`CodictateLogo`).
  - Preserved `Dictionary` section.
  - Added new `Models` section from `origin/main`.
  - Omitted `Advanced` section (not present/needed in this branch).
- **Onboarding**: Kept custom multi-step onboarding flow (`src/components/onboarding/Onboarding.tsx`).

### Backend & Dependencies
- **src-tauri/src/managers/model.rs**: Auto-merged successfully. Verified structure matches expectations with new model definitions (Moonshine, Breeze ASR).
- **Dependencies**:
  - `package.json` and `Cargo.toml` updated to v0.7.2.
  - `bun.lock` and `Cargo.lock` updated.

## Verification
- `bun run lint`: Passed.
- `cargo check`: Passed.
- Branding consistency check: Passed (Codictate name and logos preserved).
