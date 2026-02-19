# Transcription Cleanup Filters

Codictate includes two toggleable filters that automatically clean up ASR (Automatic Speech Recognition) output. Both are **enabled by default** and can be independently toggled in **Settings → General**.

## Remove Filler Words

Removes common non-verbal hesitation markers from transcriptions. These are sounds that are never part of meaningful speech, making them safe to remove with pattern matching.

**Words removed:** um, uh, uhm, umm, uhh, uhhh, ah, eh, hmm, hm, mmm, mm, mh, ha, ehh

| Before | After |
|--------|-------|
| "So **um** I was thinking **uh** about that" | "So I was thinking about that" |
| "**Hmm** let me check **ah** that file" | "Let me check that file" |

> [!NOTE]
> Contextual filler words like "like", "you know", "basically", "actually", "so", etc. are **not** removed by this filter because they are often legitimate speech (e.g., "I like pizza"). These are better handled by AI **post-processing** (Refine), which can use sentence context to distinguish filler usage from meaningful usage.

## Remove Repeated Words

Collapses repeated words caused by speech recognition hallucinations. Handles two sub-patterns:

### Identical word repetition

When a word appears **3+ times consecutively**, it is collapsed. If the next word starts with the repeated word (a "completed form"), all copies of the partial word are dropped entirely.

| Before | After | Rule applied |
|--------|-------|-------------|
| "I want to **cont cont cont cont** continue" | "I want to continue" | Prefix match: "cont" → "continue", all copies dropped |
| "**sim sim sim sim sim** similar to how" | "similar to how" | Prefix match: "sim" → "similar", all copies dropped |
| "**wh wh wh wh** what happened" | "what happened" | Prefix match: "wh" → "what", all copies dropped |
| "call **call call call call** it dictionary" | "call it dictionary" | No prefix match ("it" ≠ "call…"), one copy kept |
| "one **three three three three three**" | "one three" | No next word, one copy kept |

### Threshold

Only **3 or more** consecutive identical words trigger the collapse. Two consecutive words (e.g., "the the") are left unchanged, as this can be intentional.

### Progressive self-corrections

Detects when a speaker makes short attempts before landing on the intended word. 

**Logic:**
- Iterates through words.
- For each "target" word (≥ 3 chars), looks back up to **3 words**.
- If a previous word is a short prefix (≤ 2 chars) of the target, and is **not** a protected word (like "go", "up", "a", "is"), it is removed.

This handles cases like:
- **Immediate prefix:** "n new" → "new"
- **Distant prefix:** "sh is still showing" → "is still showing" (where "sh" matches "showing")

**Protected Words:**
Common short words are preserved to prevent accidental removal of valid speech.
- **Preserved:** "I go going home" ("go" is preserved)
- **Preserved:** "I am a apple" ("a" is preserved)

| Before | After |
|--------|-------|
| "the correction is **f fu** fuzzy matching" | "the correction is fuzzy matching" |
| "Would this **n** new phonetic..." | "Would this new phonetic..." |
| "As in **f** from the sentence meaning..." | "As in from the sentence meaning..." |
| "...tile **sh** is still showing..." | "...tile is still showing..." |

> [!NOTE]
> We intentionally use two different guard lists in code:
> - **Fuzzy Guard Words** for custom-dictionary fuzzy matching safety, expanded from frequency-ranked common English words.
> - **Self-Correction Protected Short Words** for preserving valid 1-2 character words during cleanup.
> These lists are separate because they solve different error modes.

> [!IMPORTANT]
> **Filter Order:** When both filters are enabled, **Remove Filler Words** runs first. This ensures that filler words don't interrupt stutter patterns, allowing the **Remove Repeated Words** filter to catch stutters like "I uh I uh I want" (which becomes "I I I want" → "I want").

## Filler Word Counting

The function `filter_and_count_filler_words()` counts and removes all filler words in a single pass, returning `(filtered_text, count)`. The count is accumulated by iterating through each filler pattern sequentially: counting matches first, then removing them before moving to the next pattern.

> [!NOTE]
> **Counting edge case:** Because patterns are applied sequentially on a progressively mutated string, removing one filler word could theoretically create a new match for a subsequent pattern (e.g., if removing `"uh"` from between characters created a new word boundary match). In practice, word-boundary anchored patterns (`\buh\b`) make this extremely unlikely—removing a standalone filler word leaves spaces, which cannot form new word-boundary matches for other filler words. No action is needed, but this is documented for future maintainers.

## Known Limitations

- **Filler word list** is English-only. Non-English filler words are not removed.
- Both filters operate on the final text output; they cannot distinguish between actual speech and ASR artifacts at the audio level.
- Self-correction detection only removes 1-2 character prefixes. This intentionally preserves many valid 3-letter words (e.g., "bus", "car", "pen") but can miss longer false starts (e.g., "fuz fuzzy").
- **ASR model artifacts:** The ASR model may transcribe filler sounds as real words (e.g., "uh" → "in R"). These are not catchable by regex-based filler filters. Use **Refine** (AI post-processing) to correct these artifacts via the homophone correction prompt.

## Settings

| Setting | Key | Default |
|---------|-----|---------|
| Remove Filler Words | `enable_filler_word_filter` | `true` |
| Remove Repeated Words | `enable_hallucination_filter` | `true` |

Both are persisted in the app settings store and take effect immediately on the next transcription.

## Related Docs

- `doc/test/smart-insertion-language-profiles-manual-test-checklist.md` for manual verification cases.
- `doc/smart-insertion-notes.md` for smart-insertion implementation notes, validation history, and in-flight change tracking.
