# Change: Intent-First Dictionary Entry Flow

## Why
The current Dictionary modal uses action chips and mixed terminology (`Output different text`, `Advanced`) that make the first-time mental model unclear for non-technical users. Users understand intent faster when they choose between recognition and replacement up front.

This change introduces a clearer intent-first flow without changing backend matching policy.

## What Changes
- Replace the action-chip entry flow with an intent selector:
  - `Recognize this term`
  - `Replace spoken phrase`
- Keep aliases visible by default in both intents.
- Show fuzzy matching as a single conditional row only for recognize intent when input is fuzzy-eligible.
- Rename Dictionary replacement wording to `Replace spoken phrase` and keep `Snippets` out of scope for this rollout.
- Add replace-intent validation requiring output text to be non-empty and different from input.
- Update Dictionary page copy, help tips, and docs to match intent-first terminology.

## Impact
- **Affected specs**:
  - `custom-word-correction`
- **Affected code**:
  - `src/components/dictionary/DictionaryEntryModal.tsx`
  - `src/components/dictionary/DictionaryPage.tsx`
  - `src/utils/dictionaryUtils.ts`
  - `src/utils/dictionaryUtils.test.ts`
  - `src/i18n/locales/en/translation.json`
  - `doc/dictionary-user-guide.md`
  - `doc/custom-word-correction.md`
- **Behavioral impact**:
  - Entry setup is clearer and more predictable.
  - Aliases become more discoverable.
  - Fuzzy remains constrained by existing precision guards.
  - No schema or backend algorithm changes.
