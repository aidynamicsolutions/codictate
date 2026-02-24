# Custom Word Correction

Custom word correction automatically corrects or replaces transcribed words with user-defined alternatives. It is designed for proper nouns, brand names, domain-specific terminology, abbreviations, and phrases that ASR frequently mishears.

For a non-technical, UI-first walkthrough, see `doc/dictionary-user-guide.md`.

## Dictionary Entry Types

Each dictionary entry has four fields:

| Field            | Description                                                          |
| ---------------- | -------------------------------------------------------------------- |
| `input`          | Canonical word/phrase to recognize                                   |
| `aliases`        | Additional spoken variants for the same canonical term               |
| `replacement`    | What to output when a match is accepted                              |
| `is_replacement` | `true` = exact match only, `false` = fuzzy/phonetic matching enabled |

> [!NOTE]
> Legacy dictionary entries without `aliases` are supported. Missing `aliases` default to an empty list.

### Two Modes

| Mode            | `is_replacement`  | Toggle State | Matching                 | Use Case                                      |
| --------------- | ----------------- | ------------ | ------------------------ | --------------------------------------------- |
| **Vocabulary**  | `false` (default) | OFF          | Exact + Fuzzy + Phonetic | Learn terms and names (`shadcn`, `Anthropic`) |
| **Replacement** | `true`            | ON           | Case-insensitive exact   | Expand text (`btw` -> `by the way`)           |

> [!NOTE]
> **Fuzzy matching is the default.** Toggle ON "Replace with different text" for exact-only replacements.

### Case Adaptation for Replacements

Replacements adapt to input case pattern:

| Input | Output       | Case Pattern |
| ----- | ------------ | ------------ |
| `btw` | `by the way` | lowercase    |
| `Btw` | `By the way` | Title Case   |
| `BTW` | `BY THE WAY` | ALL CAPS     |

## "Best of Both Worlds" Algorithm

The matcher combines **N-gram analysis** with **phonetic and fuzzy matching** to support both single-token and split-token dictation.

### Matching Process

1. **Normalization**: Input and dictionary terms are normalized (case-insensitive, punctuation-tolerant matching).
2. **N-gram Sliding Window**:
   - Exact pass scans up to 1-8 word windows
   - Fuzzy pass scans up to 1-3 word windows
3. **Exact-first pass**: Canonical `input` and all `aliases` are checked before fuzzy matching.
4. **Fuzzy pass**:
   - **Split-token fuzzy path** for 2-3 word n-grams against single-token targets (strict guards + strict threshold)
   - **Standard fuzzy path** for regular matching
5. **Scoring stack**:
   - **Jaro-Winkler** for short words
   - **Damerau-Levenshtein** for longer words/transpositions
   - **Double Metaphone** for phonetic boost

### Multi-Word and Split-Token Support

| Transcription         | Entry                           | Match                          |
| --------------------- | ------------------------------- | ------------------------------ |
| `"super wisper"`      | `super whisper -> SuperWhisper` | ✅ standard fuzzy (multi-word) |
| `"shad cn"`           | `shadcn` with alias `shad cn`   | ✅ exact alias                 |
| `"shat cn component"` | `shadcn`                        | ✅ split-token fuzzy           |
| `"chef cn component"` | `shadcn`                        | ❌ rejected                    |

### Logic Flow

```mermaid
graph TD
    A[Input Text] --> B[Tokenize into Words]
    B --> C{Iterate Words}
    C --> D[Generate N-grams (exact 1-8, fuzzy 1-3)]
    D --> E{Exact Canonical/Alias Match?}
    E -- Yes --> F[Apply Replacement and Preserve Punctuation]
    E -- No --> G{is_replacement?}
    G -- Yes --> H[Skip Fuzzy for this entry]
    G -- No --> I{Split-token path? 2-3 tokens to single-token target}
    I -- Yes --> J{Split guards pass?}
    J -- No --> H
    J -- Yes --> K{Split score < split threshold?}
    K -- Yes --> F
    K -- No --> H
    I -- No --> L{Standard guards pass?}
    L -- No --> H
    L -- Yes --> M{Standard score < threshold?}
    M -- Yes --> F
    M -- No --> H
    F --> N[Advance Iterator past matched n-gram]
    H --> C
```

