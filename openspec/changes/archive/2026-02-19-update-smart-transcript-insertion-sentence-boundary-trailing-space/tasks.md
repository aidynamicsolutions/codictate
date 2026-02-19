## 1. OpenSpec Deltas
- [x] 1.1 Add `transcript-insertion` delta for sentence-boundary trailing space in whitespace profiles
- [x] 1.2 Validate with `openspec validate update-smart-transcript-insertion-sentence-boundary-trailing-space --strict`

## 2. Clipboard Smart Insertion Implementation
- [x] 2.1 Add dedicated trailing-space checks for word-boundary and sentence-boundary conditions
- [x] 2.2 Add sentence-boundary trailing-space guard that requires a word-like start token
- [x] 2.3 Keep conservative and no-boundary-spacing branches unchanged
- [x] 2.4 Extend smart insertion debug logging with trailing-space reason metadata

## 3. Verification
- [x] 3.1 Update existing unit tests for preserved punctuation + right-word continuation
- [x] 3.2 Add regression tests for punctuation-only inserts and right-whitespace boundaries
- [x] 3.3 Add regression tests proving conservative and no-boundary-spacing behavior is unchanged
- [x] 3.4 Run targeted Rust tests for clipboard logic

## 4. Documentation
- [x] 4.1 Update manual checklist expectations for CWS-07, CWS-08, SEL-02, and UWS-04
- [x] 4.2 Add explicit punctuation-only guard case to the manual checklist

## 5. Final Validation
- [x] 5.1 Validate `update-smart-transcript-insertion-sentence-boundary-trailing-space` with strict OpenSpec validation
- [x] 5.2 Validate whole OpenSpec workspace with strict validation
