# Change: Refactor Dictionary Storage Split

## Why
Dictionary entries are currently embedded in `AppSettings` and persisted in `settings_store.json`, which couples dictionary behavior to the settings lifecycle and reset flows. This increases blast radius for settings refactors and creates hot-path read dependencies on settings payload shape.

For current development stage, we want a simpler and more explicit architecture:
- dictionary as an independent persistence unit
- hard reset policy (no legacy migration/import)
- no production compatibility burden

## What Changes
- Remove `dictionary` from `AppSettings` and stop treating dictionary as a settings field.
- Introduce dedicated dictionary persistence in app data at `user_dictionary.json`.
- Add dedicated dictionary commands (`get_user_dictionary`, `set_user_dictionary`).
- Add in-memory dictionary state for runtime consumers; transcription/correction read from memory, not disk per request.
- Implement disk-write-first dictionary updates with serialized write gate and atomic file replacement.
- Keep legacy `settings.dictionary` bytes untouched (no cleanup or migration).
- Update reset behavior so `reset_app_settings` resets settings only and does not affect dictionary data.

## Impact
- **Affected specs**: `custom-word-correction`
- **Affected code**:
  - `src-tauri/src/settings.rs`
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/commands/dictionary.rs` (new)
  - `src-tauri/src/user_dictionary.rs` (new)
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/managers/transcription.rs`
  - `src-tauri/src/managers/correction.rs`
  - `src/stores/settingsStore.ts`
  - `src/stores/dictionaryStore.ts` (new)
  - `src/hooks/useDictionary.ts` (new)
  - `src/components/dictionary/DictionaryPage.tsx`
  - `src/components/shared/HistoryList.tsx`
  - `src/bindings.ts` (regenerated)
- **Behavioral impact**:
  - Existing legacy dictionary data in settings is intentionally not imported.
  - Dictionary updates become explicitly isolated from settings updates/reset.

