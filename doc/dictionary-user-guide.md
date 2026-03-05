# Dictionary User Guide

This guide shows how to use the Dictionary UI to improve dictation quality in everyday use.

## What the Dictionary Does

Dictionary improves final transcript text after speech recognition.

Use Dictionary entries to:

- correct repeated transcript mistakes you see often
- map spoken variants to the output you want
- expand shortcuts like `btw` into full text

Think of it as transcript correction and normalization, not guaranteed decoder control.

## What Dictionary Does Not Guarantee

Dictionary does not force the ASR model to hear a word a specific way.

This means:

- adding a word does not guarantee every pronunciation variant maps to that word
- ambiguous single-word aliases can rewrite normal language if used globally
- fuzzy matching is intentionally conservative to reduce false positives

## Where to Find It

1. Open Codictate.
2. Go to **Dictionary** in the sidebar.
3. Click **Add new**.

## Add Your First Entry

You will see:

- **Entry intent**:
  - **Recognize this term** (default)
  - **Replace spoken phrase**
- **Word or phrase** (or **What you say** in replace intent)
- **Aliases (optional)** (always visible)
- **Enable fuzzy fallback** (shows only for eligible recognize terms)

Alias input tip:

- Type an alias, then press **Enter** or **comma** to add it as a chip.
- You can add up to **8 aliases** per entry.

Case behavior:

- Dictionary matching is **case-insensitive** by default.
- Output capitalization follows your replacement text and context.

## Why It Feels Simpler Now

The modal starts with one clear intent decision:

1. **Recognize this term**:
Use exact canonical + aliases for reliable matching.
2. **Replace spoken phrase**:
Use this when spoken words should output different text.

`Aliases` are now always visible so users can improve matching immediately.

`Enable fuzzy fallback` only appears for eligible recognize terms, so fuzzy remains opt-in and low-risk.

Today, Dictionary handles both recognition entries and spoken-phrase replacements.

## Matching Modes (Recommended)

Codictate uses a precision-first dictionary policy:

- **Default**: exact canonical + exact aliases
- **Optional**: fuzzy matching only when you explicitly enable it per vocabulary entry
- **Safety guard**: short single-word targets (canonical or alias, normalized character length `<= 4`) never use fuzzy matching, even if fuzzy is enabled

Recommended best practice:

1. Use exact canonical + aliases first.
2. Only enable fuzzy for hard, uncommon proper nouns (usually longer terms).
3. Do not rely on fuzzy for short/common words.

## Ambiguous Word Safety

For ambiguous common words (for example `state`, `mode`, `model`), avoid global single-word aliases unless you truly want global behavior.

Prefer:

1. phrase replacements for stable contexts
2. aliases based on real transcript output in that context
3. fuzzy only when exact paths fail for uncommon terms

Example:

- Better for git workflow: `state changes` -> `staged changes`
- Risky as global alias: `state` -> `staged`

## Multi-Word Best Practice

For multi-word terms and phrases, use this order:

1. Add the canonical phrase exactly as you want output.
2. Add 1-3 aliases from real transcript misses.
3. Keep fuzzy OFF first.
4. Only turn fuzzy ON if aliases still miss uncommon names/terms.

Good multi-word examples:

- Canonical: `ChatGPT` | Aliases: `chat gpt`, `chat g p t` | Fuzzy: OFF
- Canonical: `Qwen Engine` | Alias: `qwen engine` | Fuzzy: OFF
- Canonical: `Anthropic SDK` | Alias: `anthropic s d k` | Fuzzy: OFF, then ON only if needed

Rule of thumb:

- Multi-word everyday language -> exact + aliases
- Multi-word uncommon proper nouns -> exact + aliases first, fuzzy only if still missing

This avoids false positives such as common words being rewritten to short custom terms.

### Option A: Vocabulary entry (recommended default)

Use this when you want Codictate to output a specific term.

Example:

- Word to recognize: `shadcn`
- Aliases:
  - `shad cn`
  - `shad c n`
- Keep intent as **Recognize this term**
- Keep **Enable fuzzy fallback** OFF unless needed

Result: if dictation hears `shad cn`, Codictate outputs `shadcn`.

### Option B: Replacement entry

Use this when you want spoken text to expand into something else.

Example:

- Choose intent: **Replace spoken phrase**
- What you say: `btw`
- Output text: `by the way`

Result: saying `btw` inserts `by the way`.

> [!TIP]
> If Output text is different from the input, that entry is exact-only by design.

## How to Use Aliases Well

Aliases are extra ways you might say the same term.

Good alias examples for `shadcn`:

- `shad cn`
- `shad c n`

Good alias examples for `ChatGPT`:

- `chat gpt`
- `chat g p t`

Tips:

- Add aliases based on what you actually see in your transcripts.
- Keep aliases short and intentional.
- Avoid adding very common words as global aliases.

## Fix Mispronunciations Effectively

When a word is misheard, use transcript-first correction:

1. Dictate normally and check the wrong output in Home or History.
2. Add that observed output as an alias on the target term.
3. Retest with a short phrase.
4. If misses continue, add a phrase replacement for the full phrase.

Example:

- Target term: `staged`
- Observed miss (in git context): `state`
- Phrase replacement:
  - What you say: `state changes`
  - Output text: `staged changes`

If the same spoken form appears in multiple meanings, prefer phrase replacement over global single-word aliasing.

## Practical Setup Recipes

### Recipe 1: Tech term (`Shadcn`)

- Word to recognize: `shadcn`
- Aliases: `shad cn`, `shad c n`
- Output text: not set

### Recipe 2: Product name (`ChatGPT`)

- Word to recognize: `ChatGPT`
- Aliases: `chat gpt`
- Output text: not set

### Recipe 3: Personal shortcut (`my email`)

- Word or phrase: `my email`
- Output text: `john@example.com`

## Editing, Searching, and Deleting

- Use the **pencil icon** to edit an entry.
- Use the **trash icon** to delete an entry.
- Use the **Search** box to find entries by input, alias, or replacement text.

## Quick Alias from History

When Codictate captures a near-miss spelling in **Home** or **History**, you may see a sparkle action on that row.

- Hover the row actions and click the **sparkle icon**.
- Codictate will add the suggested spoken form as an alias to the matching dictionary entry.
- Use this after real misses to improve future transcriptions quickly.

## Troubleshooting Checklist

If an entry is not triggering reliably:

1. Confirm the entry exists in Dictionary.
2. Check whether transcript output is exact, close, or very different.
3. Add aliases from what transcript actually produced.
4. For repeated phrase-level misses, add a phrase replacement entry.
5. Enable fuzzy only for longer/harder proper nouns after exact+aliases are in place.
6. Retest with a short phrase first (for example: "use shad cn").

## Best Results in Daily Use

- Start with 5-10 high-impact entries you use every day.
- Add new entries immediately after a misshearing.
- Keep entries clean and specific; remove old ones you no longer use.
- Review Dictionary weekly and refine aliases based on real dictation.
