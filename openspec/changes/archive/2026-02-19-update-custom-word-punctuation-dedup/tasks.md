## 1. OpenSpec Deltas
- [x] 1.1 Add `custom-word-correction` delta for trailing punctuation deduplication during replacement merge
- [x] 1.2 Validate with `openspec validate update-custom-word-punctuation-dedup --strict`

## 2. Custom Word Merge Implementation
- [x] 2.1 Add helper to detect sentence punctuation marks in replacement output
- [x] 2.2 Trim duplicate leading suffix punctuation when replacement already ends with the same terminal mark
- [x] 2.3 Apply helper in exact and fuzzy custom-word match merge paths

## 3. Verification
- [x] 3.1 Add exact alias replacement test for `e g.` -> `e.g.`
- [x] 3.2 Add vocabulary alias replacement test for `e g.` -> `e.g.`
- [x] 3.3 Add non-duplicate punctuation regression test to preserve `e.g.?`
- [x] 3.4 Run targeted Rust tests for custom word correction
- [x] 3.5 Run broader Rust test suite

## 4. Documentation
- [x] 4.1 Update `doc/smart-insertion-notes.md` with punctuation dedup regression context

## 5. Final Validation
- [x] 5.1 Validate `update-custom-word-punctuation-dedup` with strict OpenSpec validation
- [x] 5.2 Validate whole OpenSpec workspace with strict validation
