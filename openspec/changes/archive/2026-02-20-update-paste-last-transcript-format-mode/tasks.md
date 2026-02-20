## 1. Backend
- [x] 1.1 Add `paste_last_use_smart_insertion: bool` to `AppSettings` with default `false`.
- [x] 1.2 Add `change_paste_last_use_smart_insertion_setting` command and register it.
- [x] 1.3 Add paste preparation mode support (`Adaptive` vs `Literal`) in shared clipboard paste code.
- [x] 1.4 Switch `paste_last_transcript` to use literal mode by default and adaptive mode when setting is enabled.
- [x] 1.5 Keep transcribe/refine flow behavior unchanged.

## 2. Frontend
- [x] 2.1 Wire new setting updater in settings store.
- [x] 2.2 Add Advanced settings toggle near paste settings.
- [x] 2.3 Add translation keys for all locales.

## 3. Testing
- [x] 3.1 Add/extend Rust tests for preparation mode behavior, setting defaults, and action-mode selection.
- [x] 3.2 Add/extend frontend tests for setting helper logic.
- [x] 3.3 Run `cargo test` and `bun run test`.

## 4. Docs and Spec
- [x] 4.1 Update smart insertion notes and manual checklist.
- [x] 4.2 Add transcript-insertion spec delta.
- [x] 4.3 Validate OpenSpec change and full spec set with strict mode.
