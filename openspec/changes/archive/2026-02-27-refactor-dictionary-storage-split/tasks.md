## 1. OpenSpec Delta
- [x] 1.1 Add `custom-word-correction` spec delta for dictionary storage split and migration removal.
- [x] 1.2 Validate with `openspec validate refactor-dictionary-storage-split --strict`.

## 2. Backend Dictionary Split
- [x] 2.1 Add `src-tauri/src/user_dictionary.rs` with `CustomWordEntry`, envelope schema, load/save helpers, and runtime dictionary state types.
- [x] 2.2 Implement version policy (`version == 1` only; warn + empty fallback otherwise).
- [x] 2.3 Implement crash-safe persistence:
  - [x] temp file in same directory as target
  - [x] file flush + sync before rename
  - [x] rename only after successful sync
  - [x] best-effort parent directory fsync after rename (warn on failure)
- [x] 2.4 Implement write consistency policy (`set_user_dictionary`: disk-write-first then memory swap).
- [x] 2.5 Implement serialized writes with dedicated `write_gate` and short in-memory swap lock.

## 3. Settings and Reset Boundary
- [x] 3.1 Remove `dictionary` from `AppSettings` in `src-tauri/src/settings.rs`.
- [x] 3.2 Remove settings-side dictionary migration logic.
- [x] 3.3 Update `reset_app_settings` in `src-tauri/src/commands/mod.rs` to reset settings only.

## 4. Command Surface and Registration
- [x] 4.1 Add `src-tauri/src/commands/dictionary.rs` (`get_user_dictionary`, `set_user_dictionary`).
- [x] 4.2 Remove legacy `update_custom_words` command from `src-tauri/src/shortcut/mod.rs`.
- [x] 4.3 Register new dictionary commands in both specta command lists in `src-tauri/src/lib.rs`.
- [x] 4.4 Remove legacy command from both specta command lists.

## 5. Runtime Consumer Refactor
- [x] 5.1 Update `src-tauri/src/managers/transcription.rs` to read dictionary from in-memory dictionary state.
- [x] 5.2 Update `src-tauri/src/managers/correction.rs` to read dictionary from in-memory dictionary state.
- [x] 5.3 Update `src-tauri/src/audio_toolkit/text.rs` import for `CustomWordEntry`.

## 6. Frontend State Split
- [x] 6.1 Remove dictionary handling from `src/stores/settingsStore.ts`.
- [x] 6.2 Add `src/stores/dictionaryStore.ts` and `src/hooks/useDictionary.ts`.
- [x] 6.3 Refactor `src/components/dictionary/DictionaryPage.tsx` and `src/components/shared/HistoryList.tsx` to dictionary store usage.
- [x] 6.4 Regenerate `src/bindings.ts` via specta export.

## 7. Verification
- [x] 7.1 Add/adjust Rust tests for missing/malformed/unsupported version load behavior.
- [x] 7.2 Add/adjust Rust tests for disk-write-first consistency and serialized write behavior.
- [x] 7.3 Add/adjust Rust test for dir-fsync warning semantics after successful rename.
- [x] 7.4 Run `cargo check` and targeted tests.
- [ ] 7.5 Run frontend checks/tests for dictionary CRUD and history alias flow.

## 8. Documentation
- [x] 8.1 Update `doc/custom-word-correction.md` for split storage behavior and hard-reset notes.
- [x] 8.2 Update `doc/settings.md` to clarify reset does not affect dictionary.
