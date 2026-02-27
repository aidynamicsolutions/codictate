## 1. OpenSpec
- [x] 1.1 Add `custom-word-correction` spec delta for exact-default, fuzzy opt-in, short-target guard, and legacy migration compatibility
- [x] 1.2 Validate the change with `openspec validate update-dictionary-precision-defaults --strict`

## 2. Backend Schema and Migration
- [x] 2.1 Add `fuzzy_enabled: Option<bool>` to `CustomWordEntry`
- [x] 2.2 Add a shared dictionary normalization module used by matcher and migration
- [x] 2.3 Implement dictionary migration helper with deterministic `Option<bool>` upgrade/coercion rules
- [x] 2.4 Run migration helper in both settings load paths and persist updates when changed

## 3. Matcher Hardening
- [x] 3.1 Require `fuzzy_enabled == Some(true)` to enter fuzzy path
- [x] 3.2 Add hard guard for single-word fuzzy targets with normalized length `<= 4`
- [x] 3.3 Add reason-coded logs for fuzzy-disabled and short-target rejections
- [x] 3.4 Keep exact canonical/alias behavior unchanged

## 4. Frontend and Types
- [x] 4.1 Add advanced fuzzy toggle in dictionary modal (default OFF for new vocabulary entries)
- [x] 4.2 Preserve exact-only semantics for replacement entries
- [x] 4.3 Keep dictionary duplicate/identity logic unchanged with respect to fuzzy toggle
- [x] 4.4 Regenerate `src/bindings.ts` via tauri-specta export flow

## 5. Documentation
- [x] 5.1 Update `doc/dictionary-user-guide.md` to state exact+aliases default, fuzzy opt-in, and short-word fuzzy block
- [x] 5.2 Update `doc/custom-word-correction.md` to align algorithm and configuration docs with shipped behavior

## 6. Verification
- [x] 6.1 Add/adjust backend tests for migration and short-target fuzzy guard
- [x] 6.2 Add/adjust backend regression test ensuring `went -> qwen` does not fuzzy-match
- [x] 6.3 Run targeted tests and report outcomes
