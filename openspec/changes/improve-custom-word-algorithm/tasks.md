# Tasks: Improve Custom Word Correction Algorithm

## 1. Research & Preparation

- [ ] 1.1 Evaluate Rust crates for Double Metaphone (`phonetics` vs `phonetic`)
- [ ] 1.2 Verify `strsim::damerau_levenshtein` function availability in current version
- [ ] 1.3 Review existing unit tests in `src-tauri/src/audio_toolkit/text.rs`

## 2. Implementation

### 2.1 Exact Match Check (Priority 1)

- [ ] 2.1.1 Add exact case-insensitive match check before fuzzy matching loop
- [ ] 2.1.2 Return early with preserved case when exact match found
- [ ] 2.1.3 Add unit test for exact match behavior

### 2.2 Damerau-Levenshtein (Priority 2)

- [ ] 2.2.1 Replace `strsim::levenshtein` with `strsim::damerau_levenshtein`
- [ ] 2.2.2 Update existing unit tests to verify transposition handling
- [ ] 2.2.3 Add unit test for transposition case ("teh" → "the")

### 2.3 Double Metaphone (Priority 1)

- [ ] 2.3.1 Add phonetic crate dependency to `Cargo.toml`
- [ ] 2.3.2 Replace `soundex` function call with Double Metaphone
- [ ] 2.3.3 Update phonetic matching logic to use primary and secondary codes
- [ ] 2.3.4 Update scoring logic to handle dual-code matching
- [ ] 2.3.5 Add unit test for phonetic matching with Double Metaphone

## 3. Testing & Verification

- [ ] 3.1 Run existing unit tests to ensure no regressions
- [ ] 3.2 Run `cargo test audio_toolkit::text` to verify all text module tests pass
- [ ] 3.3 Build project with `cargo build --release`
- [ ] 3.4 Manual testing: add custom words and verify correction behavior

## 4. Documentation

- [ ] 4.1 Update inline documentation for `apply_custom_words` function
- [ ] 4.2 Document algorithm changes in code comments

---

## Verification Plan

### Automated Tests

Run the existing and new unit tests:

```bash
cd src-tauri
cargo test audio_toolkit::text --release
```

Expected: All tests pass including:
- `test_apply_custom_words_exact_match`
- `test_apply_custom_words_fuzzy_match`
- `test_preserve_case_pattern`
- `test_extract_punctuation`
- `test_empty_custom_words`
- NEW: `test_exact_match_before_fuzzy`
- NEW: `test_transposition_handling`
- NEW: `test_double_metaphone_phonetic`

### Manual Testing

1. **Start the app** with `bun run tauri dev`
2. **Add custom words** in Settings → Advanced → Custom Words:
   - Add "Handy"
   - Add "Anthropic"
   - Add "ChatGPT"
3. **Record a transcription** and say:
   - "I use Handy for speech to text" (test exact match for "Handy")
   - "I'm working with Anthropic Claude" (test phonetic match)
   - "Open chat GPT and ask a question" (test spacing/compound word)
4. **Verify** the custom words are correctly substituted in the output

### Dependency Check

```bash
cd src-tauri
cargo tree -i strsim
```

Verify `strsim` version supports `damerau_levenshtein`.
