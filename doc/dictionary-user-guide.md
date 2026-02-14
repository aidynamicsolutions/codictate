# Dictionary User Guide

This guide shows how to use the Dictionary UI to improve dictation quality in everyday use.

## What the Dictionary Does

Use Dictionary entries to teach Codictate:

- how to spell names, brands, and jargon you say often
- how to expand shortcuts like `btw` into full text
- how to map alternate pronunciations to the same final word

Think of it as "teach once, reuse everywhere."

## Where to Find It

1. Open Codictate.
2. Go to **Dictionary** in the sidebar.
3. Click **Add new**.

## Add Your First Entry

You will see:

- **Word to recognize** (or **When I say...** in replacement mode)
- **Aliases (optional)**
- **Replace with different text** toggle

Alias input tip:

- Type an alias, then press **Enter** or **comma** to add it as a chip.
- You can add up to **8 aliases** per entry.

Case behavior:

- Dictionary matching is **case-insensitive** by default.
- Output capitalization follows your replacement text and context.

### Option A: Vocabulary entry (recommended default)

Use this when you want Codictate to output a specific term.

Example:

- Word to recognize: `shadcn`
- Aliases:
  - `shad cn`
  - `shad c n`
- Leave **Replace with different text** OFF

Result: if dictation hears `shad cn`, Codictate outputs `shadcn`.

### Option B: Replacement entry

Use this when you want spoken text to expand into something else.

Example:

- Turn **Replace with different text** ON
- When I say: `btw`
- Replace with: `by the way`

Result: saying `btw` inserts `by the way`.

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
- Avoid adding very common words as aliases.

## Practical Setup Recipes

### Recipe 1: Tech term (`Shadcn`)

- Word to recognize: `shadcn`
- Aliases: `shad cn`, `shad c n`
- Replacement toggle: OFF

### Recipe 2: Product name (`ChatGPT`)

- Word to recognize: `ChatGPT`
- Aliases: `chat gpt`
- Replacement toggle: OFF

### Recipe 3: Personal shortcut (`my email`)

- Replacement toggle: ON
- When I say: `my email`
- Replace with: `john@example.com`

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
2. Add 1-2 aliases that match what transcript output shows.
3. Keep the canonical word exactly how you want it to appear.
4. For abbreviation expansion, make sure replacement mode is ON.
5. Retest with a short phrase first (for example: "use shad cn").

## Best Results in Daily Use

- Start with 5-10 high-impact entries you use every day.
- Add new entries immediately after a misshearing.
- Keep entries clean and specific; remove old ones you no longer use.
- Review Dictionary weekly and refine aliases based on real dictation.