### Why this Hybrid Approach?

- N-grams alone help with segmented speech but miss many phonetic variants.
- Pure phonetic/fuzzy matching alone can over-trigger without n-gram constraints.
- Hybrid matching keeps high recall for hard terms while maintaining guardrails against false positives.

## Configuration

| Setting                           | Default | Description                                               |
| --------------------------------- | ------- | --------------------------------------------------------- |
| `dictionary`                      | `[]`    | List of `CustomWordEntry` objects                         |
| `word_correction_threshold`       | `0.18`  | Standard fuzzy acceptance threshold (lower = stricter)    |
| `word_correction_split_threshold` | `0.14`  | Split-token fuzzy threshold (stricter than standard path) |

## Examples

| Transcription         | Entry (input/aliases -> replacement)        | `is_replacement` | Match Type                | Result               |
| --------------------- | ------------------------------------------- | ---------------- | ------------------------- | -------------------- |
| `"chat gpt"`          | `ChatGPT`, aliases: `chat gpt` -> `ChatGPT` | `false`          | Exact alias               | `"ChatGPT"`          |
| `"shad c n?"`         | `shadcn`, aliases: `shad c n` -> `shadcn`   | `false`          | Exact alias + punctuation | `"shadcn?"`          |
| `"Shat CN component"` | `shadcn` -> `shadcn`                        | `false`          | Split-token fuzzy         | `"Shadcn component"` |
| `"Anthrapik"`         | `Anthropic` -> `Anthropic`                  | `false`          | Standard fuzzy            | `"Anthropic"`        |
| `"btw"`               | `btw` -> `by the way`                       | `true`           | Exact replacement         | `"by the way"`       |

## Debug Logging

Enable DEBUG log level to see detailed matching decisions.

```bash
grep -E "\[CustomWords\]" $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)
```

Reason-coded decisions include:

- `exact_alias_match`
- `exact_canonical_match`
- `skip_guard_word`
- `skip_short_input`
- `skip_word_count`
- `skip_length_ratio`
- `skip_short_prefix_extension`
- `reject_score`
- `accept_split_fuzzy`
- `accept_standard_fuzzy`

Per-session summary logs include:

- `candidates_checked`
- `exact_hits`
- `split_fuzzy_hits`
- `standard_fuzzy_hits`
- `reject_counts` by reason

## Technical Details

- **Matcher**: `src-tauri/src/audio_toolkit/text.rs`
- **Settings schema**: `src-tauri/src/settings.rs`
- **Dictionary UI**: `src/components/dictionary/DictionaryEntryModal.tsx`
- **Dictionary list/search**: `src/components/dictionary/DictionaryPage.tsx`

## Known Limitations

| Limitation      | Impact                                                                                   |
| --------------- | ---------------------------------------------------------------------------------------- |
| N-gram size     | Exact matching scans up to 8 words; fuzzy matching scans up to 3 words                   |
| Rescoring       | N-best/lattice rescoring deferred until engines expose alternatives consistently         |
| False positives | Controlled by guard words, minimum length, length ratio, and split-threshold constraints |

## UI Features

| Feature             | Description                                                                          |
| ------------------- | ------------------------------------------------------------------------------------ |
| Alias editing       | Add multiple spoken variants per canonical term                                      |
| Duplicate detection | Prevents collisions across canonical inputs and aliases                              |
| Search coverage     | Search includes input, replacement, and aliases                                      |
| Character limits    | Input: 100 chars, Replacement: 300 chars, Alias input: 100 chars each, max 8 aliases |
| Auto-grow textarea  | Input fields expand up to 3 lines                                                    |
