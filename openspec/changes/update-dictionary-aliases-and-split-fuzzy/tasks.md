## 1. OpenSpec Deltas
- [x] 1.1 Add custom-word-correction spec delta for canonical aliases and split-token fuzzy matching
- [x] 1.2 Add observability spec delta for reason-coded dictionary logging and summary counters
- [x] 1.3 Validate the OpenSpec change with strict mode

## 2. Backend Schema and Settings
- [x] 2.1 Add `aliases` to `CustomWordEntry`
- [x] 2.2 Add `word_correction_split_threshold` to `AppSettings` and defaults
- [x] 2.3 Wire split threshold into transcription correction call path
- [x] 2.4 Keep no legacy compatibility behavior for dictionary entry schema

## 3. Matching Algorithm
- [x] 3.1 Build effective candidate set from canonical input + aliases
- [x] 3.2 Enforce exact alias/canonical precedence before fuzzy
- [x] 3.3 Implement constrained split-token fuzzy path for 2-3 token n-grams to single-token targets
- [x] 3.4 Add internal single-hypothesis abstraction for future rescoring integration
- [x] 3.5 Preserve existing case and punctuation behavior

## 4. Observability
- [x] 4.1 Add structured reason-coded logs for accept/reject paths
- [x] 4.2 Add per-session summary counters for candidate checks and match/reject counts
- [x] 4.3 Ensure log fields include path, reason, score, threshold, n-gram, and canonical/alias source

## 5. Frontend Dictionary UX
- [x] 5.1 Add alias editing support in dictionary modal
- [x] 5.2 Add alias-aware duplicate validation and normalization
- [x] 5.3 Add alias visibility and search coverage in dictionary list
- [x] 5.4 Update frontend type bindings for new dictionary/settings fields

## 6. Testing
- [x] 6.1 Add/update backend tests for alias exact matches and punctuation cases
- [x] 6.2 Add/update backend tests for split-token fuzzy success (`Shat CN`) and rejection (`Chef CN`)
- [x] 6.3 Add/update regression tests for existing behavior (`Chat GPT`, stop-word protection)
- [x] 6.4 Add/update frontend tests for alias duplicate checks

## 7. Documentation
- [x] 7.1 Update `doc/custom-word-correction.md` with reliable setup examples
- [x] 7.2 Add debugging workflow using reason-coded logs and session filtering
