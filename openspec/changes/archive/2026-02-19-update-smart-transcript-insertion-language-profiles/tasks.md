## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta describing language normalization, profile routing, and conservative fallback behavior
- [x] 1.2 Validate with `openspec validate update-smart-transcript-insertion-language-profiles --strict`

## 2. Phase 1 Safety Hardening
- [x] 2.1 Replace ASCII-only digit continuation check with Unicode numeric check in smart punctuation strip logic
- [x] 2.2 Add selected-language normalization and low-risk deterministic routing into smart insertion formatting
- [x] 2.3 Ensure conservative language modes (`auto`, unknown, Turkish) use boundary-safe trailing-space fallback with legacy behavior when context is unavailable
- [x] 2.4 Keep existing cased-language smart insertion behavior unchanged

## 3. Phase 2 Profile Heuristics
- [x] 3.1 Introduce internal smart insertion profile enum (`CasedWhitespace`, `UncasedWhitespace`, `NoBoundarySpacing`, `Conservative`)
- [x] 3.2 Add profile mapping and profile-aware sentence punctuation classification
- [x] 3.3 Apply casing only for cased profile, boundary spacing only for spacing-enabled profiles
- [x] 3.4 Add non-ASCII punctuation handling for mapped Arabic/CJK profiles

## 4. UX Copy Consistency
- [x] 4.1 Update English Smart Insertion copy with explicit conservative fallback disclosure (`auto`, Turkish, unsupported, no-context)
- [x] 4.2 Surface a fallback-scope note in the Smart Insertion setting UI (locale fallback to English when key is missing)
- [x] 4.3 Mirror the Smart Insertion fallback-scope copy in embedded Rust English locale resources
- [x] 4.4 Document runtime source-of-truth policy (code authoritative, prompt markdown reference-only)

## 5. Verification
- [x] 5.1 Add/extend unit tests for language normalization, profile mapping, unicode digits, and profile-specific behavior
- [x] 5.2 Run targeted Rust tests for clipboard, accessibility, and actions modules
- [x] 5.3 Run OpenSpec strict validation for the change and for all items
- [x] 5.4 Add explicit Turkish conservative fallback unit coverage
