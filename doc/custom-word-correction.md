# Custom Word Correction

Custom word correction automatically replaces transcribed words with user-defined alternatives. It is designed to handle proper nouns, brand names, and domain-specific terminology that speech-to-text models often misinterpret.

## "Best of Both Worlds" Algorithm

We use a hybrid approach that combines **N-gram analysis** (from `main`) with **Advanced Phonetic/Fuzzy Matching** (from `llm`). This allows us to correct both single words and split multi-word phrases even when they are phonetically matched rather than exactly matched.

### Matching Process

1.  **Normalization**: Input text and custom words are normalized (lowercase, punctuation stripped).
2.  **N-gram Sliding Window**: We scan the input text using a sliding window of 1 to 3 words. This allows us to catch phrases like "Chat G P T" and map them to "ChatGPT".
3.  **Hybrid Matching**: For each n-gram, we attempt to match it against your Custom Words using three prioritized techniques:
    *   **Exact Match**: Highest priority. (e.g., "handy" -> "Handy")
    *   **Phonetic Match**: Uses **Double Metaphone** to catch "sounds-like" errors. (e.g., "chat jepity" -> "ChatGPT")
    *   **Fuzzy Match**: Uses string similarity algorithms:
        *   **Jaro-Winkler**: For short words (≤6 chars) to prioritize prefix matching.
        *   **Damerau-Levenshtein**: For longer words to handle transpositions and typos.

### Logic Flow

```mermaid
graph TD
    A[Input Text] --> B[Tokenize into Words]
    B --> C{Iterate Words}
    C --> D[Generate N-grams (1-3 words)]
    D --> E{Match Found?}
    E -- Exact --> F[Replace with Custom Word]
    E -- Phonetic --> F
    E -- Fuzzy --> F
    E -- No Match --> G[Next N-gram]
    F --> H[Advance Iterator past N-gram]
    G --> H
    H --> C
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `custom_words` | `[]` | List of words to match against |
| `word_correction_threshold` | `0.18` | Lower = stricter matching (0.0–1.0) |

## Examples

| Input Transcription | Custom Word | Match Type | Result |
|---------------------|-------------|------------|--------|
| "handy"             | "Handy"     | Exact      | "Handy" |
| "chat G P T"        | "ChatGPT"   | N-gram + Exact | "ChatGPT" |
| "chat jepity"       | "ChatGPT"   | N-gram + Phonetic | "ChatGPT" |
| "Anthrapik"         | "Anthropic" | Fuzzy (Levenshtein) | "Anthropic" |
| "teh"               | "the"       | Fuzzy (Transposition) | "the" |

## Technical Details

- **Location**: `src-tauri/src/audio_toolkit/text.rs`
- **Phonetic Algorithm**: Double Metaphone (via `rphonetic`)
- **String Similarity**: Jaro-Winkler & Damerau-Levenshtein (via `strsim`)
- **Key Config**: `SHORT_WORD_THRESHOLD = 6` (Switch point for similarity algos)

### Why this Hybrid Approach?

- **N-grams** alone (previous technique) handled "Chat G P T" well but failed on "Anthrapik" if not exact.
- **Phonetic/Fuzzy** alone (previous technique) handled "Anthrapik" well but failed on split phrases like "Chat G P T".
- **Hybrid** handles **BOTH**: It can take "Chat G P T", combine it into a candidate string, and then phonetically match that candidate against "ChatGPT".

## Known Limitations

| Limitation | Impact |
|------------|--------|
| **Performance** | N-gram analysis is O(N*M) where M is window size (3). Negligible for typical transcription lengths. |
| **Over-correction** | Aggressive phonetic matching *can* produce false positives. Adjust `threshold` if needed. |
