## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta for Japanese internal-space compaction behavior and mixed-script guardrails
- [x] 1.2 Validate with `openspec validate update-japanese-internal-space-compaction --strict`

## 2. Clipboard Smart Insertion Implementation
- [x] 2.1 Add internal compaction strategy routing (`HanBoundaryOnly`, `JapaneseMixedScript`)
- [x] 2.2 Keep existing Han/CJK compaction logic unchanged for `zh`/`zh-tw`/`yue`
- [x] 2.3 Add Japanese script classification helpers and boundary predicate
- [x] 2.4 Extend compaction helper signature to accept a strategy
- [x] 2.5 Implement Japanese compaction rule (`Japanese↔Japanese`, `ASCII↔Japanese`, preserve `ASCII↔ASCII`)
- [x] 2.6 Preserve structural whitespace and emoji guard behavior

## 3. Verification
- [x] 3.1 Replace Japanese no-compaction regression with Japanese compaction coverage
- [x] 3.2 Add Japanese tests for mixed script, numeric-unit, line-break preservation, emoji spacing, and halfwidth katakana
- [x] 3.3 Re-run existing Chinese/Cantonese compaction regression tests
- [x] 3.4 Run full `clipboard::tests` suite
- [x] 3.5 Run `cargo test --manifest-path src-tauri/Cargo.toml --lib`

## 4. Documentation
- [x] 4.1 Update manual checklist with Japanese NBS compaction cases
- [x] 4.2 Update log verification criteria to include Japanese compaction checks
- [x] 4.3 Update final pass criteria for Japanese artifact-space cleanup
- [x] 4.4 Update maintainer notes with scope, rationale, and verification commands

## 5. Final Validation
- [x] 5.1 Validate `update-japanese-internal-space-compaction` with strict OpenSpec validation
- [x] 5.2 Validate whole OpenSpec workspace with strict validation
