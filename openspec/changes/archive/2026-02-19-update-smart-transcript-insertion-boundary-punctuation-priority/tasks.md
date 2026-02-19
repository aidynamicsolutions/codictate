## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta for conflicting boundary punctuation priority in whitespace profiles
- [x] 1.2 Validate with `openspec validate update-smart-transcript-insertion-boundary-punctuation-priority --strict`

## 2. Clipboard Smart Insertion Implementation
- [x] 2.1 Keep same-mark duplicate boundary collapse unchanged
- [x] 2.2 Add conflicting-mark boundary collapse for whitespace profiles that prefers right-boundary punctuation
- [x] 2.3 Keep conservative and no-boundary-spacing profile behavior unchanged
- [x] 2.4 Add explicit debug reason for conflicting-mark collapse

## 3. Verification
- [x] 3.1 Replace differing-mark unit expectation to assert right-boundary priority
- [x] 3.2 Add cased conflicting-mark variants (`!` vs `?`, `.` vs `?`)
- [x] 3.3 Add uncased conflicting-mark variant (`؟` vs `.`)
- [x] 3.4 Add no-boundary-spacing regression to ensure no behavior change
- [x] 3.5 Run targeted and full Rust test suites

## 4. Documentation
- [x] 4.1 Update manual checklist with cased and uncased conflict cases
- [x] 4.2 Add explicit final-pass note for right-boundary punctuation priority
- [x] 4.3 Update `doc/smart-insertion-notes.md` with policy and validation entry

## 5. Final Validation
- [x] 5.1 Validate `update-smart-transcript-insertion-boundary-punctuation-priority` with strict OpenSpec validation
- [x] 5.2 Validate whole OpenSpec workspace with strict validation
