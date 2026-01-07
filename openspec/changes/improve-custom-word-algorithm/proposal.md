# Change: Improve Custom Word Correction Algorithm

## Why

The current custom word correction implementation uses **Soundex** for phonetic matching and basic **Levenshtein distance** for string similarity. Research and community feedback (Reddit, StackOverflow, industry best practices) indicate these algorithms have limitations:

| Current | Issue | Improvement |
|---------|-------|-------------|
| **Soundex** (1918) | Outdated, high false positives, English-only | **Double Metaphone** — modern, multilingual, lower false positives |
| **Levenshtein** | Treats transpositions as 2 edits | **Damerau-Levenshtein** — transpositions count as 1 edit |
| **No exact check** | Fuzzy matching even for exact matches | **Exact match first** — skip fuzzy for case-insensitive exact matches |

These improvements will increase correction accuracy while maintaining the same user-facing behavior.

## Research Validation

| Finding | Source |
|---------|--------|
| "Soundex is quite primitive, developed for manual calculation" | StackOverflow |
| "Double Metaphone recommended for general-purpose phonetic matching" | Medium, Experian documentation |
| "Over 80% of spelling errors are single-error types including transpositions" | Wikipedia (Damerau's research) |
| "Damerau-Levenshtein preferred for typing accuracy, transpositions are frequent" | StackOverflow, Reddit |
| "Exact match check before fuzzy prevents false positives" | Reddit r/LanguageTechnology |
| "`strsim` crate already provides `damerau_levenshtein`" | docs.rs/strsim |

## What Changes

### Improvement 1: Exact Match Before Fuzzy

Before running the expensive fuzzy matching algorithm, check if the word exactly matches a custom word (case-insensitive). This prevents unnecessary computation and eliminates false positives where an exact match would be replaced with a phonetically similar but incorrect word.

### Improvement 2: Soundex → Double Metaphone

Replace the Soundex phonetic algorithm with Double Metaphone:
- Generates two phonetic codes (primary and alternate) for better coverage
- Handles non-English names and words
- Accounts for spelling variations and irregular pronunciations
- Significantly lower false positive rate

### Improvement 3: Levenshtein → Damerau-Levenshtein

Replace basic Levenshtein distance with Damerau-Levenshtein:
- Adds transposition (swapping adjacent characters) as a single edit operation
- "teh" → "the" is 1 edit instead of 2
- Better reflects common speech-to-text errors and typos
- Already available in the `strsim` crate we use

## Impact

- **Affected code:**
  - `src-tauri/src/audio_toolkit/text.rs` — Main algorithm implementation
  - `Cargo.toml` — May need new phonetic crate dependency

- **New dependencies:**
  - [`phonetics`](https://crates.io/crates/phonetics) or [`phonetic`](https://crates.io/crates/phonetic) — Double Metaphone implementation
  - No new deps for Damerau-Levenshtein (`strsim` already has it)

- **No breaking changes:**
  - Same public API (`apply_custom_words`)
  - Same settings (`custom_words`, `word_correction_threshold`)
  - Same user experience

- **Performance:**
  - Exact match check adds O(n) lookup but saves fuzzy computation for exact matches
  - Double Metaphone slightly more complex than Soundex but negligible for word-level processing
  - Damerau-Levenshtein nearly identical performance to Levenshtein

## Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| Which phonetic crate? | Evaluate `phonetics` vs `phonetic` | Check which has Double Metaphone support and is maintained |
| Threshold adjustment? | Keep same 0.18 default | Algorithm changes shouldn't require threshold changes |
| Fallback if no phonetic match? | Continue with edit distance only | Same as current behavior |
