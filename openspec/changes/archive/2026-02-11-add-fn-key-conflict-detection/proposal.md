# Change: Add Fn Key Conflict Detection to Settings

## Why
When multiple applications compete for the macOS Fn/Globe key (e.g., another transcription app, system dictation), Codictate may stop receiving Fn key events. Users need a way to detect this conflict and get guidance on resolving it, but this should happen **in Settings** after onboarding—not during the initial setup flow where it adds unnecessary complexity.

## What Changes
- Add "Test Fn Key" functionality to the Shortcuts section under Settings
- Add conflict detection in the backend that tracks Fn key events
- Show troubleshooting tips when conflict is detected
- Optionally, detect conflict passively and show a notification directing users to Settings

## Impact
- Affected specs: `shortcut-settings` (new capability)
- Affected code:
  - `src-tauri/src/fn_key_monitor.rs` - Add test mode commands
  - `src-tauri/src/lib.rs` - Register new commands
  - `src/components/settings/ShortcutSettings.tsx` - Add test UI
  - `src/i18n/locales/en/translation.json` - Add translations
