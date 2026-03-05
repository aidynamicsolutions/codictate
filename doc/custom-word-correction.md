# Custom Word Correction

Custom word correction is a post-recognition text correction layer.

It applies dictionary rules to transcript text after ASR inference. It is designed for proper nouns, brand names, domain-specific terminology, abbreviations, and repeated phrase mistakes.

It is not a guaranteed decoder-time biasing system for all pronunciations.

For a non-technical, UI-first walkthrough, see `doc/dictionary-user-guide.md`.

## Dictionary Entry Types

Each dictionary entry has five fields:

| Field            | Description                                                          |
| ---------------- | -------------------------------------------------------------------- |
| `input`          | Canonical word/phrase to recognize                                   |
| `aliases`        | Additional spoken variants for the same canonical term               |
| `replacement`    | What to output when a match is accepted                              |
| `is_replacement` | `true` = replacement mode (exact-only), `false` = vocabulary mode    |
| `fuzzy_enabled`  | Per-entry fuzzy opt-in for vocabulary mode (`true` = allow fuzzy)    |

> [!NOTE]
> Legacy dictionary entries without `aliases` are supported. Missing `aliases` default to an empty list.

### Two Modes

| Intent UI                    | `is_replacement` | Matching                        | Use Case                                         |
| --------------------------- | ---------------- | ------------------------------- | ------------------------------------------------ |
| **Recognize this term**     | `false`          | Exact canonical + exact aliases | Learn terms and names (`shadcn`, `Anthropic`)    |
| **Replace spoken phrase**   | `true`           | Case-insensitive exact          | Expand text (`btw` -> `by the way`)              |

> [!NOTE]
> Fuzzy matching is not default. In vocabulary mode, fuzzy is evaluated only when `fuzzy_enabled = true`.

> [!IMPORTANT]
> Single-word targets (canonical input or active alias) with normalized character length `<= 4` are hard-blocked from fuzzy matching even if `fuzzy_enabled = true`.

## Scope and Guarantees

Guaranteed behavior:

- Exact canonical and exact alias matching in vocabulary mode
- Exact phrase mapping in replacement mode

Not guaranteed:

- Forcing ASR to output a target term from any pronunciation
- Correcting every close-sounding single-word miss automatically

Design principle:

- Precision-first correction to avoid harmful false positives in common language

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
   - Skip unless `fuzzy_enabled = true` on the entry
   - Hard-block short single-word targets (`normalized canonical or alias character length <= 4`)
   - **Split-token fuzzy path** for 2-3 word n-grams against single-token targets (strict guards + strict threshold)
   - **Standard fuzzy path** for regular matching
5. **Scoring stack**:
   - **Jaro-Winkler** for short words
   - **Damerau-Levenshtein** for longer words/transpositions
   - **Double Metaphone** for phonetic boost

## Practical Policy for Mispronunciation Cases

For user-reported misses:

1. Capture the observed wrong transcript token/phrase.
2. Add that exact observed form as alias first.
3. If the miss is phrase-stable, add a phrase replacement rule.
4. Use fuzzy as fallback only when exact rules are insufficient.

For ambiguous words, avoid global single-word aliases unless global rewriting is desired.

Example:

- Prefer phrase mapping: `state changes` -> `staged changes`
- Avoid global alias unless intended everywhere: `state` -> `staged`

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
    G -- No --> FZ{fuzzy_enabled?}
    FZ -- No --> H
    FZ -- Yes --> ST{Single-word target && normalized <=4?}
    ST -- Yes --> H
    ST -- No --> I{Split-token path? 2-3 tokens to single-token target}
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

Dictionary entries are persisted independently from app settings in:

- `app_data_dir()/user_dictionary.json`

Dictionary file envelope:

- `version: u32` (current supported value: `1`)
- `entries: CustomWordEntry[]`

Load behavior:

- Missing file -> empty dictionary
- Malformed file -> empty dictionary + warning log
- Unsupported version (`version != 1`) -> empty dictionary + warning log

> [!IMPORTANT]
> Unsupported/newer versions intentionally fall back to empty in this pre-production hard-reset policy. If the user saves afterward, that file may be overwritten with version `1` data.

| Setting                           | Default | Description                                               |
| --------------------------------- | ------- | --------------------------------------------------------- |
| `word_correction_threshold`       | `0.18`  | Standard fuzzy acceptance threshold (lower = stricter)    |
| `word_correction_split_threshold` | `0.14`  | Split-token fuzzy threshold (stricter than standard path) |

`word_correction_threshold` remains at `0.18` in this rollout.

## Examples

| Transcription         | Entry (input/aliases -> replacement)        | `is_replacement` | `fuzzy_enabled` | Match Type                | Result               |
| --------------------- | ------------------------------------------- | ---------------- | --------------- | ------------------------- | -------------------- |
| `"chat gpt"`          | `ChatGPT`, aliases: `chat gpt` -> `ChatGPT` | `false`          | `false`         | Exact alias               | `"ChatGPT"`          |
| `"shad c n?"`         | `shadcn`, aliases: `shad c n` -> `shadcn`   | `false`          | `false`         | Exact alias + punctuation | `"shadcn?"`          |
| `"Shat CN component"` | `shadcn` -> `shadcn`                        | `false`          | `true`          | Split-token fuzzy         | `"Shadcn component"` |
| `"Anthrapik"`         | `Anthropic` -> `Anthropic`                  | `false`          | `true`          | Standard fuzzy            | `"Anthropic"`        |
| `"btw"`               | `btw` -> `by the way`                       | `true`           | `false`         | Exact replacement         | `"by the way"`       |

## Migration Policy

This implementation intentionally does not migrate or import dictionary data from legacy `settings.dictionary`.

- Existing `settings.dictionary` bytes may remain on disk in `settings_store.json` until settings are rewritten/reset.
- Runtime dictionary reads come only from `user_dictionary.json` and in-memory dictionary state.

## Debug Logging

Enable DEBUG log level to see detailed matching decisions.

```bash
grep -E "\[CustomWords\]" $(ls -t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)
```

Reason-coded decisions include:

- `exact_alias_match`
- `exact_canonical_match`
- `skip_fuzzy_disabled`
- `skip_short_target`
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
- **Dictionary persistence/runtime state**: `src-tauri/src/user_dictionary.rs`
- **Dictionary UI**: `src/components/dictionary/DictionaryEntryModal.tsx`
- **Dictionary list/search**: `src/components/dictionary/DictionaryPage.tsx`

## Known Limitations

| Limitation      | Impact                                                                                   |
| --------------- | ---------------------------------------------------------------------------------------- |
| N-gram size     | Exact matching scans up to 8 words; fuzzy matching scans up to 3 words                   |
| Rescoring       | N-best/lattice rescoring deferred until engines expose alternatives consistently         |
| False positives | Controlled by guard words, minimum length, length ratio, and split-threshold constraints |

## UI Features

| Feature                | Description                                                                           |
| ---------------------- | ------------------------------------------------------------------------------------- |
| Intent-first modal     | Choose `Recognize this term` or `Replace spoken phrase` before configuring details   |
| Alias editing          | Add multiple spoken variants per canonical term                                       |
| Conditional fuzzy row  | Fuzzy toggle appears only for recognize intent when input is fuzzy-eligible          |
| Duplicate detection    | Prevents collisions across canonical inputs and aliases                               |
| Search coverage        | Search includes input, replacement, and aliases                                       |
| Character limits       | Input: 100 chars, Replacement: 300 chars, Alias input: 100 chars each, max 8 aliases |
| Auto-grow textarea     | Default inputs expand up to 3 lines; replacement mapping inputs expand up to 4 lines  |
