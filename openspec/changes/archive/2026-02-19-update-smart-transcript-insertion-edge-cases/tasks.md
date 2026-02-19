## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta with no-selection punctuation sanitation, duplicate boundary collapse, and undo consistency updates
- [x] 1.2 Validate with `openspec validate update-smart-transcript-insertion-edge-cases --strict`

## 2. Smart Insertion Edge-Case Logic
- [x] 2.1 Extend trailing `.?!` sanitation to selection and no-selection continuation cases
- [x] 2.2 Keep abbreviation guard and lowercase/digit continuation constraints
- [x] 2.3 Add duplicate boundary `.?!` collapse for same-mark right-boundary punctuation
- [x] 2.4 Preserve existing smart insertion fallback/casing/spacing behavior

## 3. Undo Payload Consistency
- [x] 3.1 Keep `undo_last_transcript` shortcut behavior unchanged
- [x] 3.2 Ensure transformed paste output is the value registered for undo tracking
- [x] 3.3 Add unit-level assertion for undo capture payload plumbing

## 4. UX Copy and Verification
- [x] 4.1 Update English Smart Insertion description for continuation cleanup and duplicate punctuation collapse
- [x] 4.2 Add/adjust clipboard unit tests for no-selection sanitation and duplicate boundary collapse
- [x] 4.3 Run targeted Rust tests covering clipboard, actions, and macOS accessibility modules
