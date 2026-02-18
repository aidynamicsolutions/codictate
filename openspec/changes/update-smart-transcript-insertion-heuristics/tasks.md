## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta with modified punctuation sanitation and sentence-boundary spacing requirements
- [x] 1.2 Validate with `openspec validate update-smart-transcript-insertion-heuristics --strict`

## 2. Accessibility Context Metadata
- [x] 2.1 Extend `TextInsertionContext` with `right_non_whitespace_char`
- [x] 2.2 Populate `right_non_whitespace_char` in macOS insertion context capture
- [x] 2.3 Update accessibility unit tests for new right-side metadata

## 3. Smart Insertion Heuristic Refinements
- [x] 3.1 Add conservative trailing `.?!` sanitation for selection-replace only
- [x] 3.2 Add abbreviation guard for punctuation sanitation
- [x] 3.3 Add sentence-punctuation leading-space rule after `.?!` when inserting word-like text
- [x] 3.4 Keep existing word-boundary spacing and fallback semantics

## 4. Observability and UX Copy
- [x] 4.1 Add debug fields for punctuation-strip decision and leading-space reason
- [x] 4.2 Update English setting description to reflect punctuation sanitation and sentence-boundary spacing

## 5. Verification
- [x] 5.1 Add/extend clipboard tests for punctuation sanitation and sentence-boundary spacing cases
- [x] 5.2 Run targeted Rust tests for clipboard and macOS accessibility modules
