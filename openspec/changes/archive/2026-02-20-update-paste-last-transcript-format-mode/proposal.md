# Change: Deterministic Paste-Last Formatting Mode

## Why
`paste_last_transcript` currently re-applies smart insertion at paste time, which can make inserted output differ from the History primary text users are trying to replay. This creates trust and predictability issues for a command that users expect to be deterministic.

## What Changes
- Add a new setting: `paste_last_use_smart_insertion` (default `false`).
- Make `paste_last_transcript` paste literal History primary text (`effective_text`) by default.
- Add optional adaptive mode for paste-last that reuses existing smart insertion formatting.
- Keep transcribe and refine flows unchanged.
- Add backend/frontend tests and documentation updates for deterministic vs adaptive behavior.

## Impact
- **Affected Specs**: `transcript-insertion`
- **Affected Code**: `src-tauri/src/actions.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/settings.rs`, `src-tauri/src/shortcut/mod.rs`, `src-tauri/src/lib.rs`, `src/stores/settingsStore.ts`, `src/components/settings/`
- **Data Safety**: No database schema/data changes.
