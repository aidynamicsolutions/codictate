## 1. OpenSpec Deltas
- [x] 1.1 Add new `transcript-insertion` spec delta covering sentence-aware casing and boundary spacing
- [x] 1.2 Validate change with `openspec validate add-smart-transcript-insertion --strict`

## 2. Accessibility Context Capture
- [x] 2.1 Add `TextInsertionContext` type to accessibility module exports
- [x] 2.2 Add `capture_insertion_context` API with macOS implementation and non-macOS stub
- [x] 2.3 Capture boundary context from AX focused element/value/range without clipboard fallback
- [x] 2.4 Add unit tests for boundary extraction helper logic

## 3. Clipboard Formatting Pipeline
- [x] 3.1 Replace unconditional trailing-space logic with `prepare_text_for_paste`
- [x] 3.2 Implement sentence-aware casing adjustments for first alphabetic character
- [x] 3.3 Implement smart boundary spacing for left/right word boundaries
- [x] 3.4 Preserve legacy trailing-space fallback when context is unavailable
- [x] 3.5 Add debug logs for context presence, casing action, spacing action, and fallback path

## 4. Compatibility and UX Copy
- [x] 4.1 Keep `append_trailing_space` setting key and backend command unchanged
- [x] 4.2 Update English settings copy to describe smart insertion behavior

## 5. Verification
- [x] 5.1 Add/extend unit tests for smart insertion casing and spacing scenarios
- [x] 5.2 Run targeted Rust tests for accessibility/clipboard modules
