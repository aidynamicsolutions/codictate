# Custom Word Correction

Custom word correction automatically replaces transcribed words with user-defined alternatives, useful for proper nouns, brand names, and domain-specific terminology.

## How It Works

The algorithm uses three techniques to find matches:

1. **Exact Match** — Case-insensitive exact match checked first (highest priority)
2. **Multi-Word Phrase Match** — Consecutive words combined to match compound terms (e.g., "chat GPT" → "ChatGPT")
3. **Fuzzy Match** — Single-word matching using:
   - Double Metaphone (phonetic similarity)
   - Damerau-Levenshtein (edit distance with transpositions)

### Matching Process

```
Input words → Try single exact match? → Yes → Replace
                                      ↓ No
                                      → Try multi-word exact match (up to 3 words)?
                                      ↓ Yes → Replace entire phrase
                                      ↓ No
                                      → Fuzzy match (phonetic + edit distance)
                                      → Score < threshold? → Replace
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `custom_words` | `[]` | List of words to match against |
| `word_correction_threshold` | `0.18` | Lower = stricter matching (0.0–1.0) |

## Examples

| Input | Custom Word | Result | Match Type |
|-------|-------------|--------|------------|
| "handy" | "Handy" | "Handy" | Single-word exact |
| "chat GPT" | "ChatGPT" | "ChatGPT" | Multi-word exact |
| "teh" | "the" | "the" | Edit distance (transposition) |
| "kat" | "cat" | "cat" | Phonetic |
| "Anthrapik" | "Anthropic" | "Anthropic" | Phonetic + edit |

## Technical Details

- **Location**: `src-tauri/src/audio_toolkit/text.rs`
- **Phonetic Algorithm**: Double Metaphone (via `rphonetic` crate)
- **Edit Distance**: Damerau-Levenshtein for long words, Jaro-Winkler for short (via `strsim` crate)
- **Max window size**: 3 words (for multi-word matching)

### Multi-Word Matching

Multi-word matching only uses **exact matching** to prevent false positives:
- "chat GPT" → "chatgpt" matches "ChatGPT" ✅
- "at Anthropic" does NOT match "Anthropic" (not exact) ✅

### Hybrid String Similarity

| Word Length | Algorithm | Why |
|-------------|-----------|-----|
| ≤6 chars (similar lengths) | **Jaro-Winkler** | Better prefix matching for names |
| >6 chars | **Damerau-Levenshtein** | Better transposition handling |

### Why Double Metaphone?

Double Metaphone (1990) improves on Soundex (1918) with:
- Dual phonetic codes for better coverage
- Multilingual support (not English-only)
- Lower false positive rate

### Why Damerau-Levenshtein?

Treats transpositions ("teh" → "the") as single edits instead of two, better matching common speech-to-text errors.

## Known Limitations

| Limitation | Reason | Impact |
|------------|--------|--------|
| **English-optimized phonetics** | Double Metaphone designed for Latin/English | Low — STT outputs primarily ASCII |
| **No diacritic normalization** | "café" stays as-is, not normalized to "cafe" | Low — STT typically strips accents |
| **Case from first word only** | Multi-word matches use first word's case pattern | Acceptable for most use cases |
| **Max 3-word phrases** | Performance trade-off | Increase `max_window_size` if needed |
