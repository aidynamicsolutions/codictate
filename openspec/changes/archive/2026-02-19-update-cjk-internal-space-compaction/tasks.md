## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta for CJK internal-space compaction and mixed-script guardrails
- [x] 1.2 Validate with `openspec validate update-cjk-internal-space-compaction --strict`

## 2. Clipboard Smart Insertion Implementation
- [x] 2.1 Add language gate for CJK compaction (`zh`, `zh-tw`, `yue`)
- [x] 2.2 Add CJK Han and CJK punctuation classification helpers
- [x] 2.3 Add ASCII token helper for mixed-script spacing preservation
- [x] 2.4 Implement CJK internal-whitespace compaction helper with line-break/tab preservation
- [x] 2.5 Integrate compaction step into `prepare_text_for_paste` after punctuation normalization and before boundary spacing checks
- [x] 2.6 Extend smart insertion debug logging with compaction metadata fields

## 3. Verification
- [x] 3.1 Add unit tests for Chinese/Cantonese compaction behavior
- [x] 3.2 Add unit tests for sentence-boundary compaction behavior in Chinese
- [x] 3.3 Add unit tests for mixed-script spacing preservation (`Open AI`, URL-like/token-like phrases)
- [x] 3.4 Add regression test to confirm Japanese no-boundary-spacing behavior is unchanged
- [x] 3.5 Run targeted Rust tests for clipboard logic

## 4. Documentation
- [x] 4.1 Update manual test checklist with Chinese/Cantonese ASR internal-space compaction cases
- [x] 4.2 Add mixed-script preservation case to manual checklist
- [x] 4.3 Add log-verification checklist entries for compaction debug fields

## 5. Final Validation
- [x] 5.1 Validate `update-cjk-internal-space-compaction` with strict OpenSpec validation
- [x] 5.2 Validate whole OpenSpec workspace with strict validation
